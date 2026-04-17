//! Tail dependence metric for copula diagnostics.
//!
//! Measures the probability of joint extreme defaults - a key indicator
//! of whether the copula model adequately captures stress scenarios.
//!
//! # Definition
//!
//! Lower tail dependence coefficient:
//! ```text
//! λ_L = lim_{u→0} P(U₂ ≤ u | U₁ ≤ u)
//! ```
//!
//! - **Gaussian copula**: λ_L = 0 (no tail dependence)
//! - **Student-t copula**: λ_L > 0 (positive tail dependence)
//!
//! # Financial Interpretation
//!
//! - λ_L = 0: Extreme joint defaults are "rare" (Gaussian assumption)
//! - λ_L > 0: Extreme joint defaults cluster (realistic for stress)
//!
//! Higher tail dependence means:
//! - Equity tranches: Higher expected loss in stress
//! - Senior tranches: Higher unexpected loss risk

use crate::instruments::credit_derivatives::cds_tranche::copula::CopulaSpec;
use crate::instruments::credit_derivatives::cds_tranche::pricer::CDSTranchePricer;
use crate::instruments::credit_derivatives::cds_tranche::CDSTranche;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Calculator for tail dependence coefficient.
///
/// Returns the lower tail dependence coefficient λ_L of the copula model
/// being used for tranche pricing. This is a diagnostic metric that
/// indicates whether the model captures joint extreme defaults.
pub(crate) struct TailDependenceCalculator;

impl MetricCalculator for TailDependenceCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let tranche = context
            .instrument
            .as_any()
            .downcast_ref::<CDSTranche>()
            .ok_or(finstack_core::Error::Input(
                finstack_core::InputError::Invalid,
            ))?;

        // Get the credit index data to determine correlation
        let index_data = match context.curves.get_credit_index(&tranche.credit_index_id) {
            Ok(data) => data,
            Err(_) => return Ok(f64::NAN),
        };

        // Get correlation for the detachment point
        let correlation = index_data
            .base_correlation_curve
            .correlation(tranche.detach_pct);

        // Get the copula from the pricer configuration
        let pricer = CDSTranchePricer::new();
        let copula_spec = &pricer.config().copula_spec;

        // Calculate tail dependence based on copula type
        let lambda = match copula_spec {
            CopulaSpec::Gaussian => 0.0,
            CopulaSpec::StudentT { degrees_of_freedom } => {
                calculate_student_t_tail_dependence(correlation, *degrees_of_freedom)
            }
            CopulaSpec::RandomFactorLoading { loading_volatility } => {
                // RFL has implicit tail dependence through stochastic correlation
                calculate_rfl_tail_dependence(correlation, *loading_volatility)
            }
            CopulaSpec::MultiFactor { .. } => {
                // Multi-factor Gaussian still has zero tail dependence
                0.0
            }
        };

        Ok(lambda)
    }
}

/// Calculate tail dependence for Student-t copula.
///
/// λ_L = 2 · t_{ν+1}(-√((ν+1)(1-ρ)/(1+ρ)))
fn calculate_student_t_tail_dependence(correlation: f64, df: f64) -> f64 {
    let rho = correlation.clamp(0.001, 0.999);
    let nu = df.max(2.1);

    let arg = -((nu + 1.0) * (1.0 - rho) / (1.0 + rho)).sqrt();

    // Use accurate Student-t CDF from finstack_core
    let t_cdf = student_t_cdf_local(arg, nu + 1.0);

    2.0 * t_cdf
}

/// Calculate approximate tail dependence for RFL copula.
///
/// RFL has implicit tail dependence through the high-loading tail.
fn calculate_rfl_tail_dependence(correlation: f64, loading_vol: f64) -> f64 {
    let mean_loading = correlation.clamp(0.0, 1.0).sqrt();

    // High loading tail contribution
    let high_loading = (mean_loading + 2.0 * loading_vol).min(0.99);
    let effective_high_corr = high_loading * high_loading;

    // Rough approximation: P(high loading) × impact
    let prob_high = 1.0 - finstack_core::math::norm_cdf(2.0);
    prob_high * effective_high_corr.sqrt() * 0.5
}

/// Calculate Student-t CDF using finstack_core's implementation.
///
/// For high degrees of freedom (df > 100), uses the normal approximation
/// which is accurate and faster. Otherwise uses the exact Student-t CDF.
fn student_t_cdf_local(x: f64, df: f64) -> f64 {
    finstack_core::math::student_t_cdf(x, df)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_student_t_cdf_accuracy() {
        // Compare against known values from statistical tables
        // t-distribution with df=5, x=-2.0 should give CDF ≈ 0.0510
        let df = 5.0;
        let x = -2.0;
        let cdf = student_t_cdf_local(x, df);

        // Expected value from statistical tables
        assert!(
            (cdf - 0.051).abs() < 0.002,
            "CDF({}, df={}) = {}, expected ~0.051",
            x,
            df,
            cdf
        );
    }

    #[test]
    fn test_student_t_cdf_symmetric() {
        // Student-t CDF should be symmetric: F(-x) = 1 - F(x)
        let df = 5.0;
        let x = 1.5;
        let cdf_neg = student_t_cdf_local(-x, df);
        let cdf_pos = student_t_cdf_local(x, df);

        assert!(
            (cdf_neg + cdf_pos - 1.0).abs() < 1e-10,
            "CDF symmetry violated: F(-{}) + F({}) = {} + {} ≠ 1",
            x,
            x,
            cdf_neg,
            cdf_pos
        );
    }

    #[test]
    fn test_student_t_cdf_high_df_approaches_normal() {
        // With high df, Student-t approaches Normal
        let x = -1.5;
        let t_cdf = student_t_cdf_local(x, 200.0);
        let normal_cdf = finstack_core::math::norm_cdf(x);

        assert!(
            (t_cdf - normal_cdf).abs() < 0.01,
            "High df t-distribution should approximate normal: t={}, normal={}",
            t_cdf,
            normal_cdf
        );
    }

    #[test]
    fn test_gaussian_tail_dependence() {
        // Gaussian has zero tail dependence
        let lambda = calculate_student_t_tail_dependence(0.5, f64::INFINITY);
        // With infinite df, should approach 0
        assert!(lambda < 0.01);
    }

    #[test]
    fn test_student_t_tail_dependence_positive() {
        let lambda = calculate_student_t_tail_dependence(0.5, 5.0);
        assert!(
            lambda > 0.0,
            "Student-t should have positive tail dependence"
        );
        assert!(lambda < 1.0);
    }

    #[test]
    fn test_tail_dependence_increases_with_correlation() {
        let lambda_low = calculate_student_t_tail_dependence(0.2, 5.0);
        let lambda_high = calculate_student_t_tail_dependence(0.8, 5.0);

        assert!(
            lambda_high > lambda_low,
            "Higher correlation should give higher tail dependence"
        );
    }

    #[test]
    fn test_tail_dependence_increases_with_lower_df() {
        let lambda_high_df = calculate_student_t_tail_dependence(0.5, 20.0);
        let lambda_low_df = calculate_student_t_tail_dependence(0.5, 4.0);

        assert!(
            lambda_low_df > lambda_high_df,
            "Lower df should give higher tail dependence"
        );
    }

    #[test]
    fn test_rfl_tail_dependence() {
        let lambda = calculate_rfl_tail_dependence(0.5, 0.15);

        // RFL should have small positive tail dependence
        assert!(lambda >= 0.0);
        assert!(lambda < 0.1);
    }
}
