//! Market data comparison and shift measurement.
//!
//! Provides utilities for measuring market movements between two `MarketContext`
//! instances. Used primarily for metrics-based P&L attribution, risk reporting,
//! and scenario analysis.
//!
//! # Purpose
//!
//! This module is symmetric to `bumps.rs`:
//! - **bumps.rs**: Apply shifts to create scenarios
//! - **diff.rs**: Measure shifts between markets
//!
//! # Use Cases
//!
//! - **P&L Attribution**: Explain P&L changes via DV01 × Δrates, CS01 × Δspreads
//! - **Risk Reporting**: Measure daily market moves for VaR and stress testing
//! - **Calibration**: Compare calibrated curves vs market inputs
//! - **Market Analysis**: Track curve movements over time
//!
//! # Examples
//!
//! ```rust
//! use finstack_core::market_data::diff::{measure_discount_curve_shift, TenorSamplingMethod};
//! use finstack_core::market_data::context::MarketContext;
//! use finstack_core::types::CurveId;
//!
//! # fn example(market_yesterday: MarketContext, market_today: MarketContext) -> finstack_core::Result<()> {
//! // Measure rate shift between two markets
//! let shift_bp = measure_discount_curve_shift(
//!     &CurveId::from("USD-OIS"),
//!     &market_yesterday,
//!     &market_today,
//!     TenorSamplingMethod::Standard,
//! )?;
//!
//! println!("USD-OIS moved {} basis points", shift_bp);
//! # Ok(())
//! # }
//! ```

use super::context::MarketContext;
use crate::currency::Currency;
use crate::Result;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

// -----------------------------------------------------------------------------
// Constants
// -----------------------------------------------------------------------------

/// Standard market tenors for curve sampling (in years).
///
/// Based on liquid swap market points: 3M, 6M, 1Y, 2Y, 3Y, 5Y, 7Y, 10Y, 30Y.
/// These are the conventional points where curves are most actively traded
/// and quoted.
pub const STANDARD_TENORS: &[f64] = &[0.25, 0.5, 1.0, 2.0, 3.0, 5.0, 7.0, 10.0, 30.0];

/// ATM reference strike multiplier (1.0 = 100% of spot).
pub const ATM_MONEYNESS: f64 = 1.0;

/// Default volatility surface expiry for sampling (1 year).
pub const DEFAULT_VOL_EXPIRY: f64 = 1.0;

// -----------------------------------------------------------------------------
// Tenor Sampling Method
// -----------------------------------------------------------------------------

/// Method for selecting tenor points when measuring curve shifts.
///
/// Different sampling strategies trade off accuracy, performance, and
/// robustness to curve structure.
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum TenorSamplingMethod {
    /// Standard swap market tenors (3M, 6M, 1Y, 2Y, 3Y, 5Y, 7Y, 10Y, 30Y).
    ///
    /// Most robust for typical interest rate curves. Matches market liquidity
    /// points and works well for parallel shift detection.
    #[default]
    Standard,

    /// Use curve's own knot points dynamically.
    ///
    /// Adapts to curve structure but may miss shifts between pillars.
    /// Good for curves with non-standard pillar structure.
    Dynamic,

    /// Custom tenor list specified by caller.
    ///
    /// Allows fine-grained control for specific use cases (e.g., matching
    /// an instrument's cashflow schedule).
    Custom(Vec<f64>),
}


impl TenorSamplingMethod {
    /// Get the tenor points to sample based on the method.
    ///
    /// For `Dynamic`, uses the knot points from the provided curve.
    /// For `Standard`, uses `STANDARD_TENORS`.
    /// For `Custom`, uses the provided tenor list.
    fn tenors<'a>(&'a self, curve_knots: Option<&'a [f64]>) -> &'a [f64] {
        match self {
            Self::Standard => STANDARD_TENORS,
            Self::Dynamic => curve_knots.unwrap_or(STANDARD_TENORS),
            Self::Custom(tenors) => tenors.as_slice(),
        }
    }
}

// -----------------------------------------------------------------------------
// Curve Shift Measurements
// -----------------------------------------------------------------------------

