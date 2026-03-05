//! Tests for FloatingRateFallback policy wired into emission.
//!
//! Covers three fallback variants when no forward curve is available:
//! - `Error`: build should fail with an error
//! - `SpreadOnly`: build succeeds, rate == spread (legacy behavior)
//! - `FixedRate(r)`: build succeeds, rate == r + spread (through params pipeline)

use finstack_core::cashflow::CFKind;
use finstack_core::currency::Currency;
use finstack_core::dates::{BusinessDayConvention, DayCount, StubKind, Tenor};
use finstack_core::money::Money;
use finstack_valuations::cashflow::builder::specs::{
    CouponType, FloatingCouponSpec, FloatingRateFallback, FloatingRateSpec,
};
use finstack_valuations::cashflow::builder::CashFlowSchedule;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use time::Month;

type Date = finstack_core::dates::Date;

/// Build a minimal floating coupon spec with the given fallback policy and spread.
fn make_float_spec(fallback: FloatingRateFallback, spread_bp: Decimal) -> FloatingCouponSpec {
    FloatingCouponSpec {
        rate_spec: FloatingRateSpec {
            index_id: "USD-SOFR-3M".into(),
            spread_bp,
            gearing: Decimal::ONE,
            gearing_includes_spread: true,
            floor_bp: None,
            cap_bp: None,
            all_in_floor_bp: None,
            index_cap_bp: None,
            reset_freq: Tenor::quarterly(),
            reset_lag_days: 0,
            dc: DayCount::Act360,
            bdc: BusinessDayConvention::Following,
            calendar_id: "weekends_only".to_string(),
            fixing_calendar_id: None,
            end_of_month: false,
            payment_lag_days: 0,
            overnight_compounding: None,
            fallback,
        },
        coupon_type: CouponType::Cash,
        freq: Tenor::quarterly(),
        stub: StubKind::None,
    }
}

// =============================================================================
// Test 1: FloatingRateFallback::Error + no curve => Err
// =============================================================================

#[test]
fn test_floating_rate_fallback_error_no_curve() {
    let issue = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    let maturity = Date::from_calendar_date(2026, Month::January, 15).unwrap();
    let init = Money::new(1_000_000.0, Currency::USD);

    let spec = make_float_spec(FloatingRateFallback::Error, dec!(200.0));

    let mut b = CashFlowSchedule::builder();
    let _ = b.principal(init, issue, maturity).floating_cf(spec);

    // No market context => no forward curve => should error
    let result = b.build_with_curves(None);
    assert!(
        result.is_err(),
        "build_with_curves(None) should fail when fallback is Error"
    );
}

// =============================================================================
// Test 2: FloatingRateFallback::SpreadOnly + no curve => spread-only rate
// =============================================================================

#[test]
fn test_floating_rate_fallback_spread_only_no_curve() {
    let issue = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    let maturity = Date::from_calendar_date(2026, Month::January, 15).unwrap();
    let init = Money::new(1_000_000.0, Currency::USD);

    // 200 bps spread => 0.02 rate when index is 0
    let spec = make_float_spec(FloatingRateFallback::SpreadOnly, dec!(200.0));

    let mut b = CashFlowSchedule::builder();
    let _ = b.principal(init, issue, maturity).floating_cf(spec);

    let schedule = b
        .build_with_curves(None)
        .expect("SpreadOnly fallback should succeed without a curve");

    // Find all FloatReset flows
    let float_flows: Vec<_> = schedule
        .flows
        .iter()
        .filter(|cf| cf.kind == CFKind::FloatReset)
        .collect();

    assert!(
        !float_flows.is_empty(),
        "Should have at least one FloatReset flow"
    );

    // With gearing=1 and index=0, rate should equal spread = 200bp = 0.02
    for cf in &float_flows {
        let rate = cf.rate.expect("FloatReset should have a rate");
        assert!(
            (rate - 0.02).abs() < 1e-10,
            "Rate should be 0.02 (spread-only), got {}",
            rate
        );
    }
}

// =============================================================================
// Test 3: FloatingRateFallback::FixedRate(0.045) + no curve => 0.045 + spread
// =============================================================================

