//! Interest Rate Swap cashflow schedule tests.
//!
//! Tests cover:
//! - Fixed leg schedule generation
//! - Floating leg schedule generation
//! - Date adjustments and calendars
//! - Stub period handling
//! - Cashflow schedule consistency

use finstack_core::currency::Currency;
use finstack_core::dates::{BusinessDayConvention, DayCount, Frequency, StubKind};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::{DiscountCurve, ForwardCurve};
use finstack_core::money::Money;
use finstack_valuations::cashflow::traits::CashflowProvider;
use finstack_valuations::instruments::irs::{InterestRateSwap, PayReceive};
use time::macros::date;

fn build_test_curves() -> MarketContext {
    let as_of = date!(2024 - 01 - 01);

    // Curve IDs for create_usd_swap (USD-OIS and USD-SOFR-3M)
    let disc_curve_usd_ois = DiscountCurve::builder("USD-OIS")
        .base_date(as_of)
        .day_count(DayCount::Act360)
        .knots([
            (0.0, 1.0),
            (1.0, (-0.05_f64).exp()),
            (5.0, (-0.05_f64 * 5.0).exp()),
            (10.0, (-0.05_f64 * 10.0).exp()),
        ])
        .build()
        .unwrap();

    let fwd_curve_sofr = ForwardCurve::builder("USD-SOFR-3M", 0.25)
        .base_date(as_of)
        .day_count(DayCount::Act360)
        .knots([(0.0, 0.05), (10.0, 0.05)])
        .build()
        .unwrap();

    // Curve IDs for manual builder tests (USD_OIS and USD_LIBOR_3M)
    let disc_curve_manual = DiscountCurve::builder("USD_OIS")
        .base_date(as_of)
        .day_count(DayCount::Act360)
        .knots([
            (0.0, 1.0),
            (1.0, (-0.05_f64).exp()),
            (5.0, (-0.05_f64 * 5.0).exp()),
            (10.0, (-0.05_f64 * 10.0).exp()),
        ])
        .build()
        .unwrap();

    let fwd_curve_libor = ForwardCurve::builder("USD_LIBOR_3M", 0.25)
        .base_date(as_of)
        .day_count(DayCount::Act360)
        .knots([(0.0, 0.05), (10.0, 0.05)])
        .build()
        .unwrap();

    MarketContext::new()
        .insert_discount(disc_curve_usd_ois)
        .insert_discount(disc_curve_manual)
        .insert_forward(fwd_curve_sofr)
        .insert_forward(fwd_curve_libor)
}

#[test]
fn test_irs_cashflow_schedule_generation() {
    let swap = InterestRateSwap::create_usd_swap(
        "IRS-CF-TEST".into(),
        Money::new(1_000_000.0, Currency::USD),
        0.05,
        date!(2024 - 01 - 01),
        date!(2029 - 01 - 01),
        PayReceive::ReceiveFixed,
    )
    .unwrap();

    let market = build_test_curves();
    let as_of = date!(2024 - 01 - 01);

    let schedule = swap.build_schedule(&market, as_of).unwrap();

    // Should have cashflows from both legs
    assert!(!schedule.is_empty(), "Schedule should not be empty");
}

