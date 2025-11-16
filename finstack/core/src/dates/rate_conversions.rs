//! Interest rate compounding convention conversion utilities.
//!
//! This module provides standard conversion functions between different interest rate
//! compounding conventions commonly used in financial markets. These conversions are
//! essential for comparing rates across different instruments and markets that use
//! different quoting conventions.
//!
//! # Market Standards and Conventions
//!
//! Different financial markets quote interest rates using different compounding conventions:
//!
//! ## Simple Interest (Money Market Convention)
//! - Used in: Short-term money markets (e.g., LIBOR, SOFR for tenors < 1 year)
//! - Formula: FV = PV × (1 + r × t)
//! - Where: r = simple rate, t = year fraction
//!
//! ## Periodic Compounding (Bond Equivalent Yield)
//! - Used in: Bond markets, swap rates
//! - US Treasuries: Semi-annual compounding (n = 2)
//! - Corporate bonds: Often annual compounding (n = 1)
//! - Formula: FV = PV × (1 + r/n)^(n×t)
//! - Where: n = compounding frequency per year
//!
//! ## Continuous Compounding
//! - Used in: Derivatives pricing (Black-Scholes, interest rate trees)
//! - Zero-coupon curve construction
//! - Formula: FV = PV × e^(r×t)
//!
//! # ISDA Standards
//!
//! - Interest rate swaps are typically quoted with semi-annual compounding
//! - Conversion to continuous rates is needed for:
//!   - Black-Scholes option pricing
//!   - Bootstrapping zero curves
//!   - Interest rate tree construction
//!
//! # Examples
//!
//! ## Converting Simple to Periodic Rate
//! ```
//! use finstack_core::dates::rate_conversions::simple_to_periodic;
//!
//! // 5% simple rate over 6 months (0.5 year fraction)
//! // Convert to equivalent semi-annual rate
//! let simple_rate = 0.05;
//! let year_fraction = 0.5;
//! let periodic_rate = simple_to_periodic(simple_rate, year_fraction, 2);
//!
//! // For short periods, rates are approximately equal
//! assert!((periodic_rate - simple_rate).abs() < 0.001);
//! ```
//!
//! ## Converting Periodic to Continuous Rate
//! ```
//! use finstack_core::dates::rate_conversions::periodic_to_continuous;
//!
//! // Convert 5% semi-annual rate to continuous
//! let periodic_rate = 0.05;
//! let continuous_rate = periodic_to_continuous(periodic_rate, 2);
//!
//! // Continuous rate is slightly lower for positive rates
//! assert!(continuous_rate < periodic_rate);
//! ```
//!
//! ## Round-Trip Conversion
//! ```
//! use finstack_core::dates::rate_conversions::{periodic_to_continuous, continuous_to_periodic};
//!
//! let original_rate = 0.05;
//! let continuous = periodic_to_continuous(original_rate, 2);
//! let back_to_periodic = continuous_to_periodic(continuous, 2);
//!
//! // Round-trip should preserve precision
//! assert!((original_rate - back_to_periodic).abs() < 1e-12);
//! ```
//!
//! # References
//!
//! - Hull, John C. "Options, Futures, and Other Derivatives" (Chapter 4: Interest Rates)
//! - ISDA Definitions (2006): Interest Rate and Currency Exchange Definitions
//! - Tuckman & Serrat: "Fixed Income Securities" (Chapter 1: Prices, Discount Factors, and Arbitrage)

use crate::{Error, Result};

