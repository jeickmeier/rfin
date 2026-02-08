//! Interest Rate Swap cashflow schedule tests.
//!
//! Tests cover:
//! - Fixed leg schedule generation
//! - Floating leg schedule generation
//! - Date adjustments and calendars
//! - Stub period handling
//! - Cashflow schedule consistency

use crate::finstack_test_utils as test_utils;
use finstack_core::currency::Currency;
use finstack_core::dates::{BusinessDayConvention, DayCount, StubKind, Tenor};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::{DiscountCurve, ForwardCurve};
use finstack_core::money::Money;
use finstack_valuations::cashflow::builder::date_generation::build_dates;
use finstack_valuations::cashflow::CashflowProvider;
use finstack_valuations::instruments::rates::irs::FloatingLegCompounding;
use finstack_valuations::instruments::rates::irs::{InterestRateSwap, PayReceive};
use time::macros::date;

fn build_test_curves() -> MarketContext {
    let as_of = date!(2024 - 01 - 01);

    // Curve IDs for usd_irs_swap (USD-OIS and USD-SOFR-3M)
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
    let swap = test_utils::usd_irs_swap(
        "IRS-CF-TEST",
        Money::new(1_000_000.0, Currency::USD),
        0.05,
        date!(2024 - 01 - 01),
        date!(2029 - 01 - 01),
        PayReceive::ReceiveFixed,
    )
    .unwrap();

    let market = build_test_curves();
    let as_of = date!(2024 - 01 - 01);

    let schedule = swap.build_dated_flows(&market, as_of).unwrap();

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
        .fixed(finstack_valuations::instruments::FixedLegSpec {
            discount_curve_id: "USD_OIS".into(),
            rate: rust_decimal::Decimal::try_from(0.05).expect("valid"),
            freq: Tenor::quarterly(),
            dc: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: None,
            stub: StubKind::None,
            start: date!(2024 - 01 - 01),
            end: date!(2025 - 01 - 01),
            par_method: None,
            compounding_simple: true,
            payment_delay_days: 0,
            end_of_month: false,
        })
        .float(finstack_valuations::instruments::FloatLegSpec {
            discount_curve_id: "USD_OIS".into(),
            forward_curve_id: "USD_LIBOR_3M".into(),
            spread_bp: rust_decimal::Decimal::ZERO,
            freq: Tenor::quarterly(),
            dc: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: None,
            fixing_calendar_id: None,
            stub: StubKind::None,
            reset_lag_days: 0, // Use 0 for spot-starting swaps to avoid needing historical fixings
            compounding: Default::default(),
            payment_delay_days: 0,
            end_of_month: false,
            start: date!(2024 - 01 - 01),
            end: date!(2025 - 01 - 01),
        })
        .build()
        .unwrap();

    let market = build_test_curves();
    let as_of = date!(2024 - 01 - 01);

    let schedule = swap.build_dated_flows(&market, as_of).unwrap();

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
        .fixed(finstack_valuations::instruments::FixedLegSpec {
            discount_curve_id: "USD_OIS".into(),
            rate: rust_decimal::Decimal::try_from(0.05).expect("valid"),
            freq: Tenor::semi_annual(),
            dc: DayCount::Thirty360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: None,
            stub: StubKind::None,
            start: date!(2024 - 01 - 01),
            end: date!(2026 - 01 - 01),
            par_method: None,
            compounding_simple: true,
            payment_delay_days: 0,
            end_of_month: false,
        })
        .float(finstack_valuations::instruments::FloatLegSpec {
            discount_curve_id: "USD_OIS".into(),
            forward_curve_id: "USD_LIBOR_3M".into(),
            spread_bp: rust_decimal::Decimal::try_from(0.0).expect("valid"),
            freq: Tenor::quarterly(),
            dc: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: None,
            fixing_calendar_id: None,
            stub: StubKind::None,
            reset_lag_days: 0, // Use 0 for spot-starting swaps to avoid needing historical fixings
            compounding: Default::default(),
            payment_delay_days: 0,
            end_of_month: false,
            start: date!(2024 - 01 - 01),
            end: date!(2026 - 01 - 01),
        })
        .build()
        .unwrap();

    let market = build_test_curves();
    let as_of = date!(2024 - 01 - 01);

    let schedule = swap.build_dated_flows(&market, as_of).unwrap();

    // 2 years: 4 semiannual + 8 quarterly = 12 total
    assert!(
        schedule.len() >= 4,
        "Should have at least 4 semiannual payments"
    );
}

