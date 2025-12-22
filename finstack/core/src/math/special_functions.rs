//! Special mathematical functions for financial computation.
//!
//! This module provides numerically stable implementations of special functions
//! commonly used in financial mathematics, including the error function, normal
//! distribution functions, and their inverses.
//!
//! **Implementation Note**: As of version 0.3.0, these functions are thin wrappers
//! around the battle-tested [`statrs`](https://crates.io/crates/statrs) crate,
//! which provides highly accurate, SIMD-optimized implementations.
//!
//! # Functions
//!
//! - [`erf`]: Error function (wrapper around `statrs`)
//! - [`norm_cdf`]: Standard normal cumulative distribution function (Φ)
//! - [`norm_pdf`]: Standard normal probability density function (φ)
//! - [`standard_normal_inv_cdf`]: Inverse standard normal CDF (Φ⁻¹)
//!
//! # Numerical Accuracy
//!
//! The `statrs` implementations prioritize:
//! - **Tail accuracy**: Critical for risk metrics (VaR, CVaR) and copula models
//! - **Boundary stability**: Proper handling of extreme values (±∞)
//! - **Determinism**: Identical results across platforms and compilers
//! - **Performance**: SIMD optimizations where available
//! - **Battle-tested**: Widely used in the Rust ecosystem
//!
//! # Use Cases
//!
//! - **Option pricing**: Black-Scholes formula requires Φ(d₁) and Φ(d₂)
//! - **Implied volatility**: Inverse CDF needed for smile calibration
//! - **Risk metrics**: VaR calculation uses Φ⁻¹(confidence level)
//! - **Copula models**: Credit correlation and CDO tranching
//! - **Monte Carlo**: Box-Muller transform uses Φ⁻¹
//!
//! # Examples
//!
//! ## Basic usage
//!
//! ```
//! use finstack_core::math::special_functions::{norm_cdf, norm_pdf, standard_normal_inv_cdf};
//!
//! // Standard normal CDF at zero should be 0.5
//! assert!((norm_cdf(0.0) - 0.5).abs() < 1e-6);
//!
//! // PDF at zero is 1/√(2π)
//! let expected = 1.0 / (2.0 * std::f64::consts::PI).sqrt();
//! assert!((norm_pdf(0.0) - expected).abs() < 1e-6);
//!
//! // Round-trip test for inverse CDF
//! let x = standard_normal_inv_cdf(0.84);
//! let p_back = norm_cdf(x);
//! assert!((p_back - 0.84).abs() < 1e-3);
//! ```
//!
//! ## Black-Scholes option pricing
//!
//! ```
//! use finstack_core::math::special_functions::norm_cdf;
//!
//! // Simplified Black-Scholes call price
//! fn black_scholes_call(s: f64, k: f64, r: f64, vol: f64, t: f64) -> f64 {
//!     let d1 = ((s / k).ln() + (r + 0.5 * vol * vol) * t) / (vol * t.sqrt());
//!     let d2 = d1 - vol * t.sqrt();
//!     
//!     s * norm_cdf(d1) - k * (-r * t).exp() * norm_cdf(d2)
//! }
//!
//! let call_price = black_scholes_call(100.0, 100.0, 0.05, 0.2, 1.0);
//! assert!(call_price > 0.0);
//! ```
//!
//! # References
//!
//! - **statrs crate**: The underlying implementation for all special functions.
//!   See <https://github.com/statrs-dev/statrs> for implementation details and accuracy benchmarks.
//!
//! - **Error Function**:
//!   - Abramowitz, M., & Stegun, I. A. (1964). *Handbook of Mathematical Functions*.
//!     National Bureau of Standards.
//!
//! - **Normal Distribution**:
//!   - Johnson, N. L., Kotz, S., & Balakrishnan, N. (1995). *Continuous Univariate
//!     Distributions, Volume 1* (2nd ed.). Wiley. Chapter 13.
//!
//! - **Inverse Normal CDF**:
//!   - Wichura, M. J. (1988). "Algorithm AS 241: The Percentage Points of the
//!     Normal Distribution." *Applied Statistics*, 37(3), 477-484.