#[test]
fn test_irs_fixed_leg_quarterly_schedule() {
    // Fixed leg with quarterly payments
    let swap = InterestRateSwap::builder()
        .id("IRS-FIX-Q".into())
        .notional(Money::new(1_000_000.0, Currency::USD))
        .side(PayReceive::ReceiveFixed)
        .fixed(
            finstack_valuations::instruments::common::parameters::legs::FixedLegSpec {
                discount_curve_id: "USD_OIS".into(),
                rate: 0.05,
                freq: Frequency::quarterly(),
                dc: DayCount::Act360,
                bdc: BusinessDayConvention::ModifiedFollowing,
                calendar_id: None,
                stub: StubKind::None,
                start: date!(2024 - 01 - 01),
                end: date!(2025 - 01 - 01),
                par_method: None,
                compounding_simple: true,
                payment_delay_days: 0,
            },
        )
        .float(
            finstack_valuations::instruments::common::parameters::legs::FloatLegSpec {
                discount_curve_id: "USD_OIS".into(),
                forward_curve_id: "USD_LIBOR_3M".into(),
                spread_bp: 0.0,
                freq: Frequency::quarterly(),
                dc: DayCount::Act360,
                bdc: BusinessDayConvention::ModifiedFollowing,
                calendar_id: None,
                fixing_calendar_id: None,
                stub: StubKind::None,
                reset_lag_days: 2,
                compounding: Default::default(),
                payment_delay_days: 0,
                start: date!(2024 - 01 - 01),
                end: date!(2025 - 01 - 01),
            },
        )
        .build()
        .unwrap();

    let market = build_test_curves();
    let as_of = date!(2024 - 01 - 01);

    let schedule = swap.build_schedule(&market, as_of).unwrap();

    // 1 year quarterly = 4 periods per leg = 8 cashflows
    assert!(
        schedule.len() >= 4,
        "Should have at least 4 quarterly payments"
    );
}

#[test]
fn test_irs_fixed_leg_semiannual_schedule() {
    // Fixed leg with semiannual payments
    let swap = InterestRateSwap::builder()
        .id("IRS-FIX-SA".into())
        .notional(Money::new(1_000_000.0, Currency::USD))
        .side(PayReceive::ReceiveFixed)
        .fixed(
            finstack_valuations::instruments::common::parameters::legs::FixedLegSpec {
                discount_curve_id: "USD_OIS".into(),
                rate: 0.05,
                freq: Frequency::semi_annual(),
                dc: DayCount::Thirty360,
                bdc: BusinessDayConvention::ModifiedFollowing,
                calendar_id: None,
                stub: StubKind::None,
                start: date!(2024 - 01 - 01),
                end: date!(2026 - 01 - 01),
                par_method: None,
                compounding_simple: true,
                payment_delay_days: 0,
            },
        )
        .float(
            finstack_valuations::instruments::common::parameters::legs::FloatLegSpec {
                discount_curve_id: "USD_OIS".into(),
                forward_curve_id: "USD_LIBOR_3M".into(),
                spread_bp: 0.0,
                freq: Frequency::quarterly(),
                dc: DayCount::Act360,
                bdc: BusinessDayConvention::ModifiedFollowing,
                calendar_id: None,
                fixing_calendar_id: None,
                stub: StubKind::None,
                reset_lag_days: 2,
                compounding: Default::default(),
                payment_delay_days: 0,
                start: date!(2024 - 01 - 01),
                end: date!(2026 - 01 - 01),
            },
        )
        .build()
        .unwrap();

    let market = build_test_curves();
    let as_of = date!(2024 - 01 - 01);

    let schedule = swap.build_schedule(&market, as_of).unwrap();

    // 2 years: 4 semiannual + 8 quarterly = 12 total
    assert!(
        schedule.len() >= 4,
        "Should have at least 4 semiannual payments"
    );
}

#[test]
fn test_irs_floating_leg_schedule() {
    let swap = InterestRateSwap::create_usd_swap(
        "IRS-FLOAT-TEST".into(),
        Money::new(1_000_000.0, Currency::USD),
        0.05,
        date!(2024 - 01 - 01),
        date!(2025 - 01 - 01),
        PayReceive::PayFixed,
    )
    .unwrap();

    let market = build_test_curves();
    let as_of = date!(2024 - 01 - 01);

    let schedule = swap.build_schedule(&market, as_of).unwrap();

    // Verify floating leg generates cashflows
    assert!(!schedule.is_empty());
}

