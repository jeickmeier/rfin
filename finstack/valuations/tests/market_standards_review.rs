use finstack_core::dates::{Date, DayCount};
use finstack_core::market_data::term_structures::ForwardCurve;
use finstack_core::market_data::MarketContext;
use finstack_valuations::cashflow::builder::rate_helpers::{
    project_floating_rate, project_floating_rate_detailed, FloatingRateParams,
};
use time::Month;

fn create_test_market(base_date: Date, rate: f64) -> MarketContext {
    let fwd_curve = ForwardCurve::builder("USD-SOFR-3M", 0.25)
        .base_date(base_date)
        .day_count(DayCount::Act360)
        .knots([(0.0, rate), (10.0, rate)])
        .build()
        .expect("ForwardCurve builder");
    MarketContext::new().insert_forward(fwd_curve)
}

#[test]
fn test_market_standard_gearing_affine() {
    // Market Standard Check: Floating Rate Projection (Affine vs Standard)
    // Standard: (Index + Spread) * Gearing
    // Affine:   (Index * Gearing) + Spread

    let date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let end = Date::from_calendar_date(2025, Month::April, 1).unwrap();
    let market = create_test_market(date, 0.03); // 3% index

    let spread_bp = 100.0; // 1%
    let gearing = 2.0;

    // Case 1: Gearing includes spread (Default/Standard)
    // Expected: (3% + 1%) * 2 = 8%
    let rate_std = project_floating_rate(
        date,
        end,
        "USD-SOFR-3M",
        spread_bp,
        gearing,
        None,
        None,
        &market,
    )
    .unwrap();

    assert!(
        (rate_std - 0.08).abs() < 1e-6,
        "Standard gearing failed: got {}",
        rate_std
    );

    // Case 2: Affine model (Gearing excludes spread)
    // Expected: (3% * 2) + 1% = 7%
    // We need to use project_floating_rate_detailed to set the flag, as project_floating_rate uses defaults.

    let fwd = market.get_forward_ref("USD-SOFR-3M").unwrap();
    let params = FloatingRateParams {
        spread_bp,
        gearing,
        gearing_includes_spread: false, // Key flag
        index_floor_bp: None,
        index_cap_bp: None,
        all_in_floor_bp: None,
        all_in_cap_bp: None,
    };

    let rate_affine = project_floating_rate_detailed(date, end, fwd, &params).unwrap();
    assert!(
        (rate_affine - 0.07).abs() < 1e-6,
        "Affine gearing failed: got {}",
        rate_affine
    );
}

#[test]
fn test_market_standard_floor_application() {
    // Market Standard Check: Index Floor vs All-in Floor

    let date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let end = Date::from_calendar_date(2025, Month::April, 1).unwrap();
    let market = create_test_market(date, 0.01); // 1% index
    let fwd = market.get_forward_ref("USD-SOFR-3M").unwrap();

    // Case 1: Index Floor
    // Index = 1%, Floor = 2%. Effective Index = 2%.
    // Spread = 1%. Total = 3%.
    let params_idx = FloatingRateParams {
        spread_bp: 100.0,
        gearing: 1.0,
        gearing_includes_spread: true,
        index_floor_bp: Some(200.0),
        index_cap_bp: None,
        all_in_floor_bp: None,
        all_in_cap_bp: None,
    };
    let rate_idx = project_floating_rate_detailed(date, end, fwd, &params_idx).unwrap();
    assert!(
        (rate_idx - 0.03).abs() < 1e-6,
        "Index floor failed: got {}",
        rate_idx
    );

    // Case 2: All-in Floor
    // Index = 1%. Spread = 1%. Total = 2%.
    // All-in Floor = 2.5%. Result = 2.5%.
    let params_all_in = FloatingRateParams {
        spread_bp: 100.0,
        gearing: 1.0,
        gearing_includes_spread: true,
        index_floor_bp: None,
        index_cap_bp: None,
        all_in_floor_bp: Some(250.0),
        all_in_cap_bp: None,
    };
    let rate_all_in = project_floating_rate_detailed(date, end, fwd, &params_all_in).unwrap();
    assert!(
        (rate_all_in - 0.025).abs() < 1e-6,
        "All-in floor failed: got {}",
        rate_all_in
    );
}
