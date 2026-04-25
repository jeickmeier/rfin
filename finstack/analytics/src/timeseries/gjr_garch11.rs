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
//! - Glosten, Jagannathan & Runkle (1993): see docs/REFERENCES.md#glosten1993

use super::forecast::VarianceForecast;
use super::garch::{GarchFit, GarchModel, GarchParams};
use super::innovations::InnovationDist;

/// GJR-GARCH(1,1) model.
pub struct GjrGarch11;

impl GarchModel for GjrGarch11 {
    fn name(&self) -> &'static str {
        "GJR-GARCH(1,1)"
    }

    fn family(&self) -> super::garch::GarchFamily {
        super::garch::GarchFamily::GjrGarch11
    }

    fn num_params(&self) -> usize {
        4
    }

    fn param_names(&self) -> Vec<&'static str> {
        vec!["omega", "alpha", "beta", "gamma"]
    }

    fn has_gamma(&self) -> bool {
        true
    }

    fn parameter_bounds(&self, sample_var: f64) -> Vec<(f64, f64)> {
        vec![
            (1e-10, 10.0 * sample_var), // omega
            (1e-6, 0.50),               // alpha
            (1e-6, 0.9999),             // beta
            (0.0, 0.50),                // gamma (GJR: non-negative leverage)
        ]
    }

    fn is_stationary(&self, params: &[f64]) -> bool {
        let alpha = params[1];
        let beta = params[2];
        let gamma = params[3];
        // GJR stationarity under symmetric innovations (Gaussian and symmetric
        // Student-t satisfy E[I{z<0} z^2] = 1/2): alpha + beta + gamma/2 < 1.
        alpha + beta + gamma / 2.0 < 0.9999 && params[0] > 0.0
    }

    fn filter(&self, returns: &[f64], params: &GarchParams, sigma2_out: &mut [f64]) {
        if returns.is_empty() {
            return;
        }

        let gamma = params.gamma.unwrap_or(0.0);
        let mu = params.mean;

        let sigma2_0 = params.unconditional_variance().unwrap_or_else(|| {
            let n = returns.len() as f64;
            returns.iter().map(|r| (r - mu).powi(2)).sum::<f64>() / n
        });

        // Standard GARCH convention: sigma^2_0 = unconditional. For t >= 1
        // the GJR leverage indicator triggers on the demeaned residual.
        sigma2_out[0] = sigma2_0.max(1e-20);

        for t in 1..returns.len() {
            let eps_prev = returns[t - 1] - mu;
            let e2 = eps_prev * eps_prev;
            let indicator = if eps_prev < 0.0 { 1.0 } else { 0.0 };
            sigma2_out[t] = params.omega
                + (params.alpha + gamma * indicator) * e2
                + params.beta * sigma2_out[t - 1];
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
        let sigma2_1 = terminal_residual
            .map(|eps_t| {
                let indicator = if eps_t < 0.0 { 1.0 } else { 0.0 };
                p.omega + (p.alpha + gamma * indicator) * eps_t * eps_t + p.beta * sigma2_t
            })
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
    fn filter_leverage_effect() {
        let params = GarchParams {
            omega: 0.00001,
            alpha: 0.05,
            beta: 0.85,
            gamma: Some(0.10),
            dist: InnovationDist::Gaussian,
            family: super::super::garch::GarchFamily::GjrGarch11,
            mean: 0.0,
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
            family: super::super::garch::GarchFamily::GjrGarch11,
            mean: 0.0,
        };
        let params_garch = GarchParams {
            omega: 0.00001,
            alpha: 0.10,
            beta: 0.85,
            gamma: None,
            dist: InnovationDist::Gaussian,
            family: super::super::garch::GarchFamily::Garch11,
            mean: 0.0,
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
            family: super::super::garch::GarchFamily::GjrGarch11,
            mean: 0.0,
        };
        // GJR persistence: alpha + beta + gamma/2 = 0.05 + 0.85 + 0.05 = 0.95
        assert!((params.persistence() - 0.95).abs() < 1e-12);
        assert!(params.persistence() < 1.0);
        assert!(params.unconditional_variance().is_some());
    }

    #[test]
    fn egarch_persistence_is_beta_only() {
        // Regression: persistence() must branch on family. EGARCH persistence
        // is beta alone; applying alpha+beta would mis-state half-life and
        // (more importantly) break comparisons against GARCH/GJR.
        let params = GarchParams {
            omega: -0.1,
            alpha: 0.15,
            beta: 0.95,
            gamma: Some(-0.05),
            dist: InnovationDist::Gaussian,
            family: super::super::garch::GarchFamily::Egarch11,
            mean: 0.0,
        };
        assert!((params.persistence() - 0.95).abs() < 1e-12);
        // Simple unconditional variance is not well-defined for EGARCH; must be None.
        assert!(params.unconditional_variance().is_none());
    }

    #[test]
    fn forecast_uses_terminal_residual_sign_for_one_step() {
        let fit = GarchFit {
            model: "GJR-GARCH(1,1)".to_string(),
            params: GarchParams {
                omega: 0.00001,
                alpha: 0.05,
                beta: 0.85,
                gamma: Some(0.10),
                dist: InnovationDist::Gaussian,
                family: super::super::garch::GarchFamily::GjrGarch11,
                mean: 0.0,
            },
            std_errors: None,
            log_likelihood: -1000.0,
            n_obs: 1000,
            n_params: 4,
            aic: 2008.0,
            bic: 2026.0,
            hqic: 2012.0,
            conditional_variances: vec![0.0002; 1000],
            standardized_residuals: vec![0.0; 1000],
            terminal_variance: 0.0003,
            converged: true,
            iterations: 100,
        };

        let positive = GjrGarch11.forecast(&fit, &[1], 252.0, Some(0.02));
        let negative = GjrGarch11.forecast(&fit, &[1], 252.0, Some(-0.02));

        assert!(negative[0].variance > positive[0].variance);
        assert!((negative[0].variance - 0.000325).abs() < 1e-12);
    }
}