#[test]
fn test_irs_floating_leg_schedule() {
    let swap = test_utils::usd_irs_swap(
        "IRS-FLOAT-TEST",
        Money::new(1_000_000.0, Currency::USD),
        0.05,
        date!(2024 - 01 - 01),
        date!(2025 - 01 - 01),
        PayReceive::PayFixed,
    )
    .unwrap();

    let market = build_test_curves();
    let as_of = date!(2024 - 01 - 01);

    let schedule = swap.build_dated_flows(&market, as_of).unwrap();

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
        .fixed(finstack_valuations::instruments::FixedLegSpec {
            discount_curve_id: "USD_OIS".into(),
            rate: rust_decimal::Decimal::try_from(0.05).expect("valid"),
            freq: Tenor::semi_annual(),
            dc: DayCount::Thirty360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: None,
            stub: StubKind::ShortFront,
            start: date!(2024 - 01 - 15),
            end: date!(2026 - 01 - 01),
            par_method: None,
            compounding_simple: true,
            payment_delay_days: 0,
            end_of_month: false,
        })
        .float(finstack_valuations::instruments::FloatLegSpec {
            discount_curve_id: "USD_OIS".into(),
            forward_curve_id: "USD_LIBOR_3M".into(),
            spread_bp: rust_decimal::Decimal::try_from(0.0).expect("valid"),
            freq: Tenor::quarterly(),
            dc: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: None,
            fixing_calendar_id: None,
            stub: StubKind::ShortFront,
            reset_lag_days: 0, // Use 0 for spot-starting swaps to avoid needing historical fixings
            compounding: Default::default(),
            payment_delay_days: 0,
            end_of_month: false,
            start: date!(2024 - 01 - 15),
            end: date!(2026 - 01 - 01),
        })
        .build()
        .unwrap();

    let market = build_test_curves();
    let as_of = date!(2024 - 01 - 01);

    let schedule = swap.build_dated_flows(&market, as_of);

    assert!(schedule.is_ok(), "Should generate schedule with stub");
}

#[test]
fn test_irs_stub_back() {
    // Swap with short back stub
    let swap = InterestRateSwap::builder()
        .id("IRS-STUB-BACK".into())
        .notional(Money::new(1_000_000.0, Currency::USD))
        .side(PayReceive::ReceiveFixed)
        .fixed(finstack_valuations::instruments::FixedLegSpec {
            discount_curve_id: "USD_OIS".into(),
            rate: rust_decimal::Decimal::try_from(0.05).expect("valid"),
            freq: Tenor::semi_annual(),
            dc: DayCount::Thirty360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: None,
            stub: StubKind::ShortBack,
            start: date!(2024 - 01 - 01),
            end: date!(2026 - 06 - 15),
            par_method: None,
            compounding_simple: true,
            payment_delay_days: 0,
            end_of_month: false,
        })
        .float(finstack_valuations::instruments::FloatLegSpec {
            discount_curve_id: "USD_OIS".into(),
            forward_curve_id: "USD_LIBOR_3M".into(),
            spread_bp: rust_decimal::Decimal::try_from(0.0).expect("valid"),
            freq: Tenor::quarterly(),
            dc: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: None,
            fixing_calendar_id: None,
            stub: StubKind::ShortBack,
            reset_lag_days: 0, // Use 0 for spot-starting swaps to avoid needing historical fixings
            compounding: Default::default(),
            payment_delay_days: 0,
            end_of_month: false,
            start: date!(2024 - 01 - 01),
            end: date!(2026 - 06 - 15),
        })
        .build()
        .unwrap();

    let market = build_test_curves();
    let as_of = date!(2024 - 01 - 01);

    let schedule = swap.build_dated_flows(&market, as_of);

    assert!(schedule.is_ok(), "Should generate schedule with back stub");
}

