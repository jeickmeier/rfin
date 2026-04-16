//! Exponential GARCH(1,1) model (Nelson, 1991).
//!
//! Conditional variance equation (log-variance):
//!   ln(sigma^2_t) = omega + alpha * (|z_{t-1}| - E|z|) + gamma * z_{t-1} + beta * ln(sigma^2_{t-1})
//!
//! where z_t = epsilon_t / sigma_t is the standardized residual.
//!
//! Key properties:
//! - Models log-variance, so sigma^2_t > 0 is guaranteed without parameter constraints.
//! - gamma captures the leverage effect: gamma < 0 means negative returns increase volatility.
//! - No positivity constraints on omega, alpha, gamma needed.
//! - Stationarity requires |beta| < 1.
//!
//! # References
//!
//! - Nelson, D. B. (1991). "Conditional Heteroskedasticity in Asset Returns:
//!   A New Approach." Econometrica, 59(2), 347-370.

use super::forecast::VarianceForecast;
use super::garch::{fit_garch_mle, FitConfig, GarchFit, GarchModel, GarchParams};
use super::innovations::InnovationDist;

/// Exponential GARCH(1,1) model.
pub struct Egarch11;

impl GarchModel for Egarch11 {
    fn name(&self) -> &'static str {
        "EGARCH(1,1)"
    }

    fn num_params(&self) -> usize {
        4
    }

    fn param_names(&self) -> Vec<&'static str> {
        vec!["omega", "alpha", "beta", "gamma"]
    }

    fn filter(&self, returns: &[f64], params: &GarchParams, sigma2_out: &mut [f64]) {
        if returns.is_empty() {
            return;
        }

        let gamma = params.gamma.unwrap_or(0.0);
        let e_abs_z = params.dist.expected_abs();

        // Initialize from unconditional log-variance: omega / (1 - beta)
        let beta_abs = params.beta.abs();
        let ln_sigma2_0 = if beta_abs < 1.0 {
            params.omega / (1.0 - params.beta)
        } else {
            // Fallback: use log of sample variance
            let n = returns.len() as f64;
            let sv = returns.iter().map(|r| r * r).sum::<f64>() / n;
            sv.max(1e-20).ln()
        };

        let mut ln_sigma2 = ln_sigma2_0;

        for t in 0..returns.len() {
            if t > 0 {
                let sigma_prev = sigma2_out[t - 1].max(1e-20).sqrt();
                let z = returns[t - 1] / sigma_prev;
                ln_sigma2 = params.omega
                    + params.alpha * (z.abs() - e_abs_z)
                    + gamma * z
                    + params.beta * sigma2_out[t - 1].max(1e-20).ln();
            }
            // Clamp log-variance to avoid overflow/underflow
            ln_sigma2 = ln_sigma2.clamp(-50.0, 50.0);
            sigma2_out[t] = ln_sigma2.exp();
        }
    }

    fn log_likelihood(
        &self,
        returns: &[f64],
        params: &GarchParams,
        dist: InnovationDist,
    ) -> f64 {
        let n = returns.len();
        if n == 0 {
            return f64::NEG_INFINITY;
        }

        let mut sigma2 = vec![0.0; n];
        self.filter(returns, params, &mut sigma2);

        let mut ll = 0.0;
        for t in 0..n {
            let s2 = sigma2[t];
            if s2 <= 0.0 || !s2.is_finite() {
                return f64::NEG_INFINITY;
            }
            let z = returns[t] / s2.sqrt();
            ll += -0.5 * s2.ln() + dist.log_pdf(z);
        }

        if ll.is_finite() {
            ll
        } else {
            f64::NEG_INFINITY
        }
    }

    fn fit(
        &self,
        returns: &[f64],
        dist: InnovationDist,
        config: Option<&FitConfig>,
    ) -> crate::Result<GarchFit> {
        let default_config = FitConfig::default();
        let config = config.unwrap_or(&default_config);

        let sample_var = {
            let n = returns.len() as f64;
            let mean = returns.iter().sum::<f64>() / n;
            returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / (n - 1.0)
        };

        // EGARCH operates in log-variance space, so omega bounds are different
        let mut bounds = vec![
            (-5.0, 5.0),     // omega (log-variance intercept)
            (0.0, 1.0),      // alpha (magnitude effect)
            (-0.9999, 0.9999), // beta (persistence in log-variance)
            (-0.50, 0.50),   // gamma (leverage)
        ];
        if let InnovationDist::StudentT(_) = dist {
            bounds.push((InnovationDist::dof_lower_bound(), 100.0));
        }

        let stationarity_check = |x: &[f64]| -> bool {
            let beta = x[2];
            beta.abs() < 0.9999
        };

        // Override the grid search with EGARCH-specific initial values
        let _ = sample_var;
        fit_garch_mle(self, returns, dist, config, true, &bounds, stationarity_check)
    }

    fn forecast(
        &self,
        fit: &GarchFit,
        horizons: &[usize],
        trading_days_per_year: f64,
    ) -> Vec<VarianceForecast> {
        let p = &fit.params;
        let beta = p.beta;

        // For EGARCH, the unconditional log-variance is omega / (1 - beta)
        let ln_sigma2_unc = if beta.abs() < 1.0 {
            p.omega / (1.0 - beta)
        } else {
            fit.terminal_variance.max(1e-20).ln()
        };

        let ln_sigma2_t = fit.terminal_variance.max(1e-20).ln();

        horizons
            .iter()
            .map(|&h| {
                let ln_sigma2_h = if h == 0 {
                    ln_sigma2_t
                } else {
                    // Under E[z] = 0 forecast: ln(sigma2_{t+h}) converges to unconditional
                    ln_sigma2_unc
                        + beta.powi(h as i32) * (ln_sigma2_t - ln_sigma2_unc)
                };
                let sigma2_h = ln_sigma2_h.exp();
                VarianceForecast {
                    horizon: h,
                    variance: sigma2_h,
                    annualized_vol: (sigma2_h * trading_days_per_year).sqrt(),
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_produces_positive_variances() {
        let returns = [0.01, -0.02, 0.015, -0.01, 0.005, -0.03, 0.01, 0.02];
        let params = GarchParams {
            omega: -0.1,
            alpha: 0.15,
            beta: 0.95,
            gamma: Some(-0.05),
            dist: InnovationDist::Gaussian,
        };

        let mut sigma2 = vec![0.0; returns.len()];
        Egarch11.filter(&returns, &params, &mut sigma2);

        // EGARCH models log-variance, so output is always positive
        for &s in &sigma2 {
            assert!(s > 0.0, "EGARCH variance must be positive, got {}", s);
        }
    }

    #[test]
    fn leverage_effect() {
        // After a negative shock, EGARCH with gamma < 0 should produce higher variance
        let params = GarchParams {
            omega: -0.1,
            alpha: 0.15,
            beta: 0.90,
            gamma: Some(-0.10),
            dist: InnovationDist::Gaussian,
        };

        // Positive shock series
        let returns_pos = [0.02, 0.02, 0.02, 0.02];
        let mut sigma2_pos = vec![0.0; 4];
        Egarch11.filter(&returns_pos, &params, &mut sigma2_pos);

        // Negative shock series (same magnitude)
        let returns_neg = [-0.02, -0.02, -0.02, -0.02];
        let mut sigma2_neg = vec![0.0; 4];
        Egarch11.filter(&returns_neg, &params, &mut sigma2_neg);

        // With gamma < 0, negative returns should produce higher subsequent variance
        // Compare the last variance (after the shocks have propagated)
        assert!(
            sigma2_neg[3] > sigma2_pos[3],
            "EGARCH with gamma<0: negative shocks ({}) should produce higher variance than positive shocks ({})",
            sigma2_neg[3], sigma2_pos[3]
        );
    }
}
