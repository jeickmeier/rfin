//! Standard GARCH(1,1) model (Bollerslev, 1986).
//!
//! Conditional variance equation:
//!   sigma^2_t = omega + alpha * epsilon^2_{t-1} + beta * sigma^2_{t-1}
//!
//! Constraints:
//!   omega > 0, alpha >= 0, beta >= 0, alpha + beta < 1 (stationarity)
//!
//! # References
//!
//! - Bollerslev, T. (1986). "Generalized Autoregressive Conditional
//!   Heteroskedasticity." Journal of Econometrics, 31(3), 307-327.

use super::forecast::VarianceForecast;
use super::garch::{GarchFit, GarchModel, GarchParams};
use super::innovations::InnovationDist;

/// Standard GARCH(1,1) model.
pub struct Garch11;

impl GarchModel for Garch11 {
    fn name(&self) -> &'static str {
        "GARCH(1,1)"
    }

    fn family(&self) -> super::garch::GarchFamily {
        super::garch::GarchFamily::Garch11
    }

    fn num_params(&self) -> usize {
        3
    }

    fn param_names(&self) -> Vec<&'static str> {
        vec!["omega", "alpha", "beta"]
    }

    fn has_gamma(&self) -> bool {
        false
    }

    fn parameter_bounds(&self, sample_var: f64) -> Vec<(f64, f64)> {
        vec![
            (1e-10, 10.0 * sample_var), // omega
            (1e-6, 0.50),               // alpha
            (1e-6, 0.9999),             // beta
        ]
    }

    fn is_stationary(&self, params: &[f64]) -> bool {
        let alpha = params[1];
        let beta = params[2];
        alpha + beta < 0.9999 && params[0] > 0.0
    }

    fn filter(&self, returns: &[f64], params: &GarchParams, sigma2_out: &mut [f64]) {
        if returns.is_empty() {
            return;
        }

        let mu = params.mean;
        let sigma2_0 = params.unconditional_variance().unwrap_or_else(|| {
            let n = returns.len() as f64;
            returns.iter().map(|r| (r - mu).powi(2)).sum::<f64>() / n
        });

        // Standard GARCH convention: sigma^2_0 is the unconditional
        // variance, and sigma^2_t = omega + alpha*eps_{t-1}^2 + beta*sigma^2_{t-1}
        // for t >= 1. This keeps the filter non-anticipating (sigma^2_t
        // depends on information strictly before t) and matches the
        // EGARCH / GJR convention in this crate.
        sigma2_out[0] = sigma2_0.max(1e-20);

        for t in 1..returns.len() {
            let eps_prev = returns[t - 1] - mu;
            sigma2_out[t] =
                params.omega + params.alpha * eps_prev * eps_prev + params.beta * sigma2_out[t - 1];
            sigma2_out[t] = sigma2_out[t].max(1e-20);
        }
    }

    fn log_likelihood(&self, returns: &[f64], params: &GarchParams, dist: InnovationDist) -> f64 {
        let n = returns.len();
        if n == 0 {
            return f64::NEG_INFINITY;
        }

        let mut sigma2 = vec![0.0; n];
        self.filter(returns, params, &mut sigma2);
        let mu = params.mean;

        let mut ll = 0.0;
        for t in 0..n {
            let s2 = sigma2[t];
            if s2 <= 0.0 || !s2.is_finite() {
                return f64::NEG_INFINITY;
            }
            // Standardised residual on the demeaned innovation.
            let z = (returns[t] - mu) / s2.sqrt();
            ll += -0.5 * s2.ln() + dist.log_pdf(z);
        }

        if ll.is_finite() {
            ll
        } else {
            f64::NEG_INFINITY
        }
    }

    fn forecast(
        &self,
        fit: &GarchFit,
        horizons: &[usize],
        trading_days_per_year: f64,
        terminal_residual: Option<f64>,
    ) -> Vec<VarianceForecast> {
        let p = &fit.params;
        let persistence = p.alpha + p.beta;
        let sigma2_unc = p.unconditional_variance().unwrap_or(fit.terminal_variance);
        let sigma2_t = fit.terminal_variance;
        let sigma2_1 = terminal_residual
            .map(|eps_t| p.omega + p.alpha * eps_t * eps_t + p.beta * sigma2_t)
            .unwrap_or_else(|| sigma2_unc + persistence * (sigma2_t - sigma2_unc));

        horizons
            .iter()
            .map(|&h| {
                let sigma2_h = if h == 0 {
                    sigma2_t
                } else if h == 1 {
                    sigma2_1
                } else {
                    sigma2_unc + persistence.powi(h as i32 - 1) * (sigma2_1 - sigma2_unc)
                };
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
    fn filter_known_data() {
        // Hand-verify the GARCH(1,1) filter on a short series under the
        // standard non-anticipating convention: sigma^2_0 = unconditional,
        // sigma^2_t uses eps_{t-1} for t >= 1.
        let returns = [0.01, -0.02, 0.015, -0.01, 0.005];
        let mu = 0.0; // params.mean defaults to 0 in this hand-constructed case
        let params = GarchParams {
            omega: 0.00001,
            alpha: 0.1,
            beta: 0.85,
            gamma: None,
            dist: InnovationDist::Gaussian,
            family: super::super::garch::GarchFamily::Garch11,
            mean: mu,
        };

        let mut sigma2 = vec![0.0; 5];
        Garch11.filter(&returns, &params, &mut sigma2);

        let uncond = params.omega / (1.0 - params.alpha - params.beta);

        // sigma^2_0 = unconditional (no prior data used).
        assert!(
            (sigma2[0] - uncond).abs() < 1e-12,
            "sigma2[0]={}, expected uncond={}",
            sigma2[0],
            uncond
        );

        // sigma^2_1 = omega + alpha * (r_0 - mu)^2 + beta * sigma^2_0
        let eps_0 = returns[0] - mu;
        let expected_1 = params.omega + params.alpha * eps_0 * eps_0 + params.beta * sigma2[0];
        assert!(
            (sigma2[1] - expected_1).abs() < 1e-12,
            "sigma2[1]={}, expected={}",
            sigma2[1],
            expected_1
        );

        // All variances positive.
        for &s in &sigma2 {
            assert!(s > 0.0);
        }
    }

    #[test]
    fn stationarity_check() {
        let params = GarchParams {
            omega: 0.00001,
            alpha: 0.1,
            beta: 0.85,
            gamma: None,
            dist: InnovationDist::Gaussian,
            family: super::super::garch::GarchFamily::Garch11,
            mean: 0.0,
        };
        assert!(params.persistence() < 1.0);
        assert!(params.unconditional_variance().is_some());
        assert!(params.half_life().is_some());
    }

    #[test]
    fn non_stationary_no_unconditional() {
        let params = GarchParams {
            omega: 0.00001,
            alpha: 0.5,
            beta: 0.6,
            gamma: None,
            dist: InnovationDist::Gaussian,
            family: super::super::garch::GarchFamily::Garch11,
            mean: 0.0,
        };
        assert!(params.persistence() > 1.0);
        assert!(params.unconditional_variance().is_none());
    }

    #[test]
    fn forecast_converges_to_unconditional() {
        let fit = GarchFit {
            model: "GARCH(1,1)".to_string(),
            params: GarchParams {
                omega: 0.00001,
                alpha: 0.1,
                beta: 0.85,
                gamma: None,
                dist: InnovationDist::Gaussian,
                family: super::super::garch::GarchFamily::Garch11,
                mean: 0.0,
            },
            std_errors: None,
            log_likelihood: -1000.0,
            n_obs: 1000,
            n_params: 3,
            aic: 2006.0,
            bic: 2020.0,
            hqic: 2010.0,
            conditional_variances: vec![0.0002; 1000],
            standardized_residuals: vec![0.0; 1000],
            terminal_variance: 0.0003,
            converged: true,
            iterations: 100,
        };

        let forecasts = Garch11.forecast(&fit, &[1, 5, 21, 63, 252, 1000], 252.0, None);
        let uncond = fit.params.unconditional_variance().unwrap();

        // Check that forecasts converge to unconditional
        let last = forecasts.last().unwrap();
        assert!(
            (last.variance - uncond).abs() < 1e-6,
            "Far-horizon forecast {} should converge to unconditional {}",
            last.variance,
            uncond
        );

        // Check monotone convergence: differences from uncond should decrease
        for i in 1..forecasts.len() {
            let diff_prev = (forecasts[i - 1].variance - uncond).abs();
            let diff_curr = (forecasts[i].variance - uncond).abs();
            assert!(
                diff_curr <= diff_prev + 1e-15,
                "Forecast term structure should monotonically converge"
            );
        }
    }

    #[test]
    fn forecast_uses_terminal_residual_for_one_step() {
        let fit = GarchFit {
            model: "GARCH(1,1)".to_string(),
            params: GarchParams {
                omega: 0.00001,
                alpha: 0.1,
                beta: 0.85,
                gamma: None,
                dist: InnovationDist::Gaussian,
                family: super::super::garch::GarchFamily::Garch11,
                mean: 0.01,
            },
            std_errors: None,
            log_likelihood: -1000.0,
            n_obs: 1000,
            n_params: 3,
            aic: 2006.0,
            bic: 2020.0,
            hqic: 2010.0,
            conditional_variances: vec![0.0002; 1000],
            standardized_residuals: vec![0.0; 1000],
            terminal_variance: 0.0003,
            converged: true,
            iterations: 100,
        };

        let forecasts = Garch11.forecast(&fit, &[1], 252.0, Some(0.02));
        assert!((forecasts[0].variance - 0.000305).abs() < 1e-12);
    }
}
