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

// =============================================================================
// Test 6: PIK flows carry rate and accrual_factor from parent coupon
// =============================================================================

/// PIK flows should carry rate and accrual_factor from the parent coupon.
#[test]
fn test_pik_flow_metadata() {
    // Build a 100% PIK floating rate bond with SpreadOnly fallback (no curve needed).
    // With CouponType::PIK, the full coupon goes to PIK flows.
    let issue = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    let maturity = Date::from_calendar_date(2026, Month::January, 15).unwrap();
    let init = Money::new(1_000_000.0, Currency::USD);

    // 200 bps spread => 0.02 rate when index is 0 (SpreadOnly)
    let mut spec = make_float_spec(FloatingRateFallback::SpreadOnly, dec!(200.0));
    spec.coupon_type = CouponType::PIK;

    let mut b = CashFlowSchedule::builder();
    let _ = b.principal(init, issue, maturity).floating_cf(spec);

    let schedule = b
        .build_with_curves(None)
        .expect("PIK with SpreadOnly fallback should succeed without a curve");

    // Find all PIK flows
    let pik_flows: Vec<_> = schedule
        .flows
        .iter()
        .filter(|cf| cf.kind == CFKind::PIK)
        .collect();

    assert!(
        !pik_flows.is_empty(),
        "Should have at least one PIK flow for a 100% PIK coupon"
    );

    // Verify that all PIK flows carry rate and accrual_factor from parent coupon
    for cf in &pik_flows {
        let rate = cf
            .rate
            .expect("PIK flow should carry rate from parent coupon");
        assert!(
            (rate - 0.02).abs() < 1e-10,
            "PIK flow rate should be 0.02 (spread-only), got {}",
            rate
        );
        assert!(
            cf.accrual_factor > 0.0,
            "PIK flow accrual_factor should be > 0.0, got {}",
            cf.accrual_factor
        );
    }

    // Also verify there are no FloatReset flows (100% PIK means no cash coupons)
    let float_flows: Vec<_> = schedule
        .flows
        .iter()
        .filter(|cf| cf.kind == CFKind::FloatReset)
        .collect();
    assert!(
        float_flows.is_empty(),
        "100% PIK coupon should have no FloatReset (cash) flows"
    );
}

// =============================================================================
// Golden Value Tests
// =============================================================================

const RATE_TOLERANCE: f64 = 1e-10;

/// Helper: create a flat forward curve at `flat_rate` for "USD-SOFR-3M" with
/// base date `base` and Act/360.
fn make_flat_forward_market(
    base: Date,
    flat_rate: f64,
) -> finstack_core::market_data::context::MarketContext {
    use finstack_core::market_data::context::MarketContext;
    use finstack_core::market_data::term_structures::ForwardCurve;

    let fwd = ForwardCurve::builder("USD-SOFR-3M", 0.25)
        .base_date(base)
        .day_count(DayCount::Act360)
        .knots([(0.0, flat_rate), (5.0, flat_rate)])
        .build()
        .expect("Flat ForwardCurve builder should succeed");
    MarketContext::new().insert_forward(fwd)
}

// =============================================================================
// Golden Value Test 1: SOFR + 200bp flat curve
// =============================================================================