/// Convert a simple (linear) interest rate to a periodically compounded rate.
///
/// Simple interest accrues linearly: FV = PV × (1 + r × t)
/// Periodic compounding: FV = PV × (1 + r/n)^(n×t)
///
/// This function solves for the periodic rate that gives the same future value
/// as the simple rate over the specified time period.
///
/// # Arguments
///
/// * `simple_rate` - The simple (linear) interest rate (e.g., 0.05 for 5%)
/// * `year_fraction` - The time period as a fraction of a year (from day-count convention)
/// * `periods_per_year` - Compounding frequency (e.g., 2 for semi-annual, 4 for quarterly)
///
/// # Returns
///
/// The equivalent periodically compounded rate.
///
/// # Errors
///
/// Returns an error if:
/// - `periods_per_year` is zero
/// - `year_fraction` is negative
/// - The resulting calculation would be undefined (e.g., negative value under root)
///
/// # Formula
///
/// Given: FV_simple = 1 + r_simple × t
///
/// Want: r_periodic such that (1 + r_periodic/n)^(n×t) = 1 + r_simple × t
///
/// Solution: r_periodic = n × [(1 + r_simple × t)^(1/(n×t)) - 1]
///
/// # Examples
///
/// ```
/// use finstack_core::dates::rate_conversions::simple_to_periodic;
///
/// // 5% simple rate over 1 year, convert to semi-annual
/// let periodic = simple_to_periodic(0.05, 1.0, 2).expect("conversion should succeed");
/// assert!((periodic - 0.04939).abs() < 0.00001);
///
/// // For very short periods, rates converge
/// let periodic_short = simple_to_periodic(0.05, 0.01, 2).expect("conversion should succeed");
/// assert!((periodic_short - 0.05).abs() < 0.0001);
/// ```
#[inline]
pub fn simple_to_periodic(
    simple_rate: f64,
    year_fraction: f64,
    periods_per_year: u32,
) -> Result<f64> {
    if periods_per_year == 0 {
        return Err(Error::Validation(
            "periods_per_year must be positive in simple_to_periodic".to_string(),
        ));
    }

    if year_fraction < 0.0 {
        return Err(Error::Validation(
            "year_fraction must be non-negative in simple_to_periodic".to_string(),
        ));
    }

    // Handle edge case: zero year fraction
    if year_fraction.abs() < 1e-15 {
        return Ok(simple_rate);
    }

    let n = periods_per_year as f64;
    let one_plus_simple = 1.0 + simple_rate * year_fraction;

    if one_plus_simple <= 0.0 {
        return Err(Error::Validation(
            "simple rate and year fraction combination results in negative discount factor in simple_to_periodic".to_string(),
        ));
    }

    let exponent = 1.0 / (n * year_fraction);
    let periodic_rate = n * (one_plus_simple.powf(exponent) - 1.0);

    Ok(periodic_rate)
}

/// Convert a periodically compounded rate to a simple (linear) rate.
///
/// This is the inverse of `simple_to_periodic`.
///
/// # Arguments
///
/// * `periodic_rate` - The periodically compounded rate (e.g., 0.05 for 5%)
/// * `year_fraction` - The time period as a fraction of a year
/// * `periods_per_year` - Compounding frequency (e.g., 2 for semi-annual)
///
/// # Returns
///
/// The equivalent simple interest rate.
///
/// # Errors
///
/// Returns an error if:
/// - `periods_per_year` is zero
/// - `year_fraction` is negative or zero
/// - The periodic rate is too negative (would result in negative discount factor)
///
/// # Formula
///
/// simple_rate = [(1 + periodic_rate/n)^(n×t) - 1] / t
///
/// # Examples
///
/// ```
/// use finstack_core::dates::rate_conversions::{simple_to_periodic, periodic_to_simple};
///
/// // Round-trip conversion
/// let original = 0.05;
/// let periodic = simple_to_periodic(original, 0.5, 2).expect("conversion should succeed");
/// let back = periodic_to_simple(periodic, 0.5, 2).expect("conversion should succeed");
/// assert!((original - back).abs() < 1e-12);
/// ```
#[inline]
pub fn periodic_to_simple(
    periodic_rate: f64,
    year_fraction: f64,
    periods_per_year: u32,
) -> Result<f64> {
    if periods_per_year == 0 {
        return Err(Error::Validation(
            "periods_per_year must be positive in periodic_to_simple".to_string(),
        ));
    }

    if year_fraction <= 0.0 {
        return Err(Error::Validation(
            "year_fraction must be positive for periodic_to_simple conversion".to_string(),
        ));
    }

    let n = periods_per_year as f64;
    let one_plus_periodic = 1.0 + periodic_rate / n;

    if one_plus_periodic <= 0.0 {
        return Err(Error::Validation(
            "periodic rate results in negative discount factor in periodic_to_simple".to_string(),
        ));
    }

    let future_value = one_plus_periodic.powf(n * year_fraction);
    let simple_rate = (future_value - 1.0) / year_fraction;

    Ok(simple_rate)
}

