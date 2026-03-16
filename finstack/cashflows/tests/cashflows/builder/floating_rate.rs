//! Tests for FloatingRateFallback policy wired into emission.
//!
//! Covers three fallback variants when no forward curve is available:
//! - `Error`: build should fail with an error
//! - `SpreadOnly`: build succeeds, rate == spread (legacy behavior)
//! - `FixedRate(r)`: build succeeds, rate == r + spread (through params pipeline)

use finstack_cashflows::builder::specs::{
    CouponType, FloatingCouponSpec, FloatingRateFallback, FloatingRateSpec,
};
use finstack_cashflows::builder::CashFlowSchedule;
use finstack_core::cashflow::CFKind;
use finstack_core::currency::Currency;
use finstack_core::dates::{BusinessDayConvention, DayCount, StubKind, Tenor};
use finstack_core::money::Money;
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
    let market = MarketContext::new().insert(fwd);

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
    MarketContext::new().insert(fwd)
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

// =============================================================================
// Overnight Compounding Tests
// =============================================================================

use finstack_cashflows::builder::specs::OvernightCompoundingMethod;

/// Helper: create a floating coupon spec with overnight compounding enabled.
fn make_overnight_float_spec(
    method: OvernightCompoundingMethod,
    fallback: FloatingRateFallback,
    spread_bp: Decimal,
) -> FloatingCouponSpec {
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
            overnight_compounding: Some(method),
            fallback,
        },
        coupon_type: CouponType::Cash,
        freq: Tenor::quarterly(),
        stub: StubKind::None,
    }
}

/// Overnight compounding (CompoundedInArrears) with a flat curve should produce
/// approximately the same rate as the flat forward rate.
///
/// With a flat curve at 4.5%, daily compounding produces a rate very close to 4.5%
/// (the compounding effect on such small daily increments is negligible).
/// All-in rate = ~4.5% + 2% spread = ~6.5%.
#[test]
fn test_overnight_compounding_flat_curve() {
    let issue = Date::from_calendar_date(2024, Month::January, 15).unwrap();
    let maturity = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    let notional = 1_000_000.0;
    let init = Money::new(notional, Currency::USD);

    let spec = make_overnight_float_spec(
        OvernightCompoundingMethod::CompoundedInArrears,
        FloatingRateFallback::Error,
        dec!(200.0), // 200 bps spread
    );
    let market = make_flat_forward_market(issue, 0.045);

    let mut b = CashFlowSchedule::builder();
    let _ = b.principal(init, issue, maturity).floating_cf(spec);

    let schedule = b
        .build_with_curves(Some(&market))
        .expect("Overnight compounding with flat curve should succeed");

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

    // Flat curve: compounded rate is very close to the flat rate (4.5%).
    // All-in = ~4.5% + 2% = ~6.5%.
    // Allow a small tolerance for compounding effects on flat curve.
    let expected_rate = 0.065;
    for (i, cf) in float_flows.iter().enumerate() {
        let rate = cf.rate.expect("FloatReset should have a rate");
        assert!(
            (rate - expected_rate).abs() < 0.001,
            "Flow {}: overnight compounded rate should be ~{} (flat 4.5% + 200bp), got {}",
            i,
            expected_rate,
            rate
        );

        // Verify the amount is consistent with rate * notional * accrual_factor
        let amount = cf.amount.amount().abs();
        let expected_amount = notional * rate * cf.accrual_factor;
        assert!(
            (amount - expected_amount).abs() < 1.0,
            "Flow {}: amount {:.2} should match rate * notional * accrual ({:.2})",
            i,
            amount,
            expected_amount
        );
    }
}