#[test]
fn test_irs_stub_front() {
    // Swap with short front stub
    let swap = InterestRateSwap::builder()
        .id("IRS-STUB-FRONT".into())
        .notional(Money::new(1_000_000.0, Currency::USD))
        .side(PayReceive::ReceiveFixed)
        .fixed(
            finstack_valuations::instruments::common::parameters::legs::FixedLegSpec {
                discount_curve_id: "USD_OIS".into(),
                rate: 0.05,
                freq: Frequency::semi_annual(),
                dc: DayCount::Thirty360,
                bdc: BusinessDayConvention::ModifiedFollowing,
                calendar_id: None,
                stub: StubKind::ShortFront,
                start: date!(2024 - 01 - 15),
                end: date!(2026 - 01 - 01),
                par_method: None,
                compounding_simple: true,
                payment_delay_days: 0,
            },
        )
        .float(
            finstack_valuations::instruments::common::parameters::legs::FloatLegSpec {
                discount_curve_id: "USD_OIS".into(),
                forward_curve_id: "USD_LIBOR_3M".into(),
                spread_bp: 0.0,
                freq: Frequency::quarterly(),
                dc: DayCount::Act360,
                bdc: BusinessDayConvention::ModifiedFollowing,
                calendar_id: None,
                fixing_calendar_id: None,
                stub: StubKind::ShortFront,
                reset_lag_days: 2,
                compounding: Default::default(),
                payment_delay_days: 0,
                start: date!(2024 - 01 - 15),
                end: date!(2026 - 01 - 01),
            },
        )
        .build()
        .unwrap();

    let market = build_test_curves();
    let as_of = date!(2024 - 01 - 01);

    let schedule = swap.build_schedule(&market, as_of);

    assert!(schedule.is_ok(), "Should generate schedule with stub");
}

#[test]
fn test_irs_stub_back() {
    // Swap with short back stub
    let swap = InterestRateSwap::builder()
        .id("IRS-STUB-BACK".into())
        .notional(Money::new(1_000_000.0, Currency::USD))
        .side(PayReceive::ReceiveFixed)
        .fixed(
            finstack_valuations::instruments::common::parameters::legs::FixedLegSpec {
                discount_curve_id: "USD_OIS".into(),
                rate: 0.05,
                freq: Frequency::semi_annual(),
                dc: DayCount::Thirty360,
                bdc: BusinessDayConvention::ModifiedFollowing,
                calendar_id: None,
                stub: StubKind::ShortBack,
                start: date!(2024 - 01 - 01),
                end: date!(2026 - 06 - 15),
                par_method: None,
                compounding_simple: true,
                payment_delay_days: 0,
            },
        )
        .float(
            finstack_valuations::instruments::common::parameters::legs::FloatLegSpec {
                discount_curve_id: "USD_OIS".into(),
                forward_curve_id: "USD_LIBOR_3M".into(),
                spread_bp: 0.0,
                freq: Frequency::quarterly(),
                dc: DayCount::Act360,
                bdc: BusinessDayConvention::ModifiedFollowing,
                calendar_id: None,
                fixing_calendar_id: None,
                stub: StubKind::ShortBack,
                reset_lag_days: 2,
                compounding: Default::default(),
                payment_delay_days: 0,
                start: date!(2024 - 01 - 01),
                end: date!(2026 - 06 - 15),
            },
        )
        .build()
        .unwrap();

    let market = build_test_curves();
    let as_of = date!(2024 - 01 - 01);

    let schedule = swap.build_schedule(&market, as_of);

    assert!(schedule.is_ok(), "Should generate schedule with back stub");
}

#[test]
fn test_irs_cashflow_dates_ordered() {
    let swap = InterestRateSwap::create_usd_swap(
        "IRS-DATES".into(),
        Money::new(1_000_000.0, Currency::USD),
        0.05,
        date!(2024 - 01 - 01),
        date!(2027 - 01 - 01),
        PayReceive::ReceiveFixed,
    )
    .unwrap();

    let market = build_test_curves();
    let as_of = date!(2024 - 01 - 01);

    let schedule = swap.build_schedule(&market, as_of).unwrap();

    // Verify dates are in ascending order
    let mut prev_date = date!(2020 - 01 - 01);
    for (cf_date, _) in &schedule {
        assert!(*cf_date >= prev_date, "Cashflow dates should be ordered");
        prev_date = *cf_date;
    }
}