/// Measure average parallel rate shift in discount curve (basis points).
///
/// Samples the curve at specified tenors, computes zero rates at each point
/// for both T₀ and T₁, and returns the average shift in basis points.
///
/// # Arguments
///
/// * `curve_id` - Discount curve identifier to compare
/// * `market_t0` - Market context at T₀
/// * `market_t1` - Market context at T₁
/// * `method` - Tenor sampling strategy
///
/// # Returns
///
/// Average parallel shift in basis points. Positive means rates increased.
///
/// # Errors
///
/// Returns error if:
/// - Curve not found in either market
/// - Curves have incompatible structure
///
/// # Examples
///
/// ```rust
/// # use finstack_core::market_data::diff::{measure_discount_curve_shift, TenorSamplingMethod};
/// # use finstack_core::market_data::context::MarketContext;
/// # use finstack_core::types::CurveId;
/// # fn example(market_t0: MarketContext, market_t1: MarketContext) -> finstack_core::Result<()> {
/// let shift = measure_discount_curve_shift(
///     &CurveId::from("USD-OIS"),
///     &market_t0,
///     &market_t1,
///     TenorSamplingMethod::Standard,
/// )?;
///
/// println!("Rates moved {} bps", shift);
/// # Ok(())
/// # }
/// ```
pub fn measure_discount_curve_shift(
    curve_id: impl AsRef<str>,
    market_t0: &MarketContext,
    market_t1: &MarketContext,
    method: TenorSamplingMethod,
) -> Result<f64> {
    // Extract curves from both markets
    let curve_t0 = market_t0.get_discount(&curve_id)?;
    let curve_t1 = market_t1.get_discount(&curve_id)?;

    // Get tenor points to sample
    let tenors = method.tenors(Some(curve_t0.knots()));

    // Sample zero rates at each tenor
    let mut total_shift = 0.0;
    let mut sample_count = 0;

    for &tenor in tenors {
        if tenor <= 0.0 {
            continue; // Skip non-positive tenors
        }

        let zero_t0 = curve_t0.zero(tenor);
        let zero_t1 = curve_t1.zero(tenor);

        // Shift in basis points: (rate_t1 - rate_t0) * 10000
        let shift_bp = (zero_t1 - zero_t0) * 10_000.0;

        total_shift += shift_bp;
        sample_count += 1;
    }

    if sample_count == 0 {
        return Ok(0.0);
    }

    // Return average shift
    Ok(total_shift / sample_count as f64)
}

/// Measure bucketed rate shifts for detailed attribution.
///
/// Returns shift at each specified tenor, useful for bucketed DV01 attribution.
///
/// # Arguments
///
/// * `curve_id` - Discount curve identifier
/// * `market_t0` - Market context at T₀
/// * `market_t1` - Market context at T₁
/// * `tenors` - Tenor points to measure (in years)
///
/// # Returns
///
/// Vector of (tenor, shift_bp) pairs for each tenor point.
///
/// # Errors
///
/// Returns error if curve not found in either market.
pub fn measure_bucketed_discount_shift(
    curve_id: impl AsRef<str>,
    market_t0: &MarketContext,
    market_t1: &MarketContext,
    tenors: &[f64],
) -> Result<Vec<(f64, f64)>> {
    let curve_t0 = market_t0.get_discount(&curve_id)?;
    let curve_t1 = market_t1.get_discount(&curve_id)?;

    let mut shifts = Vec::with_capacity(tenors.len());

    for &tenor in tenors {
        if tenor <= 0.0 {
            continue;
        }

        let zero_t0 = curve_t0.zero(tenor);
        let zero_t1 = curve_t1.zero(tenor);
        let shift_bp = (zero_t1 - zero_t0) * 10_000.0;

        shifts.push((tenor, shift_bp));
    }

    Ok(shifts)
}