/// Golden value: SOFR + 200bp, quarterly, Act/360, $1M notional.
/// Flat forward curve at 4.5%.
/// All-in rate = 4.5% + 2.0% = 6.5%
/// Each quarterly coupon ~ $1M x 0.065 x (days/360)
#[test]
fn test_floating_rate_golden_sofr_200bp() {
    let issue = Date::from_calendar_date(2024, Month::January, 15).unwrap();
    let maturity = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    let notional = 1_000_000.0;
    let init = Money::new(notional, Currency::USD);

    let spec = make_float_spec(FloatingRateFallback::Error, dec!(200.0));
    let market = make_flat_forward_market(issue, 0.045);

    let mut b = CashFlowSchedule::builder();
    let _ = b.principal(init, issue, maturity).floating_cf(spec);

    let schedule = b
        .build_with_curves(Some(&market))
        .expect("Golden SOFR+200bp build should succeed with flat curve");

    let float_flows: Vec<_> = schedule
        .flows
        .iter()
        .filter(|cf| cf.kind == CFKind::FloatReset)
        .collect();

    // Expect 4 quarterly FloatReset flows for a 1-year bond
    assert_eq!(
        float_flows.len(),
        4,
        "Expected 4 quarterly FloatReset flows, got {}",
        float_flows.len()
    );

    let expected_rate = 0.065; // 4.5% index + 2.0% spread

    for (i, cf) in float_flows.iter().enumerate() {
        let rate = cf.rate.expect("FloatReset should have a rate");
        assert!(
            (rate - expected_rate).abs() < RATE_TOLERANCE,
            "Flow {}: rate should be {} (SOFR 4.5% + 200bp), got {}",
            i,
            expected_rate,
            rate
        );

        // Coupon amount = notional * rate * accrual_factor
        // For quarterly Act/360, accrual_factor ~ days_in_period / 360
        // Each quarter has ~90-92 days, so accrual ~ 0.25
        // Expected coupon ~ 1_000_000 * 0.065 * 0.25 ~ $16,250
        let amount = cf.amount.amount().abs();
        assert!(
            amount > 15_000.0 && amount < 18_500.0,
            "Flow {}: coupon amount should be ~$16,250 (within bounds), got {:.2}",
            i,
            amount
        );

        // Verify the amount is consistent with rate * notional * accrual_factor
        let expected_amount = notional * expected_rate * cf.accrual_factor;
        assert!(
            (amount - expected_amount).abs() < 1.0,
            "Flow {}: amount {:.2} should match rate * notional * accrual ({:.2})",
            i,
            amount,
            expected_amount
        );
    }
}

// =============================================================================
// Golden Value Test 2: Zero spread (index only)
// =============================================================================

/// Golden value: SOFR + 0bp. Rate should equal index rate (4.5%).
#[test]
fn test_floating_rate_golden_zero_spread() {
    let issue = Date::from_calendar_date(2024, Month::January, 15).unwrap();
    let maturity = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    let notional = 1_000_000.0;
    let init = Money::new(notional, Currency::USD);

    // Zero spread
    let spec = make_float_spec(FloatingRateFallback::Error, dec!(0.0));
    let market = make_flat_forward_market(issue, 0.045);

    let mut b = CashFlowSchedule::builder();
    let _ = b.principal(init, issue, maturity).floating_cf(spec);

    let schedule = b
        .build_with_curves(Some(&market))
        .expect("Golden zero-spread build should succeed with flat curve");

    let float_flows: Vec<_> = schedule
        .flows
        .iter()
        .filter(|cf| cf.kind == CFKind::FloatReset)
        .collect();

    assert_eq!(
        float_flows.len(),
        4,
        "Expected 4 quarterly FloatReset flows, got {}",
        float_flows.len()
    );

    let expected_rate = 0.045; // Index rate only, no spread

    for (i, cf) in float_flows.iter().enumerate() {
        let rate = cf.rate.expect("FloatReset should have a rate");
        assert!(
            (rate - expected_rate).abs() < RATE_TOLERANCE,
            "Flow {}: rate should be exactly {} (index only, zero spread), got {}",
            i,
            expected_rate,
            rate
        );

        // Coupon amount = notional * 0.045 * accrual ~ $11,250 per quarter
        let amount = cf.amount.amount().abs();
        let expected_amount = notional * expected_rate * cf.accrual_factor;
        assert!(
            (amount - expected_amount).abs() < 1.0,
            "Flow {}: amount {:.2} should match rate * notional * accrual ({:.2})",
            i,
            amount,
            expected_amount
        );
    }
}

// =============================================================================
// Golden Value Test 3: Gearing (gearing_includes_spread = true)
// =============================================================================