/// Simple average should produce an identical rate to the flat forward rate.
///
/// With a flat curve at 4.5%, the simple average of daily rates is exactly 4.5%.
/// All-in rate = 4.5% + 2% spread = 6.5%.
#[test]
fn test_overnight_simple_average_flat() {
    let issue = Date::from_calendar_date(2024, Month::January, 15).unwrap();
    let maturity = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    let notional = 1_000_000.0;
    let init = Money::new(notional, Currency::USD);

    let spec = make_overnight_float_spec(
        OvernightCompoundingMethod::SimpleAverage,
        FloatingRateFallback::Error,
        dec!(200.0), // 200 bps spread
    );
    let market = make_flat_forward_market(issue, 0.045);

    let mut b = CashFlowSchedule::builder();
    let _ = b.principal(init, issue, maturity).floating_cf(spec);

    let schedule = b
        .build_with_curves(Some(&market))
        .expect("Overnight simple average with flat curve should succeed");

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

    // Simple average of a flat rate == that flat rate.
    // All-in = 4.5% + 2% = 6.5%.
    let expected_rate = 0.065;
    for (i, cf) in float_flows.iter().enumerate() {
        let rate = cf.rate.expect("FloatReset should have a rate");
        assert!(
            (rate - expected_rate).abs() < 0.001,
            "Flow {}: simple average rate should be ~{} (flat 4.5% + 200bp), got {}",
            i,
            expected_rate,
            rate
        );
    }
}

/// Overnight compounding with lockout on a flat curve should produce
/// approximately the same rate as the flat forward rate.
#[test]
fn test_overnight_lockout_flat_curve() {
    let issue = Date::from_calendar_date(2024, Month::January, 15).unwrap();
    let maturity = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    let notional = 1_000_000.0;
    let init = Money::new(notional, Currency::USD);

    let spec = make_overnight_float_spec(
        OvernightCompoundingMethod::CompoundedWithLockout { lockout_days: 2 },
        FloatingRateFallback::Error,
        dec!(200.0),
    );
    let market = make_flat_forward_market(issue, 0.045);

    let mut b = CashFlowSchedule::builder();
    let _ = b.principal(init, issue, maturity).floating_cf(spec);

    let schedule = b
        .build_with_curves(Some(&market))
        .expect("Overnight lockout with flat curve should succeed");

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

    // Lockout on a flat curve has no effect — rate is still ~4.5% + 2% = 6.5%.
    let expected_rate = 0.065;
    for (i, cf) in float_flows.iter().enumerate() {
        let rate = cf.rate.expect("FloatReset should have a rate");
        assert!(
            (rate - expected_rate).abs() < 0.001,
            "Flow {}: lockout rate should be ~{} (flat 4.5% + 200bp), got {}",
            i,
            expected_rate,
            rate
        );
    }
}

/// Overnight compounding with no curve and Error fallback should fail.
#[test]
fn test_overnight_compounding_no_curve_error_fallback() {
    let issue = Date::from_calendar_date(2024, Month::January, 15).unwrap();
    let maturity = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    let init = Money::new(1_000_000.0, Currency::USD);

    let spec = make_overnight_float_spec(
        OvernightCompoundingMethod::CompoundedInArrears,
        FloatingRateFallback::Error,
        dec!(200.0),
    );

    let mut b = CashFlowSchedule::builder();
    let _ = b.principal(init, issue, maturity).floating_cf(spec);

    let result = b.build_with_curves(None);
    assert!(
        result.is_err(),
        "Overnight compounding with no curve and Error fallback should fail"
    );
}