/// Measure average parallel spread shift in hazard curve (basis points).
///
/// Similar to discount curve shifts, but measures credit spread movements.
///
/// # Arguments
///
/// * `curve_id` - Hazard curve identifier
/// * `market_t0` - Market context at T₀
/// * `market_t1` - Market context at T₁
/// * `method` - Tenor sampling strategy
///
/// # Returns
///
/// Average spread shift in basis points. Positive means spreads widened.
///
/// # Errors
///
/// Returns error if curve not found in either market.
pub fn measure_hazard_curve_shift(
    curve_id: impl AsRef<str>,
    market_t0: &MarketContext,
    market_t1: &MarketContext,
    _method: TenorSamplingMethod,
) -> Result<f64> {
    let curve_t0 = market_t0.get_hazard(&curve_id)?;
    let curve_t1 = market_t1.get_hazard(&curve_id)?;

    // Collect knot points (time, lambda) from both curves
    let knots_t0: Vec<(f64, f64)> = curve_t0.knot_points().collect();
    let knots_t1: Vec<(f64, f64)> = curve_t1.knot_points().collect();

    if knots_t0.is_empty() || knots_t1.is_empty() {
        return Ok(0.0);
    }

    // Compare lambda (hazard intensity) values at common tenors
    let mut total_shift = 0.0;
    let mut sample_count = 0;

    for (t0, lambda0) in &knots_t0 {
        // Find closest point in t1
        if let Some((_, lambda1)) = knots_t1.iter().find(|(t1, _)| (t1 - t0).abs() < 0.01) {
            // Shift in basis points
            let shift_bp = (lambda1 - lambda0) * 10_000.0;
            total_shift += shift_bp;
            sample_count += 1;
        }
    }

    if sample_count == 0 {
        // If no matching tenors, compare first knot
        let shift_bp = (knots_t1[0].1 - knots_t0[0].1) * 10_000.0;
        return Ok(shift_bp);
    }

    Ok(total_shift / sample_count as f64)
}

/// Measure average inflation rate shift (basis points).
///
/// Measures the change in implied inflation rates between two inflation curves.
///
/// # Arguments
///
/// * `curve_id` - Inflation curve identifier
/// * `market_t0` - Market context at T₀
/// * `market_t1` - Market context at T₁
///
/// # Returns
///
/// Average inflation rate shift in basis points.
///
/// # Errors
///
/// Returns error if curve not found in either market.
pub fn measure_inflation_curve_shift(
    curve_id: impl AsRef<str>,
    market_t0: &MarketContext,
    market_t1: &MarketContext,
) -> Result<f64> {
    let curve_t0 = market_t0.get_inflation(&curve_id)?;
    let curve_t1 = market_t1.get_inflation(&curve_id)?;

    // Sample at standard tenors
    let mut total_shift = 0.0;
    let mut sample_count = 0;

    for &tenor in STANDARD_TENORS {
        if tenor <= 0.0 {
            continue;
        }

        // Inflation rate = (CPI(t) / CPI(0))^(1/t) - 1
        let cpi_t0 = curve_t0.cpi(tenor);
        let cpi_t1 = curve_t1.cpi(tenor);

        let base_cpi_t0 = curve_t0.base_cpi();
        let base_cpi_t1 = curve_t1.base_cpi();

        let infl_rate_t0 = (cpi_t0 / base_cpi_t0).powf(1.0 / tenor) - 1.0;
        let infl_rate_t1 = (cpi_t1 / base_cpi_t1).powf(1.0 / tenor) - 1.0;

        let shift_bp = (infl_rate_t1 - infl_rate_t0) * 10_000.0;

        total_shift += shift_bp;
        sample_count += 1;
    }

    if sample_count == 0 {
        return Ok(0.0);
    }

    Ok(total_shift / sample_count as f64)
}

/// Measure average correlation shift (percentage points).
///
/// Measures change in base correlation levels between two markets.
///
/// # Arguments
///
/// * `curve_id` - Base correlation curve identifier
/// * `market_t0` - Market context at T₀
/// * `market_t1` - Market context at T₁
///
/// # Returns
///
/// Average correlation shift in percentage points (e.g., 5.0 = +5% correlation).
///
/// # Errors
///
/// Returns error if curve not found in either market.
pub fn measure_correlation_shift(
    curve_id: impl AsRef<str>,
    market_t0: &MarketContext,
    market_t1: &MarketContext,
) -> Result<f64> {
    let curve_t0 = market_t0.get_base_correlation(&curve_id)?;
    let curve_t1 = market_t1.get_base_correlation(&curve_id)?;

    // Sample at curve detachment points
    let knots_t0 = &curve_t0.detachment_points;

    let mut total_shift = 0.0;
    let mut sample_count = 0;

    for &attachment in knots_t0 {
        let corr_t0 = curve_t0.correlation(attachment);
        let corr_t1 = curve_t1.correlation(attachment);

        // Shift in percentage points
        let shift_pct = (corr_t1 - corr_t0) * 100.0;

        total_shift += shift_pct;
        sample_count += 1;
    }

    if sample_count == 0 {
        return Ok(0.0);
    }

    Ok(total_shift / sample_count as f64)
}