/// Convert a periodically compounded rate to a continuously compounded rate.
///
/// Periodic: FV = PV × (1 + r/n)^n
/// Continuous: FV = PV × e^r
///
/// # Arguments
///
/// * `periodic_rate` - The periodically compounded rate (e.g., 0.05 for 5%)
/// * `periods_per_year` - Compounding frequency (e.g., 2 for semi-annual)
///
/// # Returns
///
/// The equivalent continuously compounded rate.
///
/// # Errors
///
/// Returns an error if:
/// - `periods_per_year` is zero
/// - The periodic rate is too negative (would result in non-positive value)
///
/// # Formula
///
/// continuous_rate = n × ln(1 + periodic_rate/n)
///
/// Or equivalently for annual comparison:
/// continuous_rate = ln(1 + periodic_rate/n)^n
///
/// # Examples
///
/// ```
/// use finstack_core::dates::rate_conversions::periodic_to_continuous;
///
/// // 5% semi-annual to continuous
/// let continuous = periodic_to_continuous(0.05, 2).expect("conversion should succeed");
/// assert!((continuous - 0.04939).abs() < 0.00001);
///
/// // 10% quarterly to continuous
/// let continuous_q = periodic_to_continuous(0.10, 4).expect("conversion should succeed");
/// assert!(continuous_q < 0.10); // Continuous rate is lower
/// ```
///
/// # ISDA Reference
///
/// ISDA 2006 Definitions specify that swap rates are quoted with semi-annual
/// compounding. Converting to continuous is standard for zero curve bootstrapping
/// and derivatives pricing.
#[inline]
pub fn periodic_to_continuous(periodic_rate: f64, periods_per_year: u32) -> Result<f64> {
    if periods_per_year == 0 {
        return Err(Error::Validation(
            "periods_per_year must be positive in periodic_to_continuous".to_string(),
        ));
    }

    let n = periods_per_year as f64;
    let one_plus_periodic = 1.0 + periodic_rate / n;

    if one_plus_periodic <= 0.0 {
        return Err(Error::Validation(
            "periodic rate results in non-positive value for continuous conversion in periodic_to_continuous".to_string(),
        ));
    }

    // ln((1 + r/n)^n) = n × ln(1 + r/n)
    let continuous_rate = n * one_plus_periodic.ln();

    Ok(continuous_rate)
}

/// Convert a continuously compounded rate to a periodically compounded rate.
///
/// This is the inverse of `periodic_to_continuous`.
///
/// # Arguments
///
/// * `continuous_rate` - The continuously compounded rate (e.g., 0.05 for 5%)
/// * `periods_per_year` - Target compounding frequency (e.g., 2 for semi-annual)
///
/// # Returns
///
/// The equivalent periodically compounded rate.
///
/// # Errors
///
/// Returns an error if `periods_per_year` is zero.
///
/// # Formula
///
/// periodic_rate = n × (e^(continuous_rate/n) - 1)
///
/// # Examples
///
/// ```
/// use finstack_core::dates::rate_conversions::{periodic_to_continuous, continuous_to_periodic};
///
/// // Round-trip conversion
/// let original = 0.05;
/// let continuous = periodic_to_continuous(original, 2).expect("conversion should succeed");
/// let back = continuous_to_periodic(continuous, 2).expect("conversion should succeed");
/// assert!((original - back).abs() < 1e-14);
/// ```
#[inline]
pub fn continuous_to_periodic(continuous_rate: f64, periods_per_year: u32) -> Result<f64> {
    if periods_per_year == 0 {
        return Err(Error::Validation(
            "periods_per_year must be positive in continuous_to_periodic".to_string(),
        ));
    }

    let n = periods_per_year as f64;
    let periodic_rate = n * ((continuous_rate / n).exp() - 1.0);

    Ok(periodic_rate)
}