/// Golden value: gearing=1.5 on 4.5% SOFR + 200bp.
/// With gearing_includes_spread=true: rate = 1.5 * (4.5% + 2.0%) = 9.75%
#[test]
fn test_floating_rate_golden_gearing_includes_spread() {
    let issue = Date::from_calendar_date(2024, Month::January, 15).unwrap();
    let maturity = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    let notional = 1_000_000.0;
    let init = Money::new(notional, Currency::USD);

    let mut spec = make_float_spec(FloatingRateFallback::Error, dec!(200.0));
    spec.rate_spec.gearing = dec!(1.5);
    spec.rate_spec.gearing_includes_spread = true;

    let market = make_flat_forward_market(issue, 0.045);

    let mut b = CashFlowSchedule::builder();
    let _ = b.principal(init, issue, maturity).floating_cf(spec);

    let schedule = b
        .build_with_curves(Some(&market))
        .expect("Golden gearing (includes spread) build should succeed");

    let float_flows: Vec<_> = schedule
        .flows
        .iter()
        .filter(|cf| cf.kind == CFKind::FloatReset)
        .collect();

    assert_eq!(
        float_flows.len(),
        4,
        "Expected 4 quarterly FloatReset flows, got {}",
        float_flows.len()
    );

    // gearing_includes_spread=true: (index + spread) * gearing
    // = (0.045 + 0.02) * 1.5 = 0.065 * 1.5 = 0.0975
    let expected_rate = 0.0975;

    for (i, cf) in float_flows.iter().enumerate() {
        let rate = cf.rate.expect("FloatReset should have a rate");
        assert!(
            (rate - expected_rate).abs() < RATE_TOLERANCE,
            "Flow {}: rate should be {} (1.5 * (4.5% + 2%)), got {}",
            i,
            expected_rate,
            rate
        );

        let amount = cf.amount.amount().abs();
        let expected_amount = notional * expected_rate * cf.accrual_factor;
        assert!(
            (amount - expected_amount).abs() < 1.0,
            "Flow {}: amount {:.2} should match rate * notional * accrual ({:.2})",
            i,
            amount,
            expected_amount
        );
    }
}

// =============================================================================
// Golden Value Test 3b: Gearing (gearing_includes_spread = false, affine)
// =============================================================================

/// Golden value: gearing=1.5 on 4.5% SOFR + 200bp.
/// With gearing_includes_spread=false: rate = (1.5 * 4.5%) + 2.0% = 6.75% + 2.0% = 8.75%
#[test]
fn test_floating_rate_golden_gearing_excludes_spread() {
    let issue = Date::from_calendar_date(2024, Month::January, 15).unwrap();
    let maturity = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    let notional = 1_000_000.0;
    let init = Money::new(notional, Currency::USD);

    let mut spec = make_float_spec(FloatingRateFallback::Error, dec!(200.0));
    spec.rate_spec.gearing = dec!(1.5);
    spec.rate_spec.gearing_includes_spread = false;

    let market = make_flat_forward_market(issue, 0.045);

    let mut b = CashFlowSchedule::builder();
    let _ = b.principal(init, issue, maturity).floating_cf(spec);

    let schedule = b
        .build_with_curves(Some(&market))
        .expect("Golden gearing (excludes spread) build should succeed");

    let float_flows: Vec<_> = schedule
        .flows
        .iter()
        .filter(|cf| cf.kind == CFKind::FloatReset)
        .collect();

    assert_eq!(
        float_flows.len(),
        4,
        "Expected 4 quarterly FloatReset flows, got {}",
        float_flows.len()
    );

    // gearing_includes_spread=false (affine): (index * gearing) + spread
    // = (0.045 * 1.5) + 0.02 = 0.0675 + 0.02 = 0.0875
    let expected_rate = 0.0875;

    for (i, cf) in float_flows.iter().enumerate() {
        let rate = cf.rate.expect("FloatReset should have a rate");
        assert!(
            (rate - expected_rate).abs() < RATE_TOLERANCE,
            "Flow {}: rate should be {} (1.5 * 4.5% + 2%), got {}",
            i,
            expected_rate,
            rate
        );

        let amount = cf.amount.amount().abs();
        let expected_amount = notional * expected_rate * cf.accrual_factor;
        assert!(
            (amount - expected_amount).abs() < 1.0,
            "Flow {}: amount {:.2} should match rate * notional * accrual ({:.2})",
            i,
            amount,
            expected_amount
        );
    }

    // Additionally verify the difference between the two gearing modes:
    // Standard (includes spread): 0.0975
    // Affine (excludes spread): 0.0875
    // Difference = spread * (gearing - 1) = 0.02 * 0.5 = 0.01
    let standard_rate = 0.0975;
    let affine_rate = expected_rate;
    let expected_diff = 0.02 * (1.5 - 1.0); // spread * (gearing - 1) = 0.01
    assert!(
        ((standard_rate - affine_rate) - expected_diff).abs() < RATE_TOLERANCE,
        "Difference between standard and affine should be spread*(gearing-1) = {}, got {}",
        expected_diff,
        standard_rate - affine_rate
    );
}

// =============================================================================
// Cap/Floor and Negative Rate Tests
// =============================================================================

