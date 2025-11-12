//! Centralized rate projection for floating rate instruments.
//!
//! Provides a single implementation of floating rate projection logic used across
//! all instruments: bonds, swaps, credit facilities, and structured products.
//!
//! ## Responsibilities
//!
//! - Project forward rates from market curves
//! - Apply floors and caps according to ISDA conventions
//! - Support gearing/leverage on rates
//! - Consistent floor/cap ordering: floor(index) → spread → gearing → cap(all-in)

use finstack_core::dates::{Date, DayCountCtx};
use finstack_core::market_data::MarketContext;
use finstack_core::Result;

/// Project floating rate with optional floor, cap, and gearing.
///
/// Standard pattern for floating rate instruments following ISDA conventions:
/// 1. Look up forward rate from market for the accrual period [reset_date, reset_period_end]
/// 2. Apply floor to index rate (if specified) - applied BEFORE adding spread
/// 3. Add spread/margin to index rate
/// 4. Multiply by gearing (typically 1.0)
/// 5. Apply cap to all-in rate (if specified) - applied AFTER spread and gearing
///
/// Formula: `cap(gearing * (floor(index_rate) + spread))`
///
/// # Arguments
///
/// * `reset_date` - Start of accrual period (rate fixing date)
/// * `reset_period_end` - End of accrual period
/// * `index_id` - Forward curve identifier (e.g., "USD-SOFR-3M", "USD-LIBOR-3M")
/// * `spread_bp` - Spread/margin over index in basis points
/// * `gearing` - Multiplier applied to rate (typically 1.0)
/// * `floor_bp` - Optional floor on index rate in basis points (applied before spread)
/// * `cap_bp` - Optional cap on all-in rate in basis points (applied after spread + gearing)
/// * `market` - Market context containing forward curves
///
/// # Returns
///
/// All-in coupon rate as decimal (e.g., 0.05 for 5%)
///
/// # Errors
///
/// Returns error if:
/// - Forward curve not found in market context
/// - Year fraction calculation fails
///
/// # Example
///
/// ```rust
/// use finstack_core::dates::Date;
/// use finstack_core::market_data::MarketContext;
/// use finstack_valuations::cashflow::builder::project_floating_rate;
/// use time::Month;
///
/// # fn example() -> finstack_core::Result<()> {
/// use finstack_core::dates::create_date;
/// let reset = create_date(2025, Month::January, 15)?;
/// let period_end = create_date(2025, Month::April, 15)?;
/// # let market = MarketContext::new();
///
/// // 3M SOFR + 200bps with 0% floor, no cap
/// let rate = project_floating_rate(
///     reset,
///     period_end,
///     "USD-SOFR-3M",
///     200.0,      // 200 bps spread
///     1.0,        // no gearing
///     Some(0.0),  // 0% floor
///     None,       // no cap
///     &market,
/// )?;
/// # Ok(())
/// # }
/// ```
#[allow(clippy::too_many_arguments)]
pub fn project_floating_rate(
    reset_date: Date,
    reset_period_end: Date,
    index_id: &str,
    spread_bp: f64,
    gearing: f64,
    floor_bp: Option<f64>,
    cap_bp: Option<f64>,
    market: &MarketContext,
) -> Result<f64> {
    // Get forward curve
    let fwd = market.get_forward_ref(index_id)?;
    let fwd_dc = fwd.day_count();
    let fwd_base = fwd.base_date();

    // Compute time points for the accrual period
    let t0 = fwd_dc.year_fraction(fwd_base, reset_date, DayCountCtx::default())?;
    let t1 = fwd_dc.year_fraction(fwd_base, reset_period_end, DayCountCtx::default())?;

    // Get period forward rate
    let mut index_rate = fwd.rate_period(t0, t1);

    // Apply floor to index (before adding spread)
    if let Some(floor) = floor_bp {
        index_rate = index_rate.max(floor * 1e-4);
    }

    // Add spread and apply gearing
    let mut all_in_rate = (index_rate + spread_bp * 1e-4) * gearing;

    // Apply cap to all-in rate (after spread and gearing)
    if let Some(cap) = cap_bp {
        all_in_rate = all_in_rate.min(cap * 1e-4);
    }

    Ok(all_in_rate)
}