// -----------------------------------------------------------------------------
// Surface Shift Measurements
// -----------------------------------------------------------------------------

/// Measure volatility surface shift (percentage points).
///
/// Measures the change in implied volatility levels. Can measure at a specific
/// point or sample across the surface for average shift.
///
/// # Arguments
///
/// * `surface_id` - Volatility surface identifier
/// * `market_t0` - Market context at T₀
/// * `market_t1` - Market context at T₁
/// * `reference_expiry` - Optional expiry to measure (defaults to 1Y ATM)
/// * `reference_strike` - Optional strike to measure (defaults to ATM)
///
/// # Returns
///
/// Average volatility shift in percentage points (e.g., 2.0 = +2% vol).
///
/// # Errors
///
/// Returns error if surface not found in either market.
///
/// # Examples
///
/// ```rust
/// # use finstack_core::market_data::diff::measure_vol_surface_shift;
/// # use finstack_core::market_data::context::MarketContext;
/// # use finstack_core::types::CurveId;
/// # fn example(market_t0: MarketContext, market_t1: MarketContext) -> finstack_core::Result<()> {
/// // Measure 1Y ATM vol shift
/// let vol_shift = measure_vol_surface_shift(
///     &CurveId::from("SPX-VOL"),
///     &market_t0,
///     &market_t1,
///     Some(1.0),  // 1Y expiry
///     Some(1.0),  // ATM (100%)
/// )?;
/// # Ok(())
/// # }
/// ```
pub fn measure_vol_surface_shift(
    surface_id: impl AsRef<str>,
    market_t0: &MarketContext,
    market_t1: &MarketContext,
    reference_expiry: Option<f64>,
    reference_strike: Option<f64>,
) -> Result<f64> {
    let surface_t0 = market_t0.surface(&surface_id)?;
    let surface_t1 = market_t1.surface(&surface_id)?;

    // If specific point requested, measure there
    if let (Some(expiry), Some(strike)) = (reference_expiry, reference_strike) {
        let vol_t0 = surface_t0.value(expiry, strike);
        let vol_t1 = surface_t1.value(expiry, strike);

        // Shift in percentage points
        return Ok((vol_t1 - vol_t0) * 100.0);
    }

    // Otherwise, sample across surface at standard points
    let expiries = surface_t0.expiries();
    let strikes = surface_t0.strikes();

    let mut total_shift = 0.0;
    let mut sample_count = 0;

    // Sample at available expiries and middle strike (approximately ATM)
    for &expiry in expiries {
        if expiry <= 0.0 {
            continue;
        }

        // Use middle strike as ATM approximation
        if strikes.is_empty() {
            continue;
        }
        let mid_idx = strikes.len() / 2;
        let strike = strikes[mid_idx];

        let vol_t0 = surface_t0.value(expiry, strike);
        let vol_t1 = surface_t1.value(expiry, strike);

        let shift_pct = (vol_t1 - vol_t0) * 100.0;

        total_shift += shift_pct;
        sample_count += 1;
    }

    if sample_count == 0 {
        return Ok(0.0);
    }

    Ok(total_shift / sample_count as f64)
}

// -----------------------------------------------------------------------------
// FX and Scalar Shift Measurements
// -----------------------------------------------------------------------------

/// Measure FX spot rate shift (percentage change).
///
/// Calculates the percentage change in FX rate between two markets.
///
/// # Arguments
///
/// * `base_ccy` - Base currency
/// * `quote_ccy` - Quote currency
/// * `market_t0` - Market context at T₀
/// * `market_t1` - Market context at T₁
///
/// # Returns
///
/// Percentage change in FX rate: (rate_t1 / rate_t0 - 1) * 100
///
/// # Errors
///
/// Returns error if FX matrix not available in either market or conversion fails.
///
/// # Examples
///
/// ```rust
/// # use finstack_core::market_data::diff::measure_fx_shift;
/// # use finstack_core::market_data::context::MarketContext;
/// # use finstack_core::currency::Currency;
/// # fn example(market_t0: MarketContext, market_t1: MarketContext) -> finstack_core::Result<()> {
/// let fx_shift = measure_fx_shift(
///     Currency::USD,
///     Currency::EUR,
///     &market_t0,
///     &market_t1,
/// )?;
///
/// println!("USD/EUR moved {}%", fx_shift);
/// # Ok(())
/// # }
/// ```
pub fn measure_fx_shift(
    base_ccy: Currency,
    quote_ccy: Currency,
    market_t0: &MarketContext,
    market_t1: &MarketContext,
) -> Result<f64> {
    use crate::money::fx::FxQuery;

    // Get FX matrices
    let fx_t0 = market_t0
        .fx
        .as_ref()
        .ok_or_else(|| crate::error::InputError::NotFound {
            id: "FX_MATRIX".to_string(),
        })?;

    let fx_t1 = market_t1
        .fx
        .as_ref()
        .ok_or_else(|| crate::error::InputError::NotFound {
            id: "FX_MATRIX".to_string(),
        })?;

    // Use a reference date (today) for the query
    let ref_date = crate::dates::Date::from_calendar_date(2025, time::Month::January, 1)
        .expect("Valid reference date");

    // Get rates using FxQuery
    let query = FxQuery::new(base_ccy, quote_ccy, ref_date);
    let rate_t0 = fx_t0.rate(query)?.rate;
    let rate_t1 = fx_t1.rate(query)?.rate;

    // Percentage change
    let pct_change = (rate_t1 / rate_t0 - 1.0) * 100.0;

    Ok(pct_change)
}