/// Error function.
///
/// Computes the error function using the highly accurate implementation from `statrs`.
///
/// # Definition
///
/// ```text
/// erf(x) = (2/√π) ∫₀ˣ e^(-t²) dt
/// ```
///
/// # Arguments
///
/// * `x` - Input value (any real number)
///
/// # Returns
///
/// The error function erf(x) ∈ (-1, 1)
///
/// # Examples
///
/// ```rust
/// use finstack_core::math::special_functions::erf;
///
/// // erf(0) ≈ 0
/// assert!(erf(0.0).abs() < 1e-6);
///
/// // erf is odd: erf(-x) = -erf(x)
/// let x = 1.5;
/// assert!((erf(-x) + erf(x)).abs() < 1e-6);
///
/// // erf(∞) → 1
/// assert!((erf(5.0) - 1.0).abs() < 1e-5);
/// ```
///
/// # Implementation
///
/// This is a thin wrapper around `statrs::function::erf::erf`.
#[inline]
pub fn erf(x: f64) -> f64 {
    statrs::function::erf::erf(x)
}

/// Cumulative standard normal distribution function Φ(x).
///
/// Computes the probability that a standard normal random variable is less
/// than or equal to x.
///
/// # Definition
///
/// ```text
/// Φ(x) = P(Z ≤ x) where Z ~ N(0,1)
///      = (1/√(2π)) ∫_{-∞}^x e^(-t²/2) dt
///      = (1/2)[1 + erf(x/√2)]
/// ```
///
/// # Arguments
///
/// * `x` - Input value (any real number)
///
/// # Returns
///
/// Cumulative probability Φ(x) ∈ (0, 1)
///
/// # Numerical Stability
///
/// The `statrs` implementation uses highly accurate algorithms with proper tail handling,
/// which is critical for:
/// - Value-at-Risk with high confidence (99.9%)
/// - Credit correlation in copula models
/// - Base correlation calibration for CDO tranches
///
/// # Examples
///
/// ```rust
/// use finstack_core::math::special_functions::norm_cdf;
///
/// // Φ(0) = 0.5 (median of standard normal)
/// assert!((norm_cdf(0.0) - 0.5).abs() < 1e-6);
///
/// // Φ(-x) = 1 - Φ(x) (symmetry)
/// let x = 1.96; // 97.5th percentile
/// assert!((norm_cdf(-x) + norm_cdf(x) - 1.0).abs() < 1e-6);
///
/// // 95% confidence interval: [-1.96, 1.96]
/// assert!((norm_cdf(1.96) - 0.975).abs() < 1e-3);
/// assert!((norm_cdf(-1.96) - 0.025).abs() < 1e-3);
/// ```
///
/// # Implementation
///
/// This is a thin wrapper around `statrs::distribution::Normal::cdf`.
#[inline]
pub fn norm_cdf(x: f64) -> f64 {
    use statrs::distribution::{ContinuousCDF, Normal};
    // Standard normal: mean=0, std_dev=1
    // unwrap is safe because std_dev=1 is always valid
    let n = Normal::new(0.0, 1.0).unwrap_or_else(|_| unreachable!());
    n.cdf(x)
}