/// Convert a simple rate to a continuously compounded rate.
///
/// Convenience function that combines `simple_to_periodic` and `periodic_to_continuous`.
///
/// # Arguments
///
/// * `simple_rate` - The simple interest rate
/// * `year_fraction` - The time period as a fraction of a year
///
/// # Returns
///
/// The equivalent continuously compounded rate.
///
/// # Examples
///
/// ```
/// use finstack_core::dates::rate_conversions::simple_to_continuous;
///
/// let continuous = simple_to_continuous(0.05, 1.0).expect("conversion should succeed");
/// assert!((continuous - 0.04879).abs() < 0.00001);
/// ```
#[inline]
pub fn simple_to_continuous(simple_rate: f64, year_fraction: f64) -> Result<f64> {
    if year_fraction < 0.0 {
        return Err(Error::Validation(
            "year_fraction must be non-negative in simple_to_continuous".to_string(),
        ));
    }

    if year_fraction.abs() < 1e-15 {
        return Ok(simple_rate);
    }

    let one_plus_simple = 1.0 + simple_rate * year_fraction;

    if one_plus_simple <= 0.0 {
        return Err(Error::Validation(
            "simple rate and year fraction combination results in negative discount factor in simple_to_continuous".to_string(),
        ));
    }

    let continuous_rate = one_plus_simple.ln() / year_fraction;
    Ok(continuous_rate)
}