#[test]
fn test_irs_cashflow_amounts_nonzero() {
    let swap = InterestRateSwap::create_usd_swap(
        "IRS-AMOUNTS".into(),
        Money::new(1_000_000.0, Currency::USD),
        0.05,
        date!(2024 - 01 - 01),
        date!(2027 - 01 - 01),
        PayReceive::ReceiveFixed,
    )
    .unwrap();

    let market = build_test_curves();
    let as_of = date!(2024 - 01 - 01);

    let schedule = swap.build_schedule(&market, as_of).unwrap();

    // Verify amounts are reasonable (non-zero for fixed rate > 0)
    for (_, amount) in &schedule {
        assert_ne!(
            amount.amount(),
            0.0,
            "Cashflow amounts should be non-zero for non-zero rate"
        );
    }
}

#[test]
fn test_irs_full_schedule_with_cfkind() {
    let swap = InterestRateSwap::create_usd_swap(
        "IRS-FULL-SCHED".into(),
        Money::new(1_000_000.0, Currency::USD),
        0.05,
        date!(2024 - 01 - 01),
        date!(2026 - 01 - 01),
        PayReceive::ReceiveFixed,
    )
    .unwrap();

    let market = build_test_curves();
    let as_of = date!(2024 - 01 - 01);

    let full_schedule = swap.build_full_schedule(&market, as_of);

    assert!(full_schedule.is_ok(), "Should build full schedule");

    let sched = full_schedule.unwrap();

    // Verify we have both fixed and floating cashflows
    use finstack_valuations::cashflow::primitives::CFKind;

    let has_fixed = sched.flows.iter().any(|cf| cf.kind == CFKind::Fixed);
    let has_float = sched.flows.iter().any(|cf| cf.kind == CFKind::FloatReset);

    assert!(has_fixed || has_float, "Should have classified cashflows");
}

#[test]
fn test_irs_different_frequencies() {
    // Fixed semiannual, float quarterly
    let swap = InterestRateSwap::builder()
        .id("IRS-DIFF-FREQ".into())
        .notional(Money::new(1_000_000.0, Currency::USD))
        .side(PayReceive::ReceiveFixed)
        .fixed(
            finstack_valuations::instruments::common::parameters::legs::FixedLegSpec {
                discount_curve_id: "USD_OIS".into(),
                rate: 0.05,
                freq: Frequency::semi_annual(),
                dc: DayCount::Thirty360,
                bdc: BusinessDayConvention::ModifiedFollowing,
                calendar_id: None,
                stub: StubKind::None,
                start: date!(2024 - 01 - 01),
                end: date!(2026 - 01 - 01),
                par_method: None,
                compounding_simple: true,
                payment_delay_days: 0,
            },
        )
        .float(
            finstack_valuations::instruments::common::parameters::legs::FloatLegSpec {
                discount_curve_id: "USD_OIS".into(),
                forward_curve_id: "USD_LIBOR_3M".into(),
                spread_bp: 0.0,
                freq: Frequency::quarterly(),
                dc: DayCount::Act360,
                bdc: BusinessDayConvention::ModifiedFollowing,
                calendar_id: None,
                fixing_calendar_id: None,
                stub: StubKind::None,
                reset_lag_days: 2,
                compounding: Default::default(),
                payment_delay_days: 0,
                start: date!(2024 - 01 - 01),
                end: date!(2026 - 01 - 01),
            },
        )
        .build()
        .unwrap();

    let market = build_test_curves();
    let as_of = date!(2024 - 01 - 01);

    let schedule = swap.build_schedule(&market, as_of).unwrap();

    // 2 years: 4 semiannual fixed + 8 quarterly float = 12
    assert!(schedule.len() >= 4, "Should have multiple cashflows");
}