/// Standard normal probability density function φ(x).
///
/// Computes the probability density of the standard normal distribution at x.
///
/// # Definition
///
/// ```text
/// φ(x) = (1/√(2π)) e^(-x²/2)
/// ```
///
/// # Arguments
///
/// * `x` - Input value (any real number)
///
/// # Returns
///
/// Probability density φ(x) ∈ (0, 1/√(2π)]
/// Maximum value occurs at x = 0: φ(0) = 1/√(2π) ≈ 0.3989
///
/// # Examples
///
/// ```rust
/// use finstack_core::math::special_functions::norm_pdf;
///
/// // Maximum at x = 0
/// let max_density = norm_pdf(0.0);
/// assert!((max_density - 0.3989).abs() < 1e-4);
///
/// // Symmetric: φ(-x) = φ(x)
/// let x = 1.5;
/// assert!((norm_pdf(-x) - norm_pdf(x)).abs() < 1e-6);
///
/// // Approximately zero in tails (φ(5.0) ≈ 1.49e-6)
/// assert!(norm_pdf(5.0) < 2e-6);
/// ```
///
/// # Use Cases
///
/// - Option Greeks (vega, gamma) in Black-Scholes
/// - Maximum likelihood estimation
/// - Kernel density estimation
/// - Heat kernel in diffusion processes
///
/// # Implementation
///
/// This is a thin wrapper around `statrs::distribution::Normal::pdf`.
#[inline]
pub fn norm_pdf(x: f64) -> f64 {
    use statrs::distribution::{Continuous, Normal};
    // Standard normal: mean=0, std_dev=1
    // unwrap is safe because std_dev=1 is always valid
    let n = Normal::new(0.0, 1.0).unwrap_or_else(|_| unreachable!());
    n.pdf(x)
}

/// Inverse standard normal cumulative distribution function.
///
/// Computes the inverse of the standard normal CDF, returning the value x
/// such that Φ(x) = p. This function is particularly critical for:
/// - Base correlation calibration where tail behavior impacts conditional default probabilities
/// - Value-at-Risk calculations
/// - Implied volatility solving
///
/// # Arguments
/// * `p` - Probability in (0, 1)
///
/// # Returns
/// x such that Φ(x) = p
///
/// # Examples
///
/// ```rust
/// use finstack_core::math::special_functions::{standard_normal_inv_cdf, norm_cdf};
///
/// // Inverse of median should be zero
/// assert!((standard_normal_inv_cdf(0.5) - 0.0).abs() < 1e-6);
///
/// // Round-trip test
/// let x = standard_normal_inv_cdf(0.84);
/// let p_back = norm_cdf(x);
/// assert!((p_back - 0.84).abs() < 1e-6);
/// ```
///
/// # Implementation
///
/// This is a thin wrapper around `statrs::distribution::Normal::inverse_cdf`.
pub fn standard_normal_inv_cdf(p: f64) -> f64 {
    use statrs::distribution::{ContinuousCDF, Normal};
    // Standard normal: mean=0, std_dev=1
    // unwrap is safe because std_dev=1 is always valid
    let n = Normal::new(0.0, 1.0).unwrap_or_else(|_| unreachable!());
    n.inverse_cdf(p)
}

/// Student-t cumulative distribution function.
///
/// Computes the CDF of the Student-t distribution with the specified degrees of freedom.
/// For high degrees of freedom (df > 100), uses the normal approximation for efficiency.
///
/// # Definition
///
/// ```text
/// F(x; ν) = P(T ≤ x) where T ~ t(ν)
/// ```
///
/// # Arguments
///
/// * `x` - Input value (any real number)
/// * `df` - Degrees of freedom (must be > 0)
///
/// # Returns
///
/// Cumulative probability F(x; df) ∈ (0, 1)
///
/// # Use Cases
///
/// - **Copula models**: Student-t copula tail dependence calculations
/// - **Credit modeling**: Heavy-tailed default correlation
/// - **Risk metrics**: VaR with fat tails
/// - **Statistical tests**: t-tests, confidence intervals
///
/// # Examples
///
/// ```rust
/// use finstack_core::math::special_functions::student_t_cdf;
///
/// // CDF at zero should be 0.5 (symmetric distribution)
/// assert!((student_t_cdf(0.0, 5.0) - 0.5).abs() < 1e-6);
///
/// // High df approaches normal distribution
/// let t_cdf = student_t_cdf(1.96, 1000.0);
/// assert!((t_cdf - 0.975).abs() < 0.01);
/// ```
///
/// # Implementation
///
/// This is a thin wrapper around `statrs::distribution::StudentsT::cdf`.
/// For df > 100, uses the normal approximation for better performance.
#[inline]
pub fn student_t_cdf(x: f64, df: f64) -> f64 {
    if df > 100.0 {
        // High df: normal approximation is accurate and faster
        return norm_cdf(x);
    }

    use statrs::distribution::{ContinuousCDF, StudentsT};
    // StudentsT::new(location=0, scale=1, df) for standard t-distribution
    match StudentsT::new(0.0, 1.0, df) {
        Ok(dist) => dist.cdf(x),
        Err(_) => {
            // Fallback for invalid df (should not happen with df > 0)
            norm_cdf(x)
        }
    }
}

