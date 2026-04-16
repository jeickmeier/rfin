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
use super::garch::{fit_garch_mle, FitConfig, GarchFit, GarchModel, GarchParams};
use super::innovations::InnovationDist;

/// Standard GARCH(1,1) model.
pub struct Garch11;

impl GarchModel for Garch11 {
    fn name(&self) -> &'static str {
        "GARCH(1,1)"
    }

    fn num_params(&self) -> usize {
        3
    }

    fn param_names(&self) -> Vec<&'static str> {
        vec!["omega", "alpha", "beta"]
    }

    fn filter(&self, returns: &[f64], params: &GarchParams, sigma2_out: &mut [f64]) {
        if returns.is_empty() {
            return;
        }

        let sigma2_0 = params.unconditional_variance().unwrap_or_else(|| {
            let n = returns.len() as f64;
            returns.iter().map(|r| r * r).sum::<f64>() / n
        });

        // First observation uses sigma2_0 as the "previous" variance
        // and the first return as the "previous" shock (or we use sigma2_0
        // as the first conditional variance).
        sigma2_out[0] = params.omega + params.alpha * returns[0].powi(2) + params.beta * sigma2_0;
        // Ensure positivity
        sigma2_out[0] = sigma2_out[0].max(1e-20);

        for t in 1..returns.len() {
            sigma2_out[t] = params.omega
                + params.alpha * returns[t - 1].powi(2)
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
        ];
        if let InnovationDist::StudentT(_) = dist {
            bounds.push((InnovationDist::dof_lower_bound(), 100.0));
        }

        let stationarity_check = |x: &[f64]| -> bool {
            let alpha = x[1];
            let beta = x[2];
            alpha + beta < 0.9999 && x[0] > 0.0
        };

        fit_garch_mle(self, returns, dist, config, false, &bounds, stationarity_check)
    }

    fn forecast(
        &self,
        fit: &GarchFit,
        horizons: &[usize],
        trading_days_per_year: f64,
    ) -> Vec<VarianceForecast> {
        let p = &fit.params;
        let persistence = p.alpha + p.beta;
        let sigma2_unc = p.unconditional_variance().unwrap_or(fit.terminal_variance);
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
    fn filter_known_data() {
        // Hand-verify the GARCH(1,1) filter on a short series
        let returns = [0.01, -0.02, 0.015, -0.01, 0.005];
        let params = GarchParams {
            omega: 0.00001,
            alpha: 0.1,
            beta: 0.85,
            gamma: None,
            dist: InnovationDist::Gaussian,
        };

        let mut sigma2 = vec![0.0; 5];
        Garch11.filter(&returns, &params, &mut sigma2);

        // Check unconditional variance
        let uncond = params.omega / (1.0 - params.alpha - params.beta);

        // First: sigma2[0] = omega + alpha * r[0]^2 + beta * uncond
        let expected_0 = params.omega + params.alpha * 0.01_f64.powi(2) + params.beta * uncond;
        assert!(
            (sigma2[0] - expected_0).abs() < 1e-12,
            "sigma2[0]={}, expected={}",
            sigma2[0],
            expected_0
        );

        // Second: sigma2[1] = omega + alpha * r[0]^2 + beta * sigma2[0]
        let expected_1 =
            params.omega + params.alpha * returns[0].powi(2) + params.beta * sigma2[0];
        assert!(
            (sigma2[1] - expected_1).abs() < 1e-12,
            "sigma2[1]={}, expected={}",
            sigma2[1],
            expected_1
        );

        // All variances should be positive
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

        let forecasts = Garch11.forecast(&fit, &[1, 5, 21, 63, 252, 1000], 252.0);
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
}
