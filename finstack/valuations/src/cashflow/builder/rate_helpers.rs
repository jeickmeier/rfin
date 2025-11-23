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
//! - Consistent floor/cap ordering
//!
//! ## Formulas
//!
//! ### Gearing Includes Spread (Default)
//! `rate = cap( max( all_in_floor, gearing * ( max(index, floor) + spread ) ) )`
//!
//! ### Gearing Excludes Spread (Affine Model)
//! `rate = cap( max( all_in_floor, (gearing * max(index, floor)) + spread ) )`

use finstack_core::dates::{Date, DayCountCtx};
use finstack_core::market_data::term_structures::ForwardCurve;
use finstack_core::market_data::MarketContext;
use finstack_core::Result;

/// Parameters for floating rate projection.
#[derive(Debug, Clone)]
pub struct FloatingRateParams {
    /// Spread over index in basis points.
    pub spread_bp: f64,

    /// Gearing multiplier (default: 1.0).
    pub gearing: f64,

    /// Whether gearing includes the spread (default: true).
    /// - `true`: `(Index + Spread) * Gearing`
    /// - `false`: `(Index * Gearing) + Spread`
    pub gearing_includes_spread: bool,

    /// Floor on index rate in basis points (applied to index component).
    pub index_floor_bp: Option<f64>,

    /// Cap on index rate in basis points (applied to index component).
    pub index_cap_bp: Option<f64>,

    /// Floor on all-in rate in basis points (Min Coupon).
    pub all_in_floor_bp: Option<f64>,

    /// Cap on all-in rate in basis points (Max Coupon).
    pub all_in_cap_bp: Option<f64>,
}

impl Default for FloatingRateParams {
    fn default() -> Self {
        Self {
            spread_bp: 0.0,
            gearing: 1.0,
            gearing_includes_spread: true,
            index_floor_bp: None,
            index_cap_bp: None,
            all_in_floor_bp: None,
            all_in_cap_bp: None,
        }
    }
}

/// Project floating rate with optional floor, cap, and gearing.
///
/// Delegates to `project_floating_rate_with_curve` using standard defaults.
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
    project_floating_rate_with_curve(
        reset_date,
        reset_period_end,
        spread_bp,
        gearing,
        floor_bp,
        cap_bp,
        fwd,
    )
}

/// Project floating rate using a resolved forward curve (legacy/simplified).
///
/// Uses default conventions:
/// - Gearing includes spread
/// - floor_bp is index floor
/// - cap_bp is all-in cap
#[allow(clippy::too_many_arguments)]
pub fn project_floating_rate_with_curve(
    reset_date: Date,
    reset_period_end: Date,
    spread_bp: f64,
    gearing: f64,
    floor_bp: Option<f64>,
    cap_bp: Option<f64>,
    fwd: &ForwardCurve,
) -> Result<f64> {
    let params = FloatingRateParams {
        spread_bp,
        gearing,
        gearing_includes_spread: true,
        index_floor_bp: floor_bp,
        index_cap_bp: None,
        all_in_floor_bp: None,
        all_in_cap_bp: cap_bp,
    };
    project_floating_rate_detailed(reset_date, reset_period_end, fwd, &params)
}

/// Project floating rate using full parameter set.
pub fn project_floating_rate_detailed(
    reset_date: Date,
    reset_period_end: Date,
    fwd: &ForwardCurve,
    params: &FloatingRateParams,
) -> Result<f64> {
    let fwd_dc = fwd.day_count();
    let fwd_base = fwd.base_date();

    // Compute time points for the accrual period
    let t0 = fwd_dc.year_fraction(fwd_base, reset_date, DayCountCtx::default())?;
    let t1 = fwd_dc.year_fraction(fwd_base, reset_period_end, DayCountCtx::default())?;

    // Get period forward rate
    let mut index_rate = fwd.rate_period(t0, t1);

    // Apply index floor/cap
    if let Some(floor) = params.index_floor_bp {
        index_rate = index_rate.max(floor * 1e-4);
    }
    if let Some(cap) = params.index_cap_bp {
        index_rate = index_rate.min(cap * 1e-4);
    }

    // Calculate rate based on gearing style
    let mut rate = if params.gearing_includes_spread {
        // (Index + Spread) * Gearing
        (index_rate + params.spread_bp * 1e-4) * params.gearing
    } else {
        // (Index * Gearing) + Spread
        (index_rate * params.gearing) + params.spread_bp * 1e-4
    };

    // Apply all-in floor/cap
    if let Some(floor) = params.all_in_floor_bp {
        rate = rate.max(floor * 1e-4);
    }
    if let Some(cap) = params.all_in_cap_bp {
        rate = rate.min(cap * 1e-4);
    }

    Ok(rate)
}

/// Simplified floating rate projection using tenor approximation.
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