#[test]
fn test_floating_rate_fallback_fixed_rate() {
    let issue = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    let maturity = Date::from_calendar_date(2026, Month::January, 15).unwrap();
    let init = Money::new(1_000_000.0, Currency::USD);

    // 200 bps spread + fixed index of 4.5%
    // Expected all-in rate: (0.045 + 0.02) * 1.0 = 0.065
    let spec = make_float_spec(FloatingRateFallback::FixedRate(dec!(0.045)), dec!(200.0));

    let mut b = CashFlowSchedule::builder();
    let _ = b.principal(init, issue, maturity).floating_cf(spec);

    let schedule = b
        .build_with_curves(None)
        .expect("FixedRate fallback should succeed without a curve");

    // Find all FloatReset flows
    let float_flows: Vec<_> = schedule
        .flows
        .iter()
        .filter(|cf| cf.kind == CFKind::FloatReset)
        .collect();

    assert!(
        !float_flows.is_empty(),
        "Should have at least one FloatReset flow"
    );

    // With gearing=1, index=0.045, spread=200bp=0.02
    // rate = (0.045 + 0.02) * 1.0 = 0.065
    for cf in &float_flows {
        let rate = cf.rate.expect("FloatReset should have a rate");
        assert!(
            (rate - 0.065).abs() < 1e-10,
            "Rate should be 0.065 (fixed index 4.5% + 200bp spread), got {}",
            rate
        );
    }
}

// =============================================================================
// Test 4: FixedRate fallback respects floor/cap
// =============================================================================

#[test]
fn test_floating_rate_fallback_fixed_rate_with_floor_cap() {
    let issue = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    let maturity = Date::from_calendar_date(2026, Month::January, 15).unwrap();
    let init = Money::new(1_000_000.0, Currency::USD);

    // Fixed index of 4.5% + 200bp spread = 6.5%, but cap at 5%
    let mut spec = make_float_spec(FloatingRateFallback::FixedRate(dec!(0.045)), dec!(200.0));
    spec.rate_spec.cap_bp = Some(dec!(500.0)); // all-in cap at 5%

    let mut b = CashFlowSchedule::builder();
    let _ = b.principal(init, issue, maturity).floating_cf(spec);

    let schedule = b
        .build_with_curves(None)
        .expect("FixedRate fallback with cap should succeed");

    let float_flows: Vec<_> = schedule
        .flows
        .iter()
        .filter(|cf| cf.kind == CFKind::FloatReset)
        .collect();

    assert!(
        !float_flows.is_empty(),
        "Should have at least one FloatReset flow"
    );

    // Uncapped rate would be 0.065, but all-in cap = 5% = 0.05
    for cf in &float_flows {
        let rate = cf.rate.expect("FloatReset should have a rate");
        assert!(
            (rate - 0.05).abs() < 1e-10,
            "Rate should be capped at 0.05 (all-in cap 500bp), got {}",
            rate
        );
    }
}

// =============================================================================
// Test 5: Default fallback (Error) still works when curve IS present
// =============================================================================

#[test]
fn test_floating_rate_default_fallback_with_curve() {
    use finstack_core::market_data::context::MarketContext;
    use finstack_core::market_data::term_structures::ForwardCurve;

    let issue = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    let maturity = Date::from_calendar_date(2026, Month::January, 15).unwrap();
    let init = Money::new(1_000_000.0, Currency::USD);

    // Default fallback (Error) but with a curve present => should succeed
    let spec = make_float_spec(FloatingRateFallback::Error, dec!(200.0));

    let fwd = ForwardCurve::builder("USD-SOFR-3M", 0.25)
        .base_date(issue)
        .day_count(DayCount::Act360)
        .knots([(0.0, 0.03), (1.0, 0.035), (5.0, 0.04)])
        .build()
        .expect("ForwardCurve builder should succeed");
    let market = MarketContext::new().insert_forward(fwd);

    let mut b = CashFlowSchedule::builder();
    let _ = b.principal(init, issue, maturity).floating_cf(spec);

    let schedule = b
        .build_with_curves(Some(&market))
        .expect("Error fallback should succeed when curve is present");

    let float_flows: Vec<_> = schedule
        .flows
        .iter()
        .filter(|cf| cf.kind == CFKind::FloatReset)
        .collect();

    assert!(
        !float_flows.is_empty(),
        "Should have FloatReset flows when curve is present"
    );

    // Rate should be ~3% index + 2% spread = ~5%
    for cf in &float_flows {
        let rate = cf.rate.expect("FloatReset should have a rate");
        assert!(
            rate > 0.04 && rate < 0.06,
            "Rate should be ~5% (index + spread), got {}",
            rate
        );
    }
}