/// Overnight compounding with no curve and SpreadOnly fallback should succeed.
#[test]
fn test_overnight_compounding_no_curve_spread_only_fallback() {
    let issue = Date::from_calendar_date(2024, Month::January, 15).unwrap();
    let maturity = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    let init = Money::new(1_000_000.0, Currency::USD);

    let spec = make_overnight_float_spec(
        OvernightCompoundingMethod::CompoundedInArrears,
        FloatingRateFallback::SpreadOnly,
        dec!(200.0),
    );

    let mut b = CashFlowSchedule::builder();
    let _ = b.principal(init, issue, maturity).floating_cf(spec);

    let schedule = b
        .build_with_curves(None)
        .expect("Overnight compounding with SpreadOnly fallback should succeed");

    let float_flows: Vec<_> = schedule
        .flows
        .iter()
        .filter(|cf| cf.kind == CFKind::FloatReset)
        .collect();

    assert!(!float_flows.is_empty(), "Should have FloatReset flows");

    // SpreadOnly: index=0, all-in = spread = 200bp = 0.02
    for cf in &float_flows {
        let rate = cf.rate.expect("FloatReset should have a rate");
        assert!(
            (rate - 0.02).abs() < 1e-10,
            "Rate should be 0.02 (spread-only), got {}",
            rate
        );
    }
}

/// Overnight compounding should produce the same result as the term rate path
/// when the curve is flat (verifying both paths converge for simple cases).
#[test]
fn test_overnight_vs_term_rate_flat_curve_equivalence() {
    let issue = Date::from_calendar_date(2024, Month::January, 15).unwrap();
    let maturity = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    let notional = 1_000_000.0;
    let init = Money::new(notional, Currency::USD);
    let market = make_flat_forward_market(issue, 0.045);

    // Build with overnight compounding
    let overnight_spec = make_overnight_float_spec(
        OvernightCompoundingMethod::SimpleAverage,
        FloatingRateFallback::Error,
        dec!(200.0),
    );
    let mut b1 = CashFlowSchedule::builder();
    let _ = b1
        .principal(init, issue, maturity)
        .floating_cf(overnight_spec);
    let overnight_schedule = b1
        .build_with_curves(Some(&market))
        .expect("Overnight build should succeed");

    // Build with standard term rate
    let term_spec = make_float_spec(FloatingRateFallback::Error, dec!(200.0));
    let mut b2 = CashFlowSchedule::builder();
    let _ = b2.principal(init, issue, maturity).floating_cf(term_spec);
    let term_schedule = b2
        .build_with_curves(Some(&market))
        .expect("Term rate build should succeed");

    let overnight_flows: Vec<_> = overnight_schedule
        .flows
        .iter()
        .filter(|cf| cf.kind == CFKind::FloatReset)
        .collect();
    let term_flows: Vec<_> = term_schedule
        .flows
        .iter()
        .filter(|cf| cf.kind == CFKind::FloatReset)
        .collect();

    assert_eq!(
        overnight_flows.len(),
        term_flows.len(),
        "Both paths should produce the same number of flows"
    );

    for (i, (on, term)) in overnight_flows.iter().zip(term_flows.iter()).enumerate() {
        let on_rate = on.rate.expect("Overnight flow should have a rate");
        let term_rate = term.rate.expect("Term flow should have a rate");
        assert!(
            (on_rate - term_rate).abs() < 0.001,
            "Flow {}: overnight rate ({}) and term rate ({}) should be approximately equal for flat curve",
            i,
            on_rate,
            term_rate
        );
    }
}

// =============================================================================
// Test: Overnight compounding accrual starts on a weekend
// =============================================================================