/// Simplified floating rate projection using tenor approximation with resolved curve.
#[allow(clippy::too_many_arguments)]
pub fn project_floating_rate_simple_with_curve(
    reset_date: Date,
    tenor_years: f64,
    spread_bp: f64,
    gearing: f64,
    floor_bp: Option<f64>,
    cap_bp: Option<f64>,
    fwd: &ForwardCurve,
) -> Result<f64> {
    // Approximate period end from tenor
    let days = (tenor_years * 365.25) as i64;
    let period_end = reset_date + time::Duration::days(days);

    project_floating_rate_with_curve(
        reset_date, period_end, spread_bp, gearing, floor_bp, cap_bp, fwd,
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
            .expect("ForwardCurve builder should succeed with valid test data");
        MarketContext::new().insert_forward(fwd_curve)
    }

    #[test]
    fn test_project_floating_rate_no_floor_no_cap() {
        let reset = Date::from_calendar_date(2025, Month::January, 15).expect("Valid test date");
        let period_end = Date::from_calendar_date(2025, Month::April, 15).expect("Valid test date");
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
        .expect("Rate projection should succeed in test");

        // Should be ~3% index + 2% spread = ~5%
        assert!(rate > 0.04 && rate < 0.06, "Rate should be ~5%: {}", rate);
    }

    #[test]
    fn test_project_floating_rate_with_floor() {
        let reset = Date::from_calendar_date(2025, Month::January, 15).expect("Valid test date");
        let period_end = Date::from_calendar_date(2025, Month::April, 15).expect("Valid test date");

        // Create market with very low rates (below floor)
        let fwd_curve = ForwardCurve::builder("USD-LIBOR-3M", 0.25)
            .base_date(reset)
            .day_count(DayCount::Act360)
            .knots([(0.0, 0.001), (1.0, 0.001), (5.0, 0.001)]) // 0.1% < 1% floor
            .build()
            .expect("ForwardCurve builder should succeed with valid test data");
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
        .expect("Rate projection should succeed in test");

        // Floor lifts index to 1%, plus 1% spread = 2%
        assert!(
            (rate - 0.02).abs() < 0.001,
            "Rate should be ~2% (floor + spread): {}",
            rate
        );
    }

    #[test]
    fn test_project_floating_rate_with_cap() {
        let reset = Date::from_calendar_date(2025, Month::January, 15).expect("Valid test date");
        let period_end = Date::from_calendar_date(2025, Month::April, 15).expect("Valid test date");

        // Create market with high rates
        let fwd_curve = ForwardCurve::builder("USD-LIBOR-3M", 0.25)
            .base_date(reset)
            .day_count(DayCount::Act360)
            .knots([(0.0, 0.08), (1.0, 0.08), (5.0, 0.08)]) // 8% index
            .build()
            .expect("ForwardCurve builder should succeed with valid test data");
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
        .expect("Rate projection should succeed in test");

        // 8% index + 2% spread = 10%, capped at 5%
        assert!(
            (rate - 0.05).abs() < 0.001,
            "Rate should be capped at 5%: {}",
            rate
        );
    }

    #[test]
    fn test_floor_applied_before_spread() {
        let reset = Date::from_calendar_date(2025, Month::January, 15).expect("Valid test date");
        let period_end = Date::from_calendar_date(2025, Month::April, 15).expect("Valid test date");

        // Use very low rate (0.01% = 1 bp) which is below the floor
        let fwd_curve = ForwardCurve::builder("TEST-INDEX", 0.25)
            .base_date(reset)
            .day_count(DayCount::Act360)
            .knots([(0.0, 0.0001), (1.0, 0.0001)]) // 0.01% index (below 1% floor)
            .build()
            .expect("ForwardCurve builder should succeed with valid test data");
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
        .expect("Rate projection should succeed in test");

        // Floor lifts index from 0.01% to 1%, then add 1% spread = 2%
        assert!(
            (rate - 0.02).abs() < 0.001,
            "Rate should be 2% (floored index + spread): {}",
            rate
        );
    }

    #[test]
    fn test_cap_applied_after_gearing() {
        let reset = Date::from_calendar_date(2025, Month::January, 15).expect("Valid test date");
        let period_end = Date::from_calendar_date(2025, Month::April, 15).expect("Valid test date");

        let fwd_curve = ForwardCurve::builder("TEST-INDEX", 0.25)
            .base_date(reset)
            .day_count(DayCount::Act360)
            .knots([(0.0, 0.03), (1.0, 0.03)]) // 3% index
            .build()
            .expect("ForwardCurve builder should succeed with valid test data");
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
        .expect("Rate projection should succeed in test");

        // (3% index + 1% spread) * 2 = 8%, capped at 6%
        assert!(
            (rate - 0.06).abs() < 0.001,
            "Rate should be capped at 6% after gearing: {}",
            rate
        );
    }

    #[test]
    fn test_gearing_multiplies_all_in_rate() {
        let reset = Date::from_calendar_date(2025, Month::January, 15).expect("Valid test date");
        let period_end = Date::from_calendar_date(2025, Month::April, 15).expect("Valid test date");

        let fwd_curve = ForwardCurve::builder("TEST-INDEX", 0.25)
            .base_date(reset)
            .day_count(DayCount::Act360)
            .knots([(0.0, 0.02), (1.0, 0.02)]) // 2% index
            .build()
            .expect("ForwardCurve builder should succeed with valid test data");
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
        .expect("Rate projection should succeed in test");

        // (2% + 1%) * 1.5 = 4.5%
        assert!(
            (rate - 0.045).abs() < 0.001,
            "Rate should be 4.5% with gearing: {}",
            rate
        );
    }

    #[test]
    fn test_project_floating_rate_simple() {
        let reset = Date::from_calendar_date(2025, Month::January, 15).expect("Valid test date");
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
        .expect("Rate projection should succeed in test");

        // Should project forward rate + spread
        assert!(
            rate > 0.03 && rate < 0.06,
            "Rate should be reasonable: {}",
            rate
        );
    }
}