/// Simplified floating rate projection using tenor approximation.
///
/// Convenience wrapper for `project_floating_rate` when reset period end
/// is not explicitly known. Approximates period end from reset date + tenor.
///
/// # Arguments
///
/// * `reset_date` - Rate fixing date
/// * `tenor_years` - Accrual period length in years (e.g., 0.25 for 3M)
/// * `index_id` - Forward curve identifier
/// * `spread_bp` - Spread in basis points
/// * `gearing` - Rate multiplier
/// * `floor_bp` - Optional floor in basis points
/// * `cap_bp` - Optional cap in basis points
/// * `market` - Market context
///
/// # Returns
///
/// All-in coupon rate as decimal
#[allow(clippy::too_many_arguments)]
pub fn project_floating_rate_simple(
    reset_date: Date,
    tenor_years: f64,
    index_id: &str,
    spread_bp: f64,
    gearing: f64,
    floor_bp: Option<f64>,
    cap_bp: Option<f64>,
    market: &MarketContext,
) -> Result<f64> {
    // Approximate period end from tenor
    let days = (tenor_years * 365.25) as i64;
    let period_end = reset_date + time::Duration::days(days);

    project_floating_rate(
        reset_date, period_end, index_id, spread_bp, gearing, floor_bp, cap_bp, market,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::dates::{Date, DayCount};
    use finstack_core::market_data::term_structures::ForwardCurve;
    use finstack_core::market_data::MarketContext;
    use time::Month;

    fn create_test_market(base_date: Date) -> MarketContext {
        let fwd_curve = ForwardCurve::builder("USD-SOFR-3M", 0.25)
            .base_date(base_date)
            .day_count(DayCount::Act360)
            .knots([(0.0, 0.03), (1.0, 0.035), (5.0, 0.04)])
            .build()
            .unwrap();
        MarketContext::new().insert_forward(fwd_curve)
    }

    #[test]
    fn test_project_floating_rate_no_floor_no_cap() {
        let reset = Date::from_calendar_date(2025, Month::January, 15).unwrap();
        let period_end = Date::from_calendar_date(2025, Month::April, 15).unwrap();
        let market = create_test_market(reset);

        let rate = project_floating_rate(
            reset,
            period_end,
            "USD-SOFR-3M",
            200.0, // 200 bps
            1.0,
            None,
            None,
            &market,
        )
        .unwrap();

        // Should be ~3% index + 2% spread = ~5%
        assert!(rate > 0.04 && rate < 0.06, "Rate should be ~5%: {}", rate);
    }

    #[test]
    fn test_project_floating_rate_with_floor() {
        let reset = Date::from_calendar_date(2025, Month::January, 15).unwrap();
        let period_end = Date::from_calendar_date(2025, Month::April, 15).unwrap();

        // Create market with very low rates (below floor)
        let fwd_curve = ForwardCurve::builder("USD-LIBOR-3M", 0.25)
            .base_date(reset)
            .day_count(DayCount::Act360)
            .knots([(0.0, 0.001), (1.0, 0.001), (5.0, 0.001)]) // 0.1% < 1% floor
            .build()
            .unwrap();
        let market = MarketContext::new().insert_forward(fwd_curve);

        let rate = project_floating_rate(
            reset,
            period_end,
            "USD-LIBOR-3M",
            100.0, // 100 bps spread
            1.0,
            Some(100.0), // 1% floor on index
            None,
            &market,
        )
        .unwrap();

        // Floor lifts index to 1%, plus 1% spread = 2%
        assert!(
            (rate - 0.02).abs() < 0.001,
            "Rate should be ~2% (floor + spread): {}",
            rate
        );
    }

    #[test]
    fn test_project_floating_rate_with_cap() {
        let reset = Date::from_calendar_date(2025, Month::January, 15).unwrap();
        let period_end = Date::from_calendar_date(2025, Month::April, 15).unwrap();

        // Create market with high rates
        let fwd_curve = ForwardCurve::builder("USD-LIBOR-3M", 0.25)
            .base_date(reset)
            .day_count(DayCount::Act360)
            .knots([(0.0, 0.08), (1.0, 0.08), (5.0, 0.08)]) // 8% index
            .build()
            .unwrap();
        let market = MarketContext::new().insert_forward(fwd_curve);

        let rate = project_floating_rate(
            reset,
            period_end,
            "USD-LIBOR-3M",
            200.0, // 200 bps spread
            1.0,
            None,
            Some(500.0), // 5% cap on all-in
            &market,
        )
        .unwrap();

        // 8% index + 2% spread = 10%, capped at 5%
        assert!(
            (rate - 0.05).abs() < 0.001,
            "Rate should be capped at 5%: {}",
            rate
        );
    }

    #[test]
    fn test_floor_applied_before_spread() {
        let reset = Date::from_calendar_date(2025, Month::January, 15).unwrap();
        let period_end = Date::from_calendar_date(2025, Month::April, 15).unwrap();

        // Use very low rate (0.01% = 1 bp) which is below the floor
        let fwd_curve = ForwardCurve::builder("TEST-INDEX", 0.25)
            .base_date(reset)
            .day_count(DayCount::Act360)
            .knots([(0.0, 0.0001), (1.0, 0.0001)]) // 0.01% index (below 1% floor)
            .build()
            .unwrap();
        let market = MarketContext::new().insert_forward(fwd_curve);

        let rate = project_floating_rate(
            reset,
            period_end,
            "TEST-INDEX",
            100.0, // 100 bps spread
            1.0,
            Some(100.0), // 1% floor on index (100 bps)
            None,
            &market,
        )
        .unwrap();

        // Floor lifts index from 0.01% to 1%, then add 1% spread = 2%
        assert!(
            (rate - 0.02).abs() < 0.001,
            "Rate should be 2% (floored index + spread): {}",
            rate
        );
    }

    #[test]
    fn test_cap_applied_after_gearing() {
        let reset = Date::from_calendar_date(2025, Month::January, 15).unwrap();
        let period_end = Date::from_calendar_date(2025, Month::April, 15).unwrap();

        let fwd_curve = ForwardCurve::builder("TEST-INDEX", 0.25)
            .base_date(reset)
            .day_count(DayCount::Act360)
            .knots([(0.0, 0.03), (1.0, 0.03)]) // 3% index
            .build()
            .unwrap();
        let market = MarketContext::new().insert_forward(fwd_curve);

        let rate = project_floating_rate(
            reset,
            period_end,
            "TEST-INDEX",
            100.0, // 100 bps spread
            2.0,   // 2x gearing
            None,
            Some(600.0), // 6% cap
            &market,
        )
        .unwrap();

        // (3% index + 1% spread) * 2 = 8%, capped at 6%
        assert!(
            (rate - 0.06).abs() < 0.001,
            "Rate should be capped at 6% after gearing: {}",
            rate
        );
    }

    #[test]
    fn test_gearing_multiplies_all_in_rate() {
        let reset = Date::from_calendar_date(2025, Month::January, 15).unwrap();
        let period_end = Date::from_calendar_date(2025, Month::April, 15).unwrap();

        let fwd_curve = ForwardCurve::builder("TEST-INDEX", 0.25)
            .base_date(reset)
            .day_count(DayCount::Act360)
            .knots([(0.0, 0.02), (1.0, 0.02)]) // 2% index
            .build()
            .unwrap();
        let market = MarketContext::new().insert_forward(fwd_curve);

        let rate = project_floating_rate(
            reset,
            period_end,
            "TEST-INDEX",
            100.0, // 100 bps spread
            1.5,   // 1.5x gearing
            None,
            None,
            &market,
        )
        .unwrap();

        // (2% + 1%) * 1.5 = 4.5%
        assert!(
            (rate - 0.045).abs() < 0.001,
            "Rate should be 4.5% with gearing: {}",
            rate
        );
    }

    #[test]
    fn test_project_floating_rate_simple() {
        let reset = Date::from_calendar_date(2025, Month::January, 15).unwrap();
        let market = create_test_market(reset);

        let rate = project_floating_rate_simple(
            reset,
            0.25, // 3 month tenor
            "USD-SOFR-3M",
            150.0, // 150 bps
            1.0,
            None,
            None,
            &market,
        )
        .unwrap();

        // Should project forward rate + spread
        assert!(
            rate > 0.03 && rate < 0.06,
            "Rate should be reasonable: {}",
            rate
        );
    }
}
