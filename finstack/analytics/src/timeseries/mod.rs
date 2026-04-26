//! Time series volatility models: GARCH family with MLE fitting and forecasting.
//!
//! This module provides forward-looking volatility models that complement the
//! backward-looking realized variance estimators in `finstack-core`. The GARCH
//! family covers both symmetric and asymmetric (leverage) volatility dynamics.
//!
//! # Models
//!
//! - [`Garch11`] - Standard GARCH(1,1) (Bollerslev, 1986)
//! - [`Egarch11`] - Exponential GARCH (Nelson, 1991) with leverage via log-variance
//! - [`GjrGarch11`] - GJR-GARCH (Glosten, Jagannathan & Runkle, 1993) with asymmetric threshold
//!
//! # Quick start
//!
//! ```no_run
//! use finstack_analytics::timeseries::{
//!     auto_garch, vol_term_structure, Garch11, GarchModel, InnovationDist,
//! };
//!
//! let log_returns = vec![
//!     0.004, -0.006, 0.002, 0.009, -0.011, 0.003, -0.004, 0.006, -0.008,
//!     0.005, 0.002, -0.003, 0.007, -0.010, 0.004, 0.006, -0.002, 0.001,
//!     -0.005, 0.008,
//! ];
//!
//! // Fit GARCH(1,1) with Gaussian innovations.
//! let fit = Garch11.fit(&log_returns, InnovationDist::Gaussian, None)?;
//! println!("{}", fit.summary());
//!
//! // Forecast a volatility term structure at standard horizons.
//! let ts = vol_term_structure(&Garch11, &fit, 252.0, None);
//!
//! // Auto-select the best standard model by BIC.
//! let best = auto_garch(&log_returns, InnovationDist::Gaussian, None)?;
//! # Ok::<(), finstack_core::Error>(())
//! ```
//!
//! # References
//!
//! - Bollerslev (1986): see docs/REFERENCES.md#bollerslev1986
//! - Nelson (1991): see docs/REFERENCES.md#nelson1991
//! - Glosten, Jagannathan & Runkle (1993): see docs/REFERENCES.md#glosten1993

mod diagnostics;
mod egarch11;
mod forecast;
mod garch;
mod garch11;
mod gjr_garch11;
mod innovations;
mod optimizer;

// Curated public surface: downstream callers should use these re-exports
// rather than mixing and matching submodule paths.
pub use diagnostics::{aic, arch_lm, bic, hqic, ljung_box};
pub use egarch11::Egarch11;
pub use forecast::{
    forecast_garch_fit, vol_term_structure, VarianceForecast, VolTermStructure, STANDARD_HORIZONS,
};
pub use garch::{FitConfig, GarchFamily, GarchFit, GarchModel, GarchParams};
pub use garch11::Garch11;
pub use gjr_garch11::GjrGarch11;
pub use innovations::InnovationDist;