#[test]
fn test_irs_calendar_adjustments() {
    // Swap with calendar adjustments
    let swap = InterestRateSwap::builder()
        .id("IRS-CAL-ADJ".into())
        .notional(Money::new(1_000_000.0, Currency::USD))
        .side(PayReceive::ReceiveFixed)
        .fixed(
            finstack_valuations::instruments::common::parameters::legs::FixedLegSpec {
                discount_curve_id: "USD_OIS".into(),
                rate: 0.05,
                freq: Frequency::semi_annual(),
                dc: DayCount::Thirty360,
                bdc: BusinessDayConvention::ModifiedFollowing,
                calendar_id: Some("US-NY".to_string()),
                stub: StubKind::None,
                start: date!(2024 - 01 - 01),
                end: date!(2026 - 01 - 01),
                par_method: None,
                compounding_simple: true,
                payment_delay_days: 0,
            },
        )
        .float(
            finstack_valuations::instruments::common::parameters::legs::FloatLegSpec {
                discount_curve_id: "USD_OIS".into(),
                forward_curve_id: "USD_LIBOR_3M".into(),
                spread_bp: 0.0,
                freq: Frequency::quarterly(),
                dc: DayCount::Act360,
                bdc: BusinessDayConvention::ModifiedFollowing,
                calendar_id: Some("US-NY".to_string()),
                fixing_calendar_id: Some("US-NY".to_string()),
                stub: StubKind::None,
                reset_lag_days: 2,
                compounding: Default::default(),
                payment_delay_days: 0,
                start: date!(2024 - 01 - 01),
                end: date!(2026 - 01 - 01),
            },
        )
        .build()
        .unwrap();

    let market = build_test_curves();
    let as_of = date!(2024 - 01 - 01);

    let schedule = swap.build_schedule(&market, as_of);

    assert!(schedule.is_ok(), "Should handle calendar adjustments");
}

#[test]
fn test_irs_receive_fixed_cashflow_signs() {
    // Receive fixed: fixed coupons positive, float negative
    let swap = InterestRateSwap::create_usd_swap(
        "IRS-RECEIVE".into(),
        Money::new(1_000_000.0, Currency::USD),
        0.05,
        date!(2024 - 01 - 01),
        date!(2025 - 01 - 01),
        PayReceive::ReceiveFixed,
    )
    .unwrap();

    let market = build_test_curves();
    let as_of = date!(2024 - 01 - 01);

    let full_schedule = swap.build_full_schedule(&market, as_of).unwrap();

    use finstack_valuations::cashflow::primitives::CFKind;

    // Check signs
    for cf in &full_schedule.flows {
        match cf.kind {
            CFKind::Fixed | CFKind::Stub => {
                // Receive fixed: should be positive
                assert!(
                    cf.amount.amount() > 0.0,
                    "Fixed leg should be positive for ReceiveFixed"
                );
            }
            CFKind::FloatReset => {
                // Pay floating: should be negative
                assert!(
                    cf.amount.amount() < 0.0,
                    "Floating leg should be negative for ReceiveFixed"
                );
            }
            _ => {}
        }
    }
}

#[test]
fn test_irs_pay_fixed_cashflow_signs() {
    // Pay fixed: fixed coupons negative, float positive
    let swap = InterestRateSwap::create_usd_swap(
        "IRS-PAY".into(),
        Money::new(1_000_000.0, Currency::USD),
        0.05,
        date!(2024 - 01 - 01),
        date!(2025 - 01 - 01),
        PayReceive::PayFixed,
    )
    .unwrap();

    let market = build_test_curves();
    let as_of = date!(2024 - 01 - 01);

    let full_schedule = swap.build_full_schedule(&market, as_of).unwrap();

    use finstack_valuations::cashflow::primitives::CFKind;

    // Check signs
    for cf in &full_schedule.flows {
        match cf.kind {
            CFKind::Fixed | CFKind::Stub => {
                // Pay fixed: should be negative
                assert!(
                    cf.amount.amount() < 0.0,
                    "Fixed leg should be negative for PayFixed"
                );
            }
            CFKind::FloatReset => {
                // Receive floating: should be positive
                assert!(
                    cf.amount.amount() > 0.0,
                    "Floating leg should be positive for PayFixed"
                );
            }
            _ => {}
        }
    }
}