/// Measure market scalar shift (percentage change).
///
/// Measures the change in market scalar values (equity prices, commodities, etc.).
///
/// # Arguments
///
/// * `scalar_id` - Market scalar identifier
/// * `market_t0` - Market context at T₀
/// * `market_t1` - Market context at T₁
///
/// # Returns
///
/// Percentage change in scalar value.
///
/// # Errors
///
/// Returns error if scalar not found in either market.
pub fn measure_scalar_shift(
    scalar_id: impl AsRef<str>,
    market_t0: &MarketContext,
    market_t1: &MarketContext,
) -> Result<f64> {
    use crate::market_data::scalars::MarketScalar;

    let scalar_t0 = market_t0.price(&scalar_id)?;
    let scalar_t1 = market_t1.price(&scalar_id)?;

    // Extract numeric values from enum
    let value_t0 = match scalar_t0 {
        MarketScalar::Unitless(v) => *v,
        MarketScalar::Price(m) => m.amount(),
    };

    let value_t1 = match scalar_t1 {
        MarketScalar::Unitless(v) => *v,
        MarketScalar::Price(m) => m.amount(),
    };

    // Percentage change
    let pct_change = (value_t1 / value_t0 - 1.0) * 100.0;

    Ok(pct_change)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dates::Date;
    use crate::market_data::term_structures::discount_curve::DiscountCurve;
    use crate::market_data::term_structures::hazard_curve::HazardCurve;
    use crate::math::interp::InterpStyle;
    use time::Month;

    fn sample_date() -> Date {
        Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date")
    }

    #[test]
    fn test_parallel_discount_shift() {
        let base_date = sample_date();

        // Create base curve (4% rates approximately)
        let curve_t0 = DiscountCurve::builder("USD-OIS")
            .base_date(base_date)
            .knots([(0.0, 1.0), (1.0, 0.96), (5.0, 0.82), (10.0, 0.67)])
            .set_interp(InterpStyle::LogLinear)
            .build()
            .expect("Market diff calculation should succeed in test");

        // Create curve with higher rates (~4.5%, which is ~50bp higher)
        // DF(t) = exp(-r*t), so for +50bp: DF_new = DF_old * exp(-0.005*t)
        let curve_t1 = DiscountCurve::builder("USD-OIS")
            .base_date(base_date)
            .knots([
                (0.0, 1.0),
                (1.0, 0.96 * (-0.005_f64 * 1.0).exp()),
                (5.0, 0.82 * (-0.005_f64 * 5.0).exp()),
                (10.0, 0.67 * (-0.005_f64 * 10.0).exp()),
            ])
            .set_interp(InterpStyle::LogLinear)
            .build()
            .expect("Market diff calculation should succeed in test");

        let market_t0 = MarketContext::new().insert_discount(curve_t0);
        let market_t1 = MarketContext::new().insert_discount(curve_t1);

        let shift = measure_discount_curve_shift(
            "USD-OIS",
            &market_t0,
            &market_t1,
            TenorSamplingMethod::Standard,
        )
        .expect("Market diff calculation should succeed in test");

        // Should detect approximately 50bp shift (within tolerance for sampling/interpolation)
        assert!((shift - 50.0).abs() < 5.0, "Expected ~50bp, got {}", shift);
    }

    #[test]
    fn test_hazard_curve_shift() {
        let base_date = sample_date();

        let curve_t0 = HazardCurve::builder("CORP-01")
            .base_date(base_date)
            .recovery_rate(0.4)
            .knots(vec![(1.0, 0.01), (5.0, 0.02), (10.0, 0.025)])
            .build()
            .expect("Market diff calculation should succeed in test");

        // Create a shifted curve (+25bp = 0.0025)
        let curve_t1 = HazardCurve::builder("CORP-01")
            .base_date(base_date)
            .recovery_rate(0.4)
            .knots(vec![(1.0, 0.0125), (5.0, 0.0225), (10.0, 0.0275)])
            .build()
            .expect("Market diff calculation should succeed in test");

        let market_t0 = MarketContext::new().insert_hazard(curve_t0);
        let market_t1 = MarketContext::new().insert_hazard(curve_t1);

        let shift = measure_hazard_curve_shift(
            "CORP-01",
            &market_t0,
            &market_t1,
            TenorSamplingMethod::Standard,
        )
        .expect("Market diff calculation should succeed in test");

        assert!((shift - 25.0).abs() < 1.0, "Expected ~25bp, got {}", shift);
    }

    #[test]
    fn test_missing_curve_error() {
        let market_t0 = MarketContext::new();
        let market_t1 = MarketContext::new();

        let result = measure_discount_curve_shift(
            "MISSING",
            &market_t0,
            &market_t1,
            TenorSamplingMethod::Standard,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_bucketed_shifts() {
        let base_date = sample_date();

        // Create base curve
        let curve_t0 = DiscountCurve::builder("USD-OIS")
            .base_date(base_date)
            .knots([(0.0, 1.0), (1.0, 0.96), (5.0, 0.82), (10.0, 0.67)])
            .set_interp(InterpStyle::LogLinear)
            .build()
            .expect("Market diff calculation should succeed in test");

        // Create curve with higher rates (+50bp)
        let curve_t1 = DiscountCurve::builder("USD-OIS")
            .base_date(base_date)
            .knots([
                (0.0, 1.0),
                (1.0, 0.96 * (-0.005_f64 * 1.0).exp()),
                (5.0, 0.82 * (-0.005_f64 * 5.0).exp()),
                (10.0, 0.67 * (-0.005_f64 * 10.0).exp()),
            ])
            .set_interp(InterpStyle::LogLinear)
            .build()
            .expect("Market diff calculation should succeed in test");

        let market_t0 = MarketContext::new().insert_discount(curve_t0);
        let market_t1 = MarketContext::new().insert_discount(curve_t1);

        let tenors = vec![1.0, 5.0, 10.0];
        let shifts = measure_bucketed_discount_shift("USD-OIS", &market_t0, &market_t1, &tenors)
            .expect("Bucketed discount shift calculation should succeed in test");

        assert_eq!(shifts.len(), 3);

        for (_tenor, shift_bp) in shifts {
            assert!((shift_bp - 50.0).abs() < 1.0);
        }
    }

    #[test]
    fn test_tenor_sampling_methods() {
        // Test that all sampling methods produce results
        let base_date = sample_date();

        let curve = DiscountCurve::builder("USD-OIS")
            .base_date(base_date)
            .knots([(0.0, 1.0), (1.0, 0.98), (5.0, 0.90)])
            .set_interp(InterpStyle::Linear)
            .build()
            .expect("Market diff calculation should succeed in test");

        let market = MarketContext::new().insert_discount(curve);

        // Standard
        let shift_std = measure_discount_curve_shift(
            "USD-OIS",
            &market,
            &market,
            TenorSamplingMethod::Standard,
        )
        .expect("Market diff calculation should succeed in test");
        assert_eq!(shift_std, 0.0); // Same market → zero shift

        // Dynamic
        let shift_dyn =
            measure_discount_curve_shift("USD-OIS", &market, &market, TenorSamplingMethod::Dynamic)
                .expect("Market diff calculation should succeed in test");
        assert_eq!(shift_dyn, 0.0);

        // Custom
        let shift_custom = measure_discount_curve_shift(
            "USD-OIS",
            &market,
            &market,
            TenorSamplingMethod::Custom(vec![1.0, 2.0, 3.0]),
        )
        .expect("Market diff calculation should succeed in test");
        assert_eq!(shift_custom, 0.0);
    }
}