/// Compare multiple GARCH models on the same return series.
///
/// Fits each model (GARCH, EGARCH, GJR-GARCH) and returns results sorted
/// by BIC (lowest = best).
///
/// # Arguments
/// * `returns` - Log return series.
/// * `dist` - Innovation distribution to use for all models.
/// * `config` - Optional fitting configuration.
///
/// # Returns
/// Vector of `GarchFit` sorted by BIC ascending.
///
/// # Errors
///
/// Returns an error if every candidate model fails to fit the input series.
///
/// # Examples
///
/// ```no_run
/// use finstack_analytics::timeseries::{compare_garch_models, InnovationDist};
///
/// let returns = vec![
///     0.004, -0.006, 0.002, 0.009, -0.011, 0.003, -0.004, 0.006, -0.008,
///     0.005, 0.002, -0.003, 0.007, -0.010, 0.004, 0.006, -0.002, 0.001,
///     -0.005, 0.008,
/// ];
/// let ranked = compare_garch_models(&returns, InnovationDist::Gaussian, None)?;
/// assert!(!ranked.is_empty());
/// # Ok::<(), finstack_core::Error>(())
/// ```
pub fn compare_garch_models(
    returns: &[f64],
    dist: InnovationDist,
    config: Option<&FitConfig>,
) -> crate::Result<Vec<GarchFit>> {
    let models: [&dyn GarchModel; 3] = [&Garch11, &Egarch11, &GjrGarch11];

    let mut results: Vec<GarchFit> = Vec::new();
    for model in models {
        match model.fit(returns, dist, config) {
            Ok(fit) => results.push(fit),
            Err(_) => continue,
        }
    }

    if results.is_empty() {
        return Err(finstack_core::Error::Validation(
            "All GARCH models failed to converge".to_string(),
        ));
    }

    results.sort_by(|a, b| {
        a.bic
            .partial_cmp(&b.bic)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(results)
}

/// Fit the best GARCH model by BIC from the standard set.
///
/// Equivalent to `compare_garch_models(...)?[0]` but returns just the best fit.
///
/// # Arguments
///
/// * `returns` - Log return series.
/// * `dist` - Innovation distribution to use for all candidate models.
/// * `config` - Optional fitting configuration.
///
/// # Returns
///
/// The lowest-BIC [`GarchFit`] among the standard GARCH, EGARCH, and GJR-GARCH
/// models that successfully fit the input.
///
/// # Errors
///
/// Returns an error if all candidate models fail to fit.
///
/// # Examples
///
/// ```no_run
/// use finstack_analytics::timeseries::{auto_garch, InnovationDist};
///
/// let returns = vec![
///     0.004, -0.006, 0.002, 0.009, -0.011, 0.003, -0.004, 0.006, -0.008,
///     0.005, 0.002, -0.003, 0.007, -0.010, 0.004, 0.006, -0.002, 0.001,
///     -0.005, 0.008,
/// ];
/// let fit = auto_garch(&returns, InnovationDist::Gaussian, None)?;
/// assert!(fit.n_obs >= 10);
/// # Ok::<(), finstack_core::Error>(())
/// ```
pub fn auto_garch(
    returns: &[f64],
    dist: InnovationDist,
    config: Option<&FitConfig>,
) -> crate::Result<GarchFit> {
    let results = compare_garch_models(returns, dist, config)?;
    // results is sorted by BIC ascending, first is best
    Ok(results.into_iter().next().unwrap_or_else(|| unreachable!()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Generate a synthetic GARCH(1,1) series with known parameters.
    fn generate_garch_data(omega: f64, alpha: f64, beta: f64, n: usize, seed: u64) -> Vec<f64> {
        use rand::rngs::SmallRng;
        use rand::SeedableRng;

        let mut rng = SmallRng::seed_from_u64(seed);
        let uncond_var = omega / (1.0 - alpha - beta);
        let mut sigma2 = uncond_var;
        let mut returns = Vec::with_capacity(n);

        for _ in 0..n {
            // Box-Muller for deterministic normal draws
            let z = rand_normal(&mut rng);
            let r = z * sigma2.sqrt();
            returns.push(r);
            sigma2 = omega + alpha * r * r + beta * sigma2;
            sigma2 = sigma2.max(1e-20);
        }

        returns
    }

    /// Normal random via Box-Muller transform (avoids rand_distr dependency).
    fn rand_normal(rng: &mut impl rand::Rng) -> f64 {
        loop {
            let u1: f64 = rng.random();
            let u2: f64 = rng.random();
            if u1 > 1e-30 {
                return (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos();
            }
        }
    }

    #[test]
    fn mle_recovers_garch11_params() {
        let true_omega = 0.00001;
        let true_alpha = 0.08;
        let true_beta = 0.88;

        let returns = generate_garch_data(true_omega, true_alpha, true_beta, 3000, 42);

        let fit = Garch11
            .fit(&returns, InnovationDist::Gaussian, None)
            .expect("GARCH(1,1) fit should succeed");

        assert!(fit.converged, "Optimizer should converge");

        // Recovered parameters should be in the right ballpark
        // With 3000 observations, we expect reasonable accuracy
        let rel_alpha = (fit.params.alpha - true_alpha).abs() / true_alpha;
        let rel_beta = (fit.params.beta - true_beta).abs() / true_beta;

        assert!(
            rel_alpha < 0.5,
            "alpha recovery: estimated={}, true={}, rel_err={}",
            fit.params.alpha,
            true_alpha,
            rel_alpha
        );
        assert!(
            rel_beta < 0.15,
            "beta recovery: estimated={}, true={}, rel_err={}",
            fit.params.beta,
            true_beta,
            rel_beta
        );

        // Persistence should be close
        let true_persistence = true_alpha + true_beta;
        let est_persistence = fit.params.persistence();
        assert!(
            (est_persistence - true_persistence).abs() < 0.10,
            "persistence: estimated={}, true={}",
            est_persistence,
            true_persistence
        );
    }

    #[test]
    fn fit_rejects_short_series() {
        let returns = [0.01, -0.01, 0.02];
        let result = Garch11.fit(&returns, InnovationDist::Gaussian, None);
        assert!(result.is_err());
    }

    #[test]
    fn fit_rejects_constant_series() {
        let returns = vec![0.0; 100];
        let result = Garch11.fit(&returns, InnovationDist::Gaussian, None);
        assert!(result.is_err());
    }

    #[test]
    fn garch_fit_summary_not_empty() {
        let returns = generate_garch_data(0.00001, 0.08, 0.88, 500, 123);
        let fit = Garch11
            .fit(&returns, InnovationDist::Gaussian, None)
            .expect("fit should succeed");
        let summary = fit.summary();
        assert!(!summary.is_empty());
        assert!(summary.contains("GARCH(1,1)"));
    }

    #[test]
    fn compare_garch_models_returns_sorted_by_bic() {
        let returns = generate_garch_data(0.00001, 0.08, 0.88, 1000, 99);

        let results = compare_garch_models(&returns, InnovationDist::Gaussian, None)
            .expect("compare_garch_models should succeed");

        assert!(!results.is_empty());

        // Check BIC is sorted ascending
        for i in 1..results.len() {
            assert!(
                results[i].bic >= results[i - 1].bic,
                "Results should be sorted by BIC"
            );
        }
    }

    #[test]
    fn auto_garch_returns_best() {
        let returns = generate_garch_data(0.00001, 0.08, 0.88, 1000, 77);
        let best = auto_garch(&returns, InnovationDist::Gaussian, None)
            .expect("auto_garch should succeed");

        // The best model should have the lowest BIC
        let all = compare_garch_models(&returns, InnovationDist::Gaussian, None).unwrap();
        assert!(
            (best.bic - all[0].bic).abs() < 1e-10,
            "auto_garch should return the model with lowest BIC"
        );
    }

    #[test]
    fn student_t_fit_works() {
        let returns = generate_garch_data(0.00001, 0.08, 0.88, 500, 55);
        let fit = Garch11
            .fit(&returns, InnovationDist::StudentT(8.0), None)
            .expect("Student-t fit should succeed");

        assert!(fit.converged);
        if let InnovationDist::StudentT(nu) = fit.params.dist {
            assert!(nu > 2.0, "Estimated dof should be > 2");
        } else {
            panic!("Expected Student-t distribution in result");
        }
    }

    #[test]
    fn numerical_stability_with_outliers() {
        let mut returns = generate_garch_data(0.00001, 0.08, 0.88, 500, 33);
        // Insert extreme outliers
        returns[100] = 0.15; // 15% return
        returns[200] = -0.12; // -12% return
        returns[300] = 0.20; // 20% return

        let fit = Garch11
            .fit(&returns, InnovationDist::Gaussian, None)
            .expect("Fit with outliers should succeed");

        // Parameters should be finite
        assert!(fit.params.omega.is_finite());
        assert!(fit.params.alpha.is_finite());
        assert!(fit.params.beta.is_finite());

        // No NaN in conditional variances
        for &s in &fit.conditional_variances {
            assert!(
                s.is_finite() && s > 0.0,
                "Conditional variance should be finite positive"
            );
        }
    }

    #[test]
    fn deterministic_results() {
        let returns = generate_garch_data(0.00001, 0.08, 0.88, 500, 42);

        let fit1 = Garch11
            .fit(&returns, InnovationDist::Gaussian, None)
            .expect("fit 1");
        let fit2 = Garch11
            .fit(&returns, InnovationDist::Gaussian, None)
            .expect("fit 2");

        assert!(
            (fit1.log_likelihood - fit2.log_likelihood).abs() < 1e-10,
            "Same inputs should produce identical outputs"
        );
        assert!(
            (fit1.params.alpha - fit2.params.alpha).abs() < 1e-10,
            "Same inputs should produce identical parameters"
        );
    }

    #[test]
    fn egarch_fit_works() {
        let returns = generate_garch_data(0.00001, 0.08, 0.88, 1000, 88);

        let fit = Egarch11
            .fit(&returns, InnovationDist::Gaussian, None)
            .expect("EGARCH fit should succeed");

        assert!(fit.converged);
        assert!(fit.log_likelihood.is_finite());
        assert!(fit.params.gamma.is_some());
    }

    #[test]
    fn gjr_garch_fit_works() {
        let returns = generate_garch_data(0.00001, 0.08, 0.88, 1000, 77);

        let fit = GjrGarch11
            .fit(&returns, InnovationDist::Gaussian, None)
            .expect("GJR-GARCH fit should succeed");

        assert!(fit.converged);
        assert!(fit.log_likelihood.is_finite());
        assert!(fit.params.gamma.is_some());
    }

    #[test]
    fn ljung_box_diagnostics_on_fit() {
        let returns = generate_garch_data(0.00001, 0.08, 0.88, 1000, 66);
        let fit = Garch11
            .fit(&returns, InnovationDist::Gaussian, None)
            .expect("fit should succeed");

        // Ljung-Box on squared standardized residuals
        let pval = fit.ljung_box_squared(10);
        assert!((0.0..=1.0).contains(&pval), "p-value should be in [0,1]");
    }

    #[test]
    fn arch_lm_diagnostics_on_fit() {
        let returns = generate_garch_data(0.00001, 0.08, 0.88, 1000, 55);
        let fit = Garch11
            .fit(&returns, InnovationDist::Gaussian, None)
            .expect("fit should succeed");

        let pval = fit.arch_lm_test(5);
        assert!((0.0..=1.0).contains(&pval), "p-value should be in [0,1]");
    }

    #[test]
    fn forecast_term_structure_monotone_converging() {
        let returns = generate_garch_data(0.00001, 0.08, 0.88, 1000, 44);
        let fit = Garch11
            .fit(&returns, InnovationDist::Gaussian, None)
            .expect("fit should succeed");

        let horizons = [1, 5, 10, 21, 63, 126, 252, 504, 1000];
        let forecasts = Garch11.forecast(&fit, &horizons, 252.0, None);

        let uncond = fit.params.unconditional_variance().unwrap();

        // Differences from unconditional should decrease
        for i in 1..forecasts.len() {
            let diff_prev = (forecasts[i - 1].variance - uncond).abs();
            let diff_curr = (forecasts[i].variance - uncond).abs();
            assert!(
                diff_curr <= diff_prev + 1e-15,
                "Forecast should converge monotonically to unconditional variance"
            );
        }

        // Far-out forecast should be very close to unconditional
        let last = forecasts.last().unwrap();
        assert!(
            (last.variance - uncond).abs() / uncond < 0.01,
            "1000-step forecast should be within 1% of unconditional"
        );
    }
}
