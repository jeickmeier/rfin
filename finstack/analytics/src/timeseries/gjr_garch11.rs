//! GJR-GARCH(1,1) model (Glosten, Jagannathan & Runkle, 1993).
//!
//! Conditional variance equation:
//!   sigma^2_t = omega + (alpha + gamma * I_{epsilon<0}) * epsilon^2_{t-1} + beta * sigma^2_{t-1}
//!
//! where I_{epsilon<0} is the indicator function for negative returns.
//!
//! Key properties:
//! - gamma > 0 captures the leverage effect (negative returns have larger impact).
//! - Nests GARCH(1,1) when gamma = 0.
//! - Stationarity requires alpha + beta + gamma/2 < 1.
//!
//! # References
//!
//! - Glosten, L. R., Jagannathan, R., & Runkle, D. E. (1993). "On the Relation
//!   between the Expected Value and the Volatility of the Nominal Excess Return
//!   on Stocks." Journal of Finance, 48(5), 1779-1801.

use super::forecast::VarianceForecast;
use super::garch::{fit_garch_mle, FitConfig, GarchFit, GarchModel, GarchParams};
use super::innovations::InnovationDist;

/// GJR-GARCH(1,1) model.
pub struct GjrGarch11;

impl GarchModel for GjrGarch11 {
    fn name(&self) -> &'static str {
        "GJR-GARCH(1,1)"
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

        let sigma2_0 = params.unconditional_variance().unwrap_or_else(|| {
            let n = returns.len() as f64;
            returns.iter().map(|r| r * r).sum::<f64>() / n
        });

        // First observation
        let e2 = returns[0].powi(2);
        let indicator = if returns[0] < 0.0 { 1.0 } else { 0.0 };
        sigma2_out[0] =
            params.omega + (params.alpha + gamma * indicator) * e2 + params.beta * sigma2_0;
        sigma2_out[0] = sigma2_out[0].max(1e-20);

        for t in 1..returns.len() {
            let e2 = returns[t - 1].powi(2);
            let indicator = if returns[t - 1] < 0.0 { 1.0 } else { 0.0 };
            sigma2_out[t] = params.omega
                + (params.alpha + gamma * indicator) * e2
                + params.beta * sigma2_out[t - 1];
            sigma2_out[t] = sigma2_out[t].max(1e-20);
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

        let mut bounds = vec![
            (1e-10, 10.0 * sample_var), // omega
            (1e-6, 0.50),               // alpha
            (1e-6, 0.9999),             // beta
            (0.0, 0.50),                // gamma (GJR: non-negative leverage)
        ];
        if let InnovationDist::StudentT(_) = dist {
            bounds.push((InnovationDist::dof_lower_bound(), 100.0));
        }

        let stationarity_check = |x: &[f64]| -> bool {
            let alpha = x[1];
            let beta = x[2];
            let gamma = x[3];
            // GJR stationarity: alpha + beta + gamma/2 < 1
            alpha + beta + gamma / 2.0 < 0.9999 && x[0] > 0.0
        };

        fit_garch_mle(self, returns, dist, config, true, &bounds, stationarity_check)
    }

    fn forecast(
        &self,
        fit: &GarchFit,
        horizons: &[usize],
        trading_days_per_year: f64,
    ) -> Vec<VarianceForecast> {
        let p = &fit.params;
        let gamma = p.gamma.unwrap_or(0.0);
        // GJR effective persistence: alpha + beta + gamma/2
        let persistence = p.alpha + p.beta + gamma / 2.0;

        // Unconditional variance for GJR: omega / (1 - alpha - beta - gamma/2)
        let sigma2_unc = if persistence < 1.0 && persistence > 0.0 {
            p.omega / (1.0 - persistence)
        } else {
            fit.terminal_variance
        };

        let sigma2_t = fit.terminal_variance;

        horizons
            .iter()
            .map(|&h| {
                let sigma2_h = if h == 0 {
                    sigma2_t
                } else {
                    sigma2_unc + persistence.powi(h as i32) * (sigma2_t - sigma2_unc)
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
    fn filter_leverage_effect() {
        let params = GarchParams {
            omega: 0.00001,
            alpha: 0.05,
            beta: 0.85,
            gamma: Some(0.10),
            dist: InnovationDist::Gaussian,
        };

        // Compare effect of positive vs negative return
        let returns_pos = [0.02, 0.0, 0.0, 0.0];
        let returns_neg = [-0.02, 0.0, 0.0, 0.0];

        let mut sigma2_pos = vec![0.0; 4];
        let mut sigma2_neg = vec![0.0; 4];

        GjrGarch11.filter(&returns_pos, &params, &mut sigma2_pos);
        GjrGarch11.filter(&returns_neg, &params, &mut sigma2_neg);

        // With gamma > 0, negative returns should produce higher conditional variance
        assert!(
            sigma2_neg[1] > sigma2_pos[1],
            "GJR with gamma>0: negative shock ({}) should produce higher variance than positive ({})",
            sigma2_neg[1], sigma2_pos[1]
        );
    }

    #[test]
    fn nests_garch_when_gamma_zero() {
        let params_gjr = GarchParams {
            omega: 0.00001,
            alpha: 0.10,
            beta: 0.85,
            gamma: Some(0.0),
            dist: InnovationDist::Gaussian,
        };
        let params_garch = GarchParams {
            omega: 0.00001,
            alpha: 0.10,
            beta: 0.85,
            gamma: None,
            dist: InnovationDist::Gaussian,
        };

        let returns = [0.01, -0.02, 0.015, -0.01, 0.005];
        let mut sigma2_gjr = vec![0.0; 5];
        let mut sigma2_garch = vec![0.0; 5];

        GjrGarch11.filter(&returns, &params_gjr, &mut sigma2_gjr);
        super::super::garch11::Garch11.filter(&returns, &params_garch, &mut sigma2_garch);

        for t in 0..5 {
            assert!(
                (sigma2_gjr[t] - sigma2_garch[t]).abs() < 1e-15,
                "GJR with gamma=0 should match GARCH at t={}: {} vs {}",
                t,
                sigma2_gjr[t],
                sigma2_garch[t]
            );
        }
    }

    #[test]
    fn stationarity_condition() {
        let params = GarchParams {
            omega: 0.00001,
            alpha: 0.05,
            beta: 0.85,
            gamma: Some(0.10),
            dist: InnovationDist::Gaussian,
        };
        // GJR stationarity: alpha + beta + gamma/2 = 0.05 + 0.85 + 0.05 = 0.95 < 1
        let persistence = params.alpha + params.beta + params.gamma.unwrap() / 2.0;
        assert!(persistence < 1.0);
    }
}