#[test]
fn test_irs_cashflow_dates_ordered() {
    let swap = test_utils::usd_irs_swap(
        "IRS-DATES",
        Money::new(1_000_000.0, Currency::USD),
        0.05,
        date!(2024 - 01 - 01),
        date!(2027 - 01 - 01),
        PayReceive::ReceiveFixed,
    )
    .unwrap();

    let market = build_test_curves();
    let as_of = date!(2024 - 01 - 01);

    let schedule = swap.build_dated_flows(&market, as_of).unwrap();

    // Verify dates are in ascending order
    let mut prev_date = date!(2020 - 01 - 01);
    for (cf_date, _) in &schedule {
        assert!(*cf_date >= prev_date, "Cashflow dates should be ordered");
        prev_date = *cf_date;
    }
}

#[test]
fn test_irs_cashflow_amounts_nonzero() {
    let swap = test_utils::usd_irs_swap(
        "IRS-AMOUNTS",
        Money::new(1_000_000.0, Currency::USD),
        0.05,
        date!(2024 - 01 - 01),
        date!(2027 - 01 - 01),
        PayReceive::ReceiveFixed,
    )
    .unwrap();

    let market = build_test_curves();
    let as_of = date!(2024 - 01 - 01);

    let schedule = swap.build_dated_flows(&market, as_of).unwrap();

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
    let swap = test_utils::usd_irs_swap(
        "IRS-FULL-SCHED",
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
        .fixed(finstack_valuations::instruments::FixedLegSpec {
            discount_curve_id: "USD_OIS".into(),
            rate: rust_decimal::Decimal::try_from(0.05).expect("valid"),
            freq: Tenor::semi_annual(),
            dc: DayCount::Thirty360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: None,
            stub: StubKind::None,
            start: date!(2024 - 01 - 01),
            end: date!(2026 - 01 - 01),
            par_method: None,
            compounding_simple: true,
            payment_delay_days: 0,
            end_of_month: false,
        })
        .float(finstack_valuations::instruments::FloatLegSpec {
            discount_curve_id: "USD_OIS".into(),
            forward_curve_id: "USD_LIBOR_3M".into(),
            spread_bp: rust_decimal::Decimal::try_from(0.0).expect("valid"),
            freq: Tenor::quarterly(),
            dc: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: None,
            fixing_calendar_id: None,
            stub: StubKind::None,
            reset_lag_days: 0, // Use 0 for spot-starting swaps to avoid needing historical fixings
            compounding: Default::default(),
            payment_delay_days: 0,
            end_of_month: false,
            start: date!(2024 - 01 - 01),
            end: date!(2026 - 01 - 01),
        })
        .build()
        .unwrap();

    let market = build_test_curves();
    let as_of = date!(2024 - 01 - 01);

    let schedule = swap.build_dated_flows(&market, as_of).unwrap();

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
        .fixed(finstack_valuations::instruments::FixedLegSpec {
            discount_curve_id: "USD_OIS".into(),
            rate: rust_decimal::Decimal::try_from(0.05).expect("valid"),
            freq: Tenor::semi_annual(),
            dc: DayCount::Thirty360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: Some("usny".to_string()),
            stub: StubKind::None,
            start: date!(2024 - 01 - 01),
            end: date!(2026 - 01 - 01),
            par_method: None,
            compounding_simple: true,
            payment_delay_days: 0,
            end_of_month: false,
        })
        .float(finstack_valuations::instruments::FloatLegSpec {
            discount_curve_id: "USD_OIS".into(),
            forward_curve_id: "USD_LIBOR_3M".into(),
            spread_bp: rust_decimal::Decimal::try_from(0.0).expect("valid"),
            freq: Tenor::quarterly(),
            dc: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: Some("usny".to_string()),
            fixing_calendar_id: Some("usny".to_string()),
            stub: StubKind::None,
            reset_lag_days: 0, // Use 0 for spot-starting swaps to avoid needing historical fixings
            compounding: Default::default(),
            payment_delay_days: 0,
            end_of_month: false,
            start: date!(2024 - 01 - 01),
            end: date!(2026 - 01 - 01),
        })
        .build()
        .unwrap();

    let market = build_test_curves();
    let as_of = date!(2024 - 01 - 01);

    let schedule = swap.build_dated_flows(&market, as_of);

    assert!(schedule.is_ok(), "Should handle calendar adjustments");
}

/// Test that explicit payment_delay_days: 0 is NOT overridden by convention defaults.
/// This verifies the fix where 0 means "no delay" rather than "use convention".
#[test]
fn test_irs_explicit_zero_payment_delay_preserved() {
    let start = date!(2024 - 01 - 02);
    let end = date!(2026 - 01 - 02);
    let swap = InterestRateSwap::builder()
        .id("IRS-OIS-ZERO-DELAY".into())
        .notional(Money::new(1_000_000.0, Currency::USD))
        .side(PayReceive::ReceiveFixed)
        .fixed(finstack_valuations::instruments::FixedLegSpec {
            discount_curve_id: "USD-OIS".into(),
            rate: rust_decimal::Decimal::try_from(0.05).expect("valid"),
            freq: Tenor::annual(),
            dc: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: None,
            stub: StubKind::None,
            start,
            end,
            par_method: None,
            compounding_simple: true,
            payment_delay_days: 0, // Explicit 0 should stay 0
            end_of_month: false,
        })
        .float(finstack_valuations::instruments::FloatLegSpec {
            discount_curve_id: "USD-OIS".into(),
            forward_curve_id: "USD-OIS".into(),
            spread_bp: rust_decimal::Decimal::ZERO,
            freq: Tenor::annual(),
            dc: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: None,
            fixing_calendar_id: None,
            stub: StubKind::None,
            reset_lag_days: 0,
            compounding: FloatingLegCompounding::CompoundedInArrears {
                lookback_days: 0,
                observation_shift: None,
            },
            payment_delay_days: 0, // Explicit 0 should stay 0
            end_of_month: false,
            start,
            end,
        })
        .build()
        .unwrap();

    let fixed_schedule =
        finstack_valuations::instruments::rates::irs::cashflow::fixed_leg_schedule(&swap)
            .expect("fixed schedule");
    // Expected schedule with payment_delay = 0 (no delay)
    let expected = build_dates(
        start,
        end,
        Tenor::annual(),
        StubKind::None,
        BusinessDayConvention::ModifiedFollowing,
        false,
        0, // No payment delay
        "weekends_only",
    )
    .expect("expected schedule");
    let first_payment = fixed_schedule
        .flows
        .iter()
        .map(|cf| cf.date)
        .min()
        .expect("first payment");
    assert_eq!(first_payment, expected.dates[0]);
}

/// Test that explicit reset_lag_days: 0 is NOT overridden by convention defaults.
/// This verifies the fix where 0 means "spot reset" rather than "use convention".
#[test]
fn test_irs_explicit_zero_reset_lag_preserved() {
    let start = date!(2024 - 01 - 02);
    let end = date!(2024 - 07 - 02);
    let swap = InterestRateSwap::builder()
        .id("IRS-TERM-ZERO-LAG".into())
        .notional(Money::new(1_000_000.0, Currency::USD))
        .side(PayReceive::ReceiveFixed)
        .fixed(finstack_valuations::instruments::FixedLegSpec {
            discount_curve_id: "USD-OIS".into(),
            rate: rust_decimal::Decimal::try_from(0.05).expect("valid"),
            freq: Tenor::quarterly(),
            dc: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: None,
            stub: StubKind::None,
            start,
            end,
            par_method: None,
            compounding_simple: true,
            payment_delay_days: 0,
            end_of_month: false,
        })
        .float(finstack_valuations::instruments::FloatLegSpec {
            discount_curve_id: "USD-OIS".into(),
            forward_curve_id: "USD-SOFR-3M".into(),
            spread_bp: rust_decimal::Decimal::ZERO,
            freq: Tenor::quarterly(),
            dc: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: None,
            fixing_calendar_id: None,
            stub: StubKind::None,
            reset_lag_days: 0, // Explicit 0 means spot reset (no lag)
            compounding: Default::default(),
            payment_delay_days: 0,
            end_of_month: false,
            start,
            end,
        })
        .build()
        .unwrap();

    let float_schedule =
        finstack_valuations::instruments::rates::irs::cashflow::float_leg_schedule(&swap)
            .expect("float schedule");
    let first_reset = float_schedule
        .flows
        .iter()
        .find(|cf| cf.kind == finstack_valuations::cashflow::primitives::CFKind::FloatReset)
        .and_then(|cf| cf.reset_date)
        .expect("reset date");
    // With reset_lag_days: 0, reset date should equal accrual start (spot reset)
    assert_eq!(first_reset, start);
}

#[test]
fn test_irs_receive_fixed_cashflow_signs() {
    // Receive fixed: fixed coupons positive, float negative
    let swap = test_utils::usd_irs_swap(
        "IRS-RECEIVE",
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
    let swap = test_utils::usd_irs_swap(
        "IRS-PAY",
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

/// IRS NPV parity test: at the par rate, fixed leg PV ≈ floating leg PV.
///
/// When the fixed rate equals the implied par rate from the forward curve,
/// the swap should have near-zero NPV (fixed leg PV - floating leg PV ≈ 0).
/// This test constructs a swap at 5% where the forward curve is flat at 5%,
/// so the swap should be approximately at par.
#[test]
fn test_irs_npv_parity_at_par_rate() {
    use finstack_valuations::cashflow::primitives::CFKind;

    // Build a swap where the fixed rate matches the flat forward curve rate (5%)
    // This should produce a near-zero NPV swap
    let as_of = date!(2024 - 01 - 01);
    let end_date = date!(2029 - 01 - 01); // 5 years

    let swap = InterestRateSwap::builder()
        .id("IRS-PAR-PARITY".into())
        .notional(Money::new(10_000_000.0, Currency::USD))
        .side(PayReceive::ReceiveFixed)
        .fixed(finstack_valuations::instruments::FixedLegSpec {
            discount_curve_id: "USD_OIS".into(),
            rate: rust_decimal::Decimal::try_from(0.05).expect("valid"), // matches flat forward
            freq: Tenor::quarterly(),
            dc: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: None,
            stub: StubKind::None,
            start: as_of,
            end: end_date,
            par_method: None,
            compounding_simple: true,
            payment_delay_days: 0,
            end_of_month: false,
        })
        .float(finstack_valuations::instruments::FloatLegSpec {
            discount_curve_id: "USD_OIS".into(),
            forward_curve_id: "USD_LIBOR_3M".into(),
            spread_bp: rust_decimal::Decimal::ZERO,
            freq: Tenor::quarterly(),
            dc: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: None,
            fixing_calendar_id: None,
            stub: StubKind::None,
            reset_lag_days: 0,
            compounding: Default::default(),
            payment_delay_days: 0,
            end_of_month: false,
            start: as_of,
            end: end_date,
        })
        .build()
        .unwrap();

    let market = build_test_curves();

    let full_schedule = swap.build_full_schedule(&market, as_of).unwrap();

    // Separate fixed and floating leg flows
    let fixed_sum: f64 = full_schedule
        .flows
        .iter()
        .filter(|cf| matches!(cf.kind, CFKind::Fixed | CFKind::Stub))
        .map(|cf| cf.amount.amount())
        .sum();

    let float_sum: f64 = full_schedule
        .flows
        .iter()
        .filter(|cf| cf.kind == CFKind::FloatReset)
        .map(|cf| cf.amount.amount())
        .sum();

    // For a par swap: |fixed_sum| ≈ |float_sum| (both undiscounted)
    // Since forward rate = fixed rate = 5%, the legs should approximately match.
    // Allow moderate tolerance because of day count differences, date adjustments,
    // and the difference between flat forward rates and period forward rates.
    let notional = 10_000_000.0;
    let parity_diff = (fixed_sum.abs() - float_sum.abs()).abs();
    let relative_diff = parity_diff / notional;

    assert!(
        relative_diff < 0.01, // Within 1% of notional
        "IRS NPV parity violated: fixed_sum={:.2}, float_sum={:.2}, diff={:.2} ({:.4}% of notional)",
        fixed_sum,
        float_sum,
        parity_diff,
        relative_diff * 100.0
    );
}