/// Index floor at 0%: negative index rates are clamped to 0.
/// Flat curve at -0.4% with floor at 0 -> all-in rate = 0% + spread.
#[test]
fn test_floating_rate_index_floor_zero() {
    let issue = Date::from_calendar_date(2024, Month::January, 15).unwrap();
    let maturity = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    let notional = 1_000_000.0;
    let init = Money::new(notional, Currency::USD);

    // Build FloatingRateSpec with index floor at 0% and flat curve at -0.4%
    let mut spec = make_float_spec(FloatingRateFallback::Error, dec!(300.0)); // 3% spread
    spec.rate_spec.floor_bp = Some(dec!(0)); // index floored at 0%

    let market = make_flat_forward_market(issue, -0.004); // -0.4% flat curve

    let mut b = CashFlowSchedule::builder();
    let _ = b.principal(init, issue, maturity).floating_cf(spec);

    let schedule = b
        .build_with_curves(Some(&market))
        .expect("Index floor test should succeed with flat curve");

    let float_flows: Vec<_> = schedule
        .flows
        .iter()
        .filter(|cf| cf.kind == CFKind::FloatReset)
        .collect();

    assert!(
        !float_flows.is_empty(),
        "Should have at least one FloatReset flow"
    );

    // Index = -0.4%, floored at 0% -> eff_index = 0%
    // all-in = (0% + 3%) * 1.0 = 3% = 0.03
    let expected_rate = 0.03;
    for (i, cf) in float_flows.iter().enumerate() {
        let rate = cf.rate.expect("FloatReset should have a rate");
        assert!(
            (rate - expected_rate).abs() < RATE_TOLERANCE,
            "Flow {}: rate should be {} (index floored at 0% + 300bp spread), got {}",
            i,
            expected_rate,
            rate
        );
    }
}

/// Index cap at 5%: index rate clamped to cap.
/// Flat curve at 6% with index_cap at 5% -> all-in = 5% + spread.
#[test]
fn test_floating_rate_index_cap() {
    let issue = Date::from_calendar_date(2024, Month::January, 15).unwrap();
    let maturity = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    let notional = 1_000_000.0;
    let init = Money::new(notional, Currency::USD);

    // Build FloatingRateSpec with index cap at 5% and flat curve at 6%
    let mut spec = make_float_spec(FloatingRateFallback::Error, dec!(200.0)); // 2% spread
    spec.rate_spec.index_cap_bp = Some(dec!(500)); // 5% cap on index

    let market = make_flat_forward_market(issue, 0.06); // 6% flat curve

    let mut b = CashFlowSchedule::builder();
    let _ = b.principal(init, issue, maturity).floating_cf(spec);

    let schedule = b
        .build_with_curves(Some(&market))
        .expect("Index cap test should succeed with flat curve");

    let float_flows: Vec<_> = schedule
        .flows
        .iter()
        .filter(|cf| cf.kind == CFKind::FloatReset)
        .collect();

    assert!(
        !float_flows.is_empty(),
        "Should have at least one FloatReset flow"
    );

    // Index = 6%, capped at 5% -> eff_index = 5%
    // all-in = (5% + 2%) * 1.0 = 7% = 0.07
    let expected_rate = 0.07;
    for (i, cf) in float_flows.iter().enumerate() {
        let rate = cf.rate.expect("FloatReset should have a rate");
        assert!(
            (rate - expected_rate).abs() < RATE_TOLERANCE,
            "Flow {}: rate should be {} (index capped at 5% + 200bp spread), got {}",
            i,
            expected_rate,
            rate
        );
    }
}