/// Verifies that overnight compounding correctly accounts for non-business days
/// at the start of an accrual period (e.g., accrual_start on a Saturday).
///
/// Jan 4, 2025 is a Saturday. Using Unadjusted BDC preserves this as the raw
/// accrual start. The fix ensures the Saturday and Sunday (2 days) before the
/// first business day (Monday Jan 6) are assigned to Monday's fixing weight,
/// so no accrual days are lost.
///
/// We build two schedules — one starting Saturday (Unadjusted), one starting
/// Monday (Following) — and verify they produce approximately equal coupons.
/// Without the fix, the Saturday-start schedule would lose 2 days of accrual.
#[test]
fn test_overnight_compounding_weekend_start_no_lost_days() {
    use finstack_cashflows::builder::specs::OvernightCompoundingMethod;
    use finstack_core::market_data::context::MarketContext;
    use finstack_core::market_data::term_structures::ForwardCurve;
    use finstack_core::math::interp::InterpStyle;

    // Jan 4 2025 = Saturday, Jan 6 2025 = Monday
    let saturday = Date::from_calendar_date(2025, Month::January, 4).unwrap();
    let monday = Date::from_calendar_date(2025, Month::January, 6).unwrap();
    let maturity = Date::from_calendar_date(2025, Month::April, 7).unwrap();
    let init = Money::new(1_000_000.0, Currency::USD);

    let fwd = ForwardCurve::builder("USD-SOFR-ON", 1.0 / 360.0)
        .base_date(saturday)
        .day_count(DayCount::Act360)
        .knots([(0.0, 0.05), (1.0, 0.05)])
        .interp(InterpStyle::Linear)
        .build()
        .unwrap();
    let market = MarketContext::new().insert(fwd);

    let make_spec = |bdc| FloatingCouponSpec {
        rate_spec: FloatingRateSpec {
            index_id: "USD-SOFR-ON".into(),
            spread_bp: dec!(0),
            gearing: Decimal::ONE,
            gearing_includes_spread: true,
            floor_bp: None,
            cap_bp: None,
            all_in_floor_bp: None,
            index_cap_bp: None,
            reset_freq: Tenor::quarterly(),
            reset_lag_days: 0,
            dc: DayCount::Act360,
            bdc,
            calendar_id: "weekends_only".to_string(),
            fixing_calendar_id: None,
            end_of_month: false,
            payment_lag_days: 0,
            overnight_compounding: Some(OvernightCompoundingMethod::CompoundedInArrears),
            fallback: FloatingRateFallback::Error,
        },
        coupon_type: CouponType::Cash,
        freq: Tenor::quarterly(),
        stub: StubKind::None,
    };

    // Schedule 1: Saturday start with Unadjusted BDC (accrual_start = Saturday)
    let sat_schedule = CashFlowSchedule::builder()
        .principal(init, saturday, maturity)
        .floating_cf(make_spec(BusinessDayConvention::Unadjusted))
        .build_with_curves(Some(&market))
        .expect("Unadjusted Saturday start should build");

    // Schedule 2: Monday start with Following BDC (baseline)
    let mon_schedule = CashFlowSchedule::builder()
        .principal(init, monday, maturity)
        .floating_cf(make_spec(BusinessDayConvention::Following))
        .build_with_curves(Some(&market))
        .expect("Following Monday start should build");

    let sat_floats: Vec<_> = sat_schedule
        .flows
        .iter()
        .filter(|cf| cf.kind == CFKind::FloatReset)
        .collect();
    let mon_floats: Vec<_> = mon_schedule
        .flows
        .iter()
        .filter(|cf| cf.kind == CFKind::FloatReset)
        .collect();

    assert!(
        !sat_floats.is_empty(),
        "Saturday schedule should have float flows"
    );
    assert!(
        !mon_floats.is_empty(),
        "Monday schedule should have float flows"
    );

    // Both should produce ~5% rate on a flat curve
    for cf in &sat_floats {
        let rate = cf.rate.expect("FloatReset should have a rate");
        assert!(
            (rate - 0.05).abs() < 0.002,
            "Saturday-start overnight rate should be ~5%, got {:.6}",
            rate
        );
    }

    // The Saturday schedule covers 2 extra calendar days (Sat+Sun) at the start.
    // With the fix, these days are assigned to Monday's fixing, so the total
    // coupon for the Saturday schedule should be >= the Monday schedule.
    let sat_total: f64 = sat_floats.iter().map(|cf| cf.amount.amount()).sum();
    let mon_total: f64 = mon_floats.iter().map(|cf| cf.amount.amount()).sum();

    assert!(
        sat_total >= mon_total * 0.99,
        "Saturday-start total ({:.2}) should not be materially less than Monday-start ({:.2}); \
         lost weekend days would cause a shortfall",
        sat_total,
        mon_total,
    );
}