/// Inverse Student-t cumulative distribution function.
///
/// Computes the inverse CDF (quantile function) of the Student-t distribution.
///
/// # Arguments
///
/// * `p` - Probability in (0, 1)
/// * `df` - Degrees of freedom (must be > 0)
///
/// # Returns
///
/// x such that F(x; df) = p
///
/// # Examples
///
/// ```rust
/// use finstack_core::math::special_functions::{student_t_inv_cdf, student_t_cdf};
///
/// // Inverse of median should be zero
/// assert!((student_t_inv_cdf(0.5, 5.0) - 0.0).abs() < 1e-6);
///
/// // Round-trip test
/// let x = student_t_inv_cdf(0.95, 10.0);
/// let p_back = student_t_cdf(x, 10.0);
/// assert!((p_back - 0.95).abs() < 1e-6);
/// ```
///
/// # Implementation
///
/// This is a thin wrapper around `statrs::distribution::StudentsT::inverse_cdf`.
pub fn student_t_inv_cdf(p: f64, df: f64) -> f64 {
    if df > 100.0 {
        // High df: normal approximation
        return standard_normal_inv_cdf(p);
    }

    use statrs::distribution::{ContinuousCDF, StudentsT};
    match StudentsT::new(0.0, 1.0, df) {
        Ok(dist) => dist.inverse_cdf(p),
        Err(_) => standard_normal_inv_cdf(p),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_erf() {
        // Test known values with reasonable tolerance for approximation
        assert!(
            (erf(0.0) - 0.0).abs() < 1e-6,
            "erf(0) should be 0, got {}",
            erf(0.0)
        );
        assert!(
            (erf(1.0) - 0.8427007929).abs() < 1e-4,
            "erf(1) should be ~0.8427, got {}",
            erf(1.0)
        );
        assert!(
            (erf(-1.0) - (-0.8427007929)).abs() < 1e-4,
            "erf(-1) should be ~-0.8427, got {}",
            erf(-1.0)
        );
    }

    #[test]
    fn test_norm_cdf() {
        // Test known values
        assert!((norm_cdf(0.0) - 0.5).abs() < 1e-6);
        assert!((norm_cdf(1.0) - 0.8413447460685429).abs() < 1e-6);
        assert!((norm_cdf(-1.0) - 0.15865525393145705).abs() < 1e-6);

        // Test extreme values
        assert!(norm_cdf(-10.0) < 1e-10);
        assert!(norm_cdf(10.0) > 1.0 - 1e-10);
    }

    #[test]
    fn test_norm_pdf() {
        use std::f64::consts::PI;
        // Test known values
        assert!((norm_pdf(0.0) - (1.0 / (2.0 * PI).sqrt())).abs() < 1e-12);

        // Test symmetry
        assert!((norm_pdf(1.0) - norm_pdf(-1.0)).abs() < 1e-12);

        // Test non-negativity
        assert!(norm_pdf(5.0) >= 0.0);
    }

    #[test]
    fn test_standard_normal_inv_cdf() {
        // Test known values
        assert!((standard_normal_inv_cdf(0.5) - 0.0).abs() < 1e-6);
        assert!((standard_normal_inv_cdf(0.8413447460685429) - 1.0).abs() < 1e-3);
        assert!((standard_normal_inv_cdf(0.15865525393145705) - (-1.0)).abs() < 1e-3);

        // Test extreme values
        assert!(standard_normal_inv_cdf(1e-10) < -5.0);
        assert!(standard_normal_inv_cdf(1.0 - 1e-10) > 5.0);
    }

    #[test]
    fn test_normal_cdf_inv_cdf_roundtrip() {
        let test_values = [0.1, 0.25, 0.5, 0.75, 0.9]; // Skip extreme values for robustness

        for &p in &test_values {
            let x = standard_normal_inv_cdf(p);
            let p_back = norm_cdf(x);
            assert!(
                (p - p_back).abs() < 1e-3, // Relaxed tolerance for enhanced tail behavior
                "Failed roundtrip for p={}, got x={}, p_back={}",
                p,
                x,
                p_back
            );
        }
    }

    #[test]
    fn test_extreme_tail_behavior() {
        // Test enhanced tail behavior for extreme values critical to copula models
        let extreme_values = [1e-12, 1e-10, 1e-8, 1e-6];

        for &p in &extreme_values {
            let x_low = standard_normal_inv_cdf(p);
            let x_high = standard_normal_inv_cdf(1.0 - p);

            // Inverse CDF should be finite and reasonable
            assert!(
                x_low.is_finite(),
                "Inverse CDF should be finite for p={}",
                p
            );
            assert!(
                x_high.is_finite(),
                "Inverse CDF should be finite for p={}",
                1.0 - p
            );

            // Should maintain approximate symmetry (allow for numerical precision limits)
            let symmetry_error = (x_low + x_high).abs();
            assert!(
                symmetry_error < 0.01, // Relaxed tolerance for extreme tail behavior
                "Symmetry violated: x_low={}, x_high={} for p={}, error={}",
                x_low,
                x_high,
                p,
                symmetry_error
            );

            // CDF should be stable in extreme tails
            let p_back_low = norm_cdf(x_low);
            let p_back_high = norm_cdf(x_high);

            assert!(
                p_back_low.is_finite(),
                "CDF should be finite for x={}",
                x_low
            );
            assert!(
                p_back_high.is_finite(),
                "CDF should be finite for x={}",
                x_high
            );

            // Should be bounded properly
            assert!((0.0..=1.0).contains(&p_back_low));
            assert!((0.0..=1.0).contains(&p_back_high));

            // Test roundtrip accuracy in tail regions (more forgiving tolerance)
            if p >= 1e-10 {
                let roundtrip_error_low = (p - p_back_low).abs() / p; // Relative error
                let roundtrip_error_high = ((1.0 - p) - p_back_high).abs() / (1.0 - p);

                assert!(
                    roundtrip_error_low < 0.1, // 10% relative error tolerance in extreme tails
                    "Poor roundtrip accuracy in tail: p={}, x={}, p_back={}, rel_error={}",
                    p,
                    x_low,
                    p_back_low,
                    roundtrip_error_low
                );
                assert!(
                    roundtrip_error_high < 0.1,
                    "Poor roundtrip accuracy in tail: p={}, x={}, p_back={}, rel_error={}",
                    1.0 - p,
                    x_high,
                    p_back_high,
                    roundtrip_error_high
                );
            }
        }
    }

    #[test]
    fn test_numerical_stability_correlations() {
        // Test numerical stability for extreme correlation values used in copula models
        let extreme_correlations: [f64; 8] = [
            1e-10,
            1e-8,
            1e-6,
            0.001,
            0.999,
            1.0 - 1e-6,
            1.0 - 1e-8,
            1.0 - 1e-10,
        ];

        for &rho in &extreme_correlations {
            let sqrt_rho = rho.sqrt();
            let sqrt_one_minus_rho = (1.0 - rho).sqrt();

            // These should be finite and reasonable
            assert!(
                sqrt_rho.is_finite(),
                "sqrt(ρ) should be finite for ρ={}",
                rho
            );
            assert!(
                sqrt_one_minus_rho.is_finite(),
                "sqrt(1-ρ) should be finite for ρ={}",
                rho
            );

            // Test conditional probability calculation stability
            let default_threshold = standard_normal_inv_cdf(0.05); // 5% default prob
            for market_factor in [-3.0, -1.0, 0.0, 1.0, 3.0] {
                let conditional_threshold =
                    (default_threshold - sqrt_rho * market_factor) / sqrt_one_minus_rho;
                let cond_prob = norm_cdf(conditional_threshold);

                assert!(
                    cond_prob.is_finite(),
                    "Conditional probability should be finite for ρ={}, Z={}",
                    rho,
                    market_factor
                );
                assert!(
                    (0.0..=1.0).contains(&cond_prob),
                    "Conditional probability should be in [0,1]: got {} for ρ={}, Z={}",
                    cond_prob,
                    rho,
                    market_factor
                );
            }
        }
    }

    #[test]
    fn test_student_t_cdf() {
        // Test CDF at zero (should be 0.5 for symmetric distribution)
        assert!((student_t_cdf(0.0, 5.0) - 0.5).abs() < 1e-6);
        assert!((student_t_cdf(0.0, 10.0) - 0.5).abs() < 1e-6);

        // Test known values from statistical tables
        // t-distribution with df=5, x=-2.0 should give CDF ≈ 0.051
        let cdf = student_t_cdf(-2.0, 5.0);
        assert!(
            (cdf - 0.051).abs() < 0.002,
            "CDF(-2.0, df=5) = {}, expected ~0.051",
            cdf
        );

        // Test symmetry: F(-x) = 1 - F(x)
        let x = 1.5;
        let df = 5.0;
        let cdf_neg = student_t_cdf(-x, df);
        let cdf_pos = student_t_cdf(x, df);
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
    fn test_student_t_cdf_approaches_normal() {
        // With high df, Student-t approaches Normal
        let x = -1.5;
        let t_cdf = student_t_cdf(x, 200.0);
        let normal_cdf = norm_cdf(x);

        assert!(
            (t_cdf - normal_cdf).abs() < 0.01,
            "High df t-distribution should approximate normal: t={}, normal={}",
            t_cdf,
            normal_cdf
        );

        // Test fallback to normal for very high df
        let t_cdf_high = student_t_cdf(x, 1000.0);
        assert!(
            (t_cdf_high - normal_cdf).abs() < 0.001,
            "Very high df should be almost identical to normal"
        );
    }

    #[test]
    fn test_student_t_inv_cdf() {
        // Test inverse of median should be zero
        assert!((student_t_inv_cdf(0.5, 5.0) - 0.0).abs() < 1e-6);
        assert!((student_t_inv_cdf(0.5, 10.0) - 0.0).abs() < 1e-6);

        // Test round-trip
        let p = 0.95;
        let df = 10.0;
        let x = student_t_inv_cdf(p, df);
        let p_back = student_t_cdf(x, df);
        assert!(
            (p_back - p).abs() < 1e-6,
            "Round-trip failed: p={}, x={}, p_back={}",
            p,
            x,
            p_back
        );
    }

    #[test]
    fn test_student_t_roundtrip() {
        let test_probs = [0.1, 0.25, 0.5, 0.75, 0.9, 0.95];
        let test_dfs = [3.0, 5.0, 10.0, 30.0];

        for &p in &test_probs {
            for &df in &test_dfs {
                let x = student_t_inv_cdf(p, df);
                let p_back = student_t_cdf(x, df);
                assert!(
                    (p - p_back).abs() < 1e-5,
                    "Round-trip failed for p={}, df={}: got x={}, p_back={}",
                    p,
                    df,
                    x,
                    p_back
                );
            }
        }
    }
}