/// All-in cap at 7%: total rate clamped after adding spread.
/// Flat curve at 6%, spread 200bp, cap at 7% -> uncapped = 8%, capped = 7%.
#[test]
fn test_floating_rate_all_in_cap() {
    let issue = Date::from_calendar_date(2024, Month::January, 15).unwrap();
    let maturity = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    let notional = 1_000_000.0;
    let init = Money::new(notional, Currency::USD);

    // Build FloatingRateSpec with all-in cap at 7%
    let mut spec = make_float_spec(FloatingRateFallback::Error, dec!(200.0)); // 2% spread
    spec.rate_spec.cap_bp = Some(dec!(700)); // 7% all-in cap

    let market = make_flat_forward_market(issue, 0.06); // 6% flat curve

    let mut b = CashFlowSchedule::builder();
    let _ = b.principal(init, issue, maturity).floating_cf(spec);

    let schedule = b
        .build_with_curves(Some(&market))
        .expect("All-in cap test should succeed with flat curve");

    let float_flows: Vec<_> = schedule
        .flows
        .iter()
        .filter(|cf| cf.kind == CFKind::FloatReset)
        .collect();

    assert!(
        !float_flows.is_empty(),
        "Should have at least one FloatReset flow"
    );

    // Uncapped = 6% + 2% = 8%, but all-in cap at 7% -> rate = 0.07
    let expected_rate = 0.07;
    for (i, cf) in float_flows.iter().enumerate() {
        let rate = cf.rate.expect("FloatReset should have a rate");
        assert!(
            (rate - expected_rate).abs() < RATE_TOLERANCE,
            "Flow {}: rate should be {} (all-in capped at 7%), got {}",
            i,
            expected_rate,
            rate
        );
    }
}

/// Negative rate: EUR EURIBOR at -0.40% + 300bp spread, no floor.
/// All-in rate = -0.004 + 0.03 = 0.026.
#[test]
fn test_floating_rate_negative_index_no_floor() {
    let issue = Date::from_calendar_date(2024, Month::January, 15).unwrap();
    let maturity = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    let notional = 1_000_000.0;
    let init = Money::new(notional, Currency::USD);

    // No floor, negative index rate
    let spec = make_float_spec(FloatingRateFallback::Error, dec!(300.0)); // 3% spread, no floor
    let market = make_flat_forward_market(issue, -0.004); // -0.4% flat curve

    let mut b = CashFlowSchedule::builder();
    let _ = b.principal(init, issue, maturity).floating_cf(spec);

    let schedule = b
        .build_with_curves(Some(&market))
        .expect("Negative index no-floor test should succeed with flat curve");

    let float_flows: Vec<_> = schedule
        .flows
        .iter()
        .filter(|cf| cf.kind == CFKind::FloatReset)
        .collect();

    assert!(
        !float_flows.is_empty(),
        "Should have at least one FloatReset flow"
    );

    // No floor: index = -0.4%, spread = 3%
    // all-in = (-0.004 + 0.03) * 1.0 = 0.026
    let expected_rate = 0.026;
    for (i, cf) in float_flows.iter().enumerate() {
        let rate = cf.rate.expect("FloatReset should have a rate");
        assert!(
            (rate - expected_rate).abs() < RATE_TOLERANCE,
            "Flow {}: rate should be {} (negative index -0.4% + 300bp spread, no floor), got {}",
            i,
            expected_rate,
            rate
        );
    }
}

/// All-in floor at 1%: total rate floored after adding spread.
/// Flat curve at -2%, spread 200bp -> uncapped = -2% + 2% = 0%, but all-in floor at 1% -> rate = 1%.
#[test]
fn test_floating_rate_all_in_floor() {
    let issue = Date::from_calendar_date(2024, Month::January, 15).unwrap();
    let maturity = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    let notional = 1_000_000.0;
    let init = Money::new(notional, Currency::USD);

    // Build FloatingRateSpec with all-in floor at 1%
    let mut spec = make_float_spec(FloatingRateFallback::Error, dec!(200.0)); // 2% spread
    spec.rate_spec.all_in_floor_bp = Some(dec!(100)); // 1% all-in floor

    let market = make_flat_forward_market(issue, -0.02); // -2% flat curve

    let mut b = CashFlowSchedule::builder();
    let _ = b.principal(init, issue, maturity).floating_cf(spec);

    let schedule = b
        .build_with_curves(Some(&market))
        .expect("All-in floor test should succeed with flat curve");

    let float_flows: Vec<_> = schedule
        .flows
        .iter()
        .filter(|cf| cf.kind == CFKind::FloatReset)
        .collect();

    assert!(
        !float_flows.is_empty(),
        "Should have at least one FloatReset flow"
    );

    // Unfloored = -2% + 2% = 0%, but all-in floor at 1% -> rate = 0.01
    let expected_rate = 0.01;
    for (i, cf) in float_flows.iter().enumerate() {
        let rate = cf.rate.expect("FloatReset should have a rate");
        assert!(
            (rate - expected_rate).abs() < RATE_TOLERANCE,
            "Flow {}: rate should be {} (all-in floored at 1%), got {}",
            i,
            expected_rate,
            rate
        );
    }
}