/// Convert a continuously compounded rate to a simple rate.
///
/// This is the inverse of `simple_to_continuous`.
///
/// # Arguments
///
/// * `continuous_rate` - The continuously compounded rate
/// * `year_fraction` - The time period as a fraction of a year
///
/// # Returns
///
/// The equivalent simple interest rate.
///
/// # Examples
///
/// ```
/// use finstack_core::dates::rate_conversions::{simple_to_continuous, continuous_to_simple};
///
/// let original = 0.05;
/// let continuous = simple_to_continuous(original, 1.0).expect("conversion should succeed");
/// let back = continuous_to_simple(continuous, 1.0).expect("conversion should succeed");
/// assert!((original - back).abs() < 1e-14);
/// ```
#[inline]
pub fn continuous_to_simple(continuous_rate: f64, year_fraction: f64) -> Result<f64> {
    if year_fraction <= 0.0 {
        return Err(Error::Validation(
            "year_fraction must be positive for continuous_to_simple conversion".to_string(),
        ));
    }

    let simple_rate = ((continuous_rate * year_fraction).exp() - 1.0) / year_fraction;
    Ok(simple_rate)
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f64 = 1e-12;
    const LOOSE_EPSILON: f64 = 1e-6;

    #[test]
    fn test_simple_to_periodic_known_values() {
        // 5% simple over 0.5 years should equal periodic for one compounding period
        let simple = 0.05;
        let yf = 0.5;
        let periodic = simple_to_periodic(simple, yf, 2).expect("conversion should succeed");

        // (1 + 0.05*0.5) = 1.025
        // Want: (1 + r/2)^1 = 1.025
        // So: r/2 = 0.025, r = 0.05
        assert!((periodic - 0.05).abs() < LOOSE_EPSILON);

        // 5% simple over 1 year, semi-annual compounding
        let periodic_1y = simple_to_periodic(0.05, 1.0, 2).expect("conversion should succeed");
        // (1 + 0.05) = (1 + r/2)^2
        // 1.05 = (1 + r/2)^2
        // sqrt(1.05) = 1 + r/2
        // r = 2 * (sqrt(1.05) - 1) ≈ 0.04939
        assert!((periodic_1y - 0.04939015319).abs() < LOOSE_EPSILON);
    }

    #[test]
    fn test_periodic_to_continuous_known_values() {
        // 5% semi-annual → continuous
        let periodic = 0.05;
        let continuous = periodic_to_continuous(periodic, 2).expect("conversion should succeed");

        // Formula: n × ln(1 + r/n) = 2 × ln(1.025)
        // 2 × 0.024693... ≈ 0.049385225181
        assert!((continuous - 0.049385225181).abs() < LOOSE_EPSILON);

        // Annual compounding: 1 × ln(1 + r/1) = ln(1.05)
        let continuous_annual = periodic_to_continuous(0.05, 1).expect("conversion should succeed");
        // ln(1.05) ≈ 0.048790164169
        assert!((continuous_annual - 0.048790164169).abs() < LOOSE_EPSILON);
    }

    #[test]
    fn test_periodic_continuous_round_trip() {
        let test_cases = vec![
            (0.01, 1),  // 1% annual
            (0.05, 2),  // 5% semi-annual
            (0.10, 4),  // 10% quarterly
            (0.15, 12), // 15% monthly
        ];

        for (periodic, freq) in test_cases {
            let continuous =
                periodic_to_continuous(periodic, freq).expect("conversion should succeed");
            let back_to_periodic =
                continuous_to_periodic(continuous, freq).expect("conversion should succeed");
            assert!(
                (periodic - back_to_periodic).abs() < EPSILON,
                "Round-trip failed for rate={}, freq={}: {} vs {}",
                periodic,
                freq,
                periodic,
                back_to_periodic
            );
        }
    }

    #[test]
    fn test_simple_periodic_round_trip() {
        let test_cases = vec![
            (0.05, 0.25, 4), // 5% over quarter, quarterly
            (0.05, 0.5, 2),  // 5% over half-year, semi-annual
            (0.05, 1.0, 2),  // 5% over year, semi-annual
            (0.10, 2.0, 12), // 10% over 2 years, monthly
        ];

        for (simple, yf, freq) in test_cases {
            let periodic = simple_to_periodic(simple, yf, freq).expect("conversion should succeed");
            let back = periodic_to_simple(periodic, yf, freq).expect("conversion should succeed");
            assert!(
                (simple - back).abs() < EPSILON,
                "Round-trip failed for simple={}, yf={}, freq={}: {} vs {}",
                simple,
                yf,
                freq,
                simple,
                back
            );
        }
    }

    #[test]
    fn test_simple_continuous_round_trip() {
        let test_cases = vec![(0.05, 0.25), (0.05, 0.5), (0.05, 1.0), (0.10, 2.0)];

        for (simple, yf) in test_cases {
            let continuous = simple_to_continuous(simple, yf).expect("conversion should succeed");
            let back = continuous_to_simple(continuous, yf).expect("conversion should succeed");
            assert!(
                (simple - back).abs() < EPSILON,
                "Round-trip failed for simple={}, yf={}: {} vs {}",
                simple,
                yf,
                simple,
                back
            );
        }
    }

    #[test]
    fn test_zero_year_fraction() {
        // Zero year fraction should return the original rate
        let result = simple_to_periodic(0.05, 0.0, 2).expect("conversion should succeed");
        assert!((result - 0.05).abs() < EPSILON);
    }

    #[test]
    fn test_zero_rate() {
        // Zero rate should remain zero through all conversions
        assert!(
            (simple_to_periodic(0.0, 1.0, 2).expect("conversion should succeed") - 0.0).abs()
                < EPSILON
        );
        assert!(
            (periodic_to_continuous(0.0, 2).expect("conversion should succeed") - 0.0).abs()
                < EPSILON
        );
        assert!(
            (continuous_to_periodic(0.0, 2).expect("conversion should succeed") - 0.0).abs()
                < EPSILON
        );
    }

    #[test]
    fn test_negative_rates() {
        // Negative rates should work (important for modern markets!)
        let negative_rate = -0.005; // -0.5%

        let continuous =
            periodic_to_continuous(negative_rate, 2).expect("conversion should succeed");
        let back = continuous_to_periodic(continuous, 2).expect("conversion should succeed");
        assert!((negative_rate - back).abs() < EPSILON);

        let simple = -0.01;
        let periodic = simple_to_periodic(simple, 1.0, 2).expect("conversion should succeed");
        let back_simple = periodic_to_simple(periodic, 1.0, 2).expect("conversion should succeed");
        assert!((simple - back_simple).abs() < EPSILON);
    }

    #[test]
    fn test_high_frequency_convergence() {
        // As frequency increases, periodic should converge to continuous
        let periodic_rate = 0.05;
        let continuous =
            periodic_to_continuous(periodic_rate, 2).expect("conversion should succeed");

        let frequencies = vec![4, 12, 52, 365];
        let mut prev_diff = f64::MAX;

        for freq in frequencies {
            let periodic =
                continuous_to_periodic(continuous, freq).expect("conversion should succeed");
            let continuous_back =
                periodic_to_continuous(periodic, freq).expect("conversion should succeed");
            let diff = (continuous - continuous_back).abs();

            // Each higher frequency should be closer to continuous
            assert!(diff < prev_diff || diff < 1e-10);
            prev_diff = diff;
        }
    }

    #[test]
    fn test_validation_errors() {
        // Zero periods per year
        assert!(simple_to_periodic(0.05, 1.0, 0).is_err());
        assert!(periodic_to_continuous(0.05, 0).is_err());
        assert!(continuous_to_periodic(0.05, 0).is_err());

        // Negative year fraction
        assert!(simple_to_periodic(0.05, -1.0, 2).is_err());
        assert!(periodic_to_simple(0.05, -1.0, 2).is_err());

        // Zero year fraction for functions that require positive
        assert!(periodic_to_simple(0.05, 0.0, 2).is_err());

        // Extremely negative rate that would cause negative discount factor
        assert!(simple_to_periodic(-10.0, 1.0, 2).is_err());
        assert!(periodic_to_continuous(-20.0, 2).is_err());
    }

    #[test]
    fn test_precision_high_rates() {
        // Test precision with higher rates (10%, 20%)
        let high_rates = vec![0.10, 0.20, 0.30];

        for rate in high_rates {
            let continuous = periodic_to_continuous(rate, 2).expect("conversion should succeed");
            let back = continuous_to_periodic(continuous, 2).expect("conversion should succeed");
            assert!(
                (rate - back).abs() < EPSILON,
                "High rate precision failed for {}: {} vs {}",
                rate,
                rate,
                back
            );
        }
    }

    #[test]
    fn test_cross_conversion_consistency() {
        // simple -> periodic -> continuous should equal simple -> continuous
        let simple = 0.05;
        let yf = 1.0;
        let freq = 2;

        let path1 = simple_to_continuous(simple, yf).expect("conversion should succeed");

        let periodic = simple_to_periodic(simple, yf, freq).expect("conversion should succeed");
        let path2 = periodic_to_continuous(periodic, freq).expect("conversion should succeed");

        // These should be very close but might differ slightly due to different computation paths
        assert!(
            (path1 - path2).abs() < LOOSE_EPSILON,
            "Cross-conversion inconsistent: {} vs {}",
            path1,
            path2
        );
    }

    #[test]
    fn test_market_realistic_scenarios() {
        // US Treasury: 2.5% semi-annual to continuous (for zero curve)
        let treasury_rate = 0.025;
        let continuous =
            periodic_to_continuous(treasury_rate, 2).expect("conversion should succeed");
        assert!((continuous - 0.024845039997).abs() < LOOSE_EPSILON);

        // LIBOR 3M: 3.5% simple to semi-annual (for swap pricing)
        let libor = 0.035;
        let yf = 0.25; // 3 months
        let swap_rate = simple_to_periodic(libor, yf, 2).expect("conversion should succeed");
        // For short periods, should be very close
        assert!((swap_rate - libor).abs() < 0.001);

        // Corporate bond: 5% annual to continuous
        let corp_annual = 0.05;
        let corp_continuous =
            periodic_to_continuous(corp_annual, 1).expect("conversion should succeed");
        assert!((corp_continuous - 0.048790164169).abs() < LOOSE_EPSILON);
    }
}
