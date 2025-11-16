//! Interest Rate Swap construction and builder tests.
//!
//! Tests cover:
//! - Standard construction methods
//! - Builder pattern with validation
//! - Convention-based construction
//! - Edge cases and error handling

use finstack_core::currency::Currency;
use finstack_core::dates::{BusinessDayConvention, DayCount, Frequency, StubKind};
use finstack_core::money::Money;
use finstack_valuations::instruments::common::parameters::IRSConvention;
use finstack_valuations::instruments::irs::{InterestRateSwap, PayReceive};
use time::macros::date;

#[test]
fn test_irs_standard_construction() {
    // Standard USD swap using defaults
    let swap = InterestRateSwap::new(
        "IRS-5Y".into(),
        Money::new(1_000_000.0, Currency::USD),
        0.05,
        date!(2024 - 01 - 01),
        date!(2029 - 01 - 01),
        PayReceive::PayFixed,
    );

    assert_eq!(swap.id.as_str(), "IRS-5Y");
    assert_eq!(swap.notional.amount(), 1_000_000.0);
    assert_eq!(swap.fixed.rate, 0.05);
    assert_eq!(swap.side, PayReceive::PayFixed);
    assert_eq!(swap.fixed.discount_curve_id.as_ref(), "USD-OIS");
    assert_eq!(swap.float.forward_curve_id.as_ref(), "USD-SOFR-3M");
}

#[test]
fn test_irs_builder_pattern() {
    // Use builder for full control
    let swap = InterestRateSwap::builder()
        .id("IRS-CUSTOM".into())
        .notional(Money::new(5_000_000.0, Currency::USD))
        .side(PayReceive::ReceiveFixed)
        .fixed(
            finstack_valuations::instruments::common::parameters::legs::FixedLegSpec {
                discount_curve_id: "USD_OIS".into(),
                rate: 0.0325,
                freq: Frequency::semi_annual(),
                dc: DayCount::Thirty360,
                bdc: BusinessDayConvention::ModifiedFollowing,
                calendar_id: Some("US-NY".to_string()),
                stub: StubKind::None,
                start: date!(2024 - 01 - 15),
                end: date!(2034 - 01 - 15),
                par_method: None,
                compounding_simple: true,
            },
        )
        .float(
            finstack_valuations::instruments::common::parameters::legs::FloatLegSpec {
                discount_curve_id: "USD_OIS".into(),
                forward_curve_id: "USD_LIBOR_3M".into(),
                spread_bp: 25.0,
                freq: Frequency::quarterly(),
                dc: DayCount::Act360,
                bdc: BusinessDayConvention::ModifiedFollowing,
                calendar_id: Some("US-NY".to_string()),
                stub: StubKind::None,
                reset_lag_days: 2,
            compounding: Default::default(),
                start: date!(2024 - 01 - 15),
                end: date!(2034 - 01 - 15),
            },
        )
        .build();

    let swap = swap.expect("Builder should succeed");

    assert_eq!(swap.id.as_str(), "IRS-CUSTOM");
    assert_eq!(swap.notional.amount(), 5_000_000.0);
    assert_eq!(swap.fixed.rate, 0.0325);
    assert_eq!(swap.float.spread_bp, 25.0);
    assert_eq!(swap.fixed.freq, Frequency::semi_annual());
    assert_eq!(swap.float.freq, Frequency::quarterly());
}

#[test]
fn test_irs_usd_convention() {
    let swap = InterestRateSwap::with_convention(
        "IRS-USD-10Y".into(),
        Money::new(10_000_000.0, Currency::USD),
        0.04,
        date!(2024 - 01 - 01),
        date!(2034 - 01 - 01),
        PayReceive::PayFixed,
        IRSConvention::USDStandard,
    );

    assert_eq!(swap.id.as_str(), "IRS-USD-10Y");
    assert_eq!(swap.notional.amount(), 10_000_000.0);
    assert_eq!(swap.fixed.rate, 0.04);

    // Verify USD standard conventions
    assert_eq!(swap.fixed.freq, Frequency::semi_annual());
    assert_eq!(swap.float.freq, Frequency::semi_annual());
    assert_eq!(swap.fixed.dc, DayCount::Thirty360);
    assert_eq!(swap.float.dc, DayCount::Act360);
    assert_eq!(swap.fixed.discount_curve_id.as_ref(), "USD-OIS");
    assert_eq!(swap.float.forward_curve_id.as_ref(), "USD-SOFR-3M");
}

#[test]
fn test_irs_eur_convention() {
    let swap = InterestRateSwap::with_convention(
        "IRS-EUR-5Y".into(),
        Money::new(5_000_000.0, Currency::EUR),
        0.02,
        date!(2024 - 01 - 01),
        date!(2029 - 01 - 01),
        PayReceive::ReceiveFixed,
        IRSConvention::EURStandard,
    );

    assert_eq!(swap.id.as_str(), "IRS-EUR-5Y");

    // Verify EUR standard conventions
    assert_eq!(swap.fixed.freq, Frequency::annual());
    assert_eq!(swap.float.freq, Frequency::semi_annual());
    assert_eq!(swap.fixed.dc, DayCount::Thirty360);
    assert_eq!(swap.float.dc, DayCount::Act360);
    assert_eq!(swap.fixed.discount_curve_id.as_ref(), "EUR-ESTR");
    assert_eq!(swap.float.forward_curve_id.as_ref(), "EUR-EURIBOR-6M");
}

#[test]
fn test_irs_gbp_convention() {
    let swap = InterestRateSwap::with_convention(
        "IRS-GBP-7Y".into(),
        Money::new(3_000_000.0, Currency::GBP),
        0.035,
        date!(2024 - 01 - 01),
        date!(2031 - 01 - 01),
        PayReceive::PayFixed,
        IRSConvention::GBPStandard,
    );

    assert_eq!(swap.id.as_str(), "IRS-GBP-7Y");

    // Verify GBP standard conventions
    assert_eq!(swap.fixed.freq, Frequency::semi_annual());
    assert_eq!(swap.float.freq, Frequency::semi_annual());
    assert_eq!(swap.fixed.dc, DayCount::Act365F);
    assert_eq!(swap.float.dc, DayCount::Act365F);
    assert_eq!(swap.fixed.discount_curve_id.as_ref(), "GBP-SONIA");
    assert_eq!(swap.float.forward_curve_id.as_ref(), "GBP-SONIA");
}

#[test]
fn test_irs_basis_swap() {
    // Basis swap: float vs float with different indices
    let swap = InterestRateSwap::usd_basis_swap(
        "BASIS-3M-6M".into(),
        Money::new(1_000_000.0, Currency::USD),
        date!(2024 - 01 - 01),
        date!(2029 - 01 - 01),
        15.0, // 15bp spread on primary leg
        10.0, // 10bp spread on reference leg
    );

    assert_eq!(swap.id.as_str(), "BASIS-3M-6M");
    assert_eq!(swap.notional.amount(), 1_000_000.0);

    // Primary spread carried in fixed leg
    assert_eq!(swap.fixed.rate, 15.0 * 1e-4);

    // Reference spread on float leg
    assert_eq!(swap.float.spread_bp, 10.0);
    assert_eq!(swap.float.forward_curve_id.as_ref(), "USD-SOFR-6M");
}

#[test]
fn test_irs_receive_vs_pay() {
    let start = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);
    let notional = Money::new(1_000_000.0, Currency::USD);
    let rate = 0.05;

    let swap_receive = InterestRateSwap::new(
        "IRS-RECEIVE".into(),
        notional,
        rate,
        start,
        end,
        PayReceive::ReceiveFixed,
    );

    let swap_pay = InterestRateSwap::new(
        "IRS-PAY".into(),
        notional,
        rate,
        start,
        end,
        PayReceive::PayFixed,
    );

    assert_eq!(swap_receive.side, PayReceive::ReceiveFixed);
    assert_eq!(swap_pay.side, PayReceive::PayFixed);

    // Same parameters except direction
    assert_eq!(swap_receive.fixed.rate, swap_pay.fixed.rate);
    assert_eq!(swap_receive.notional, swap_pay.notional);
}

#[test]
fn test_irs_short_maturity() {
    // 6-month swap
    let swap = InterestRateSwap::new(
        "IRS-6M".into(),
        Money::new(1_000_000.0, Currency::USD),
        0.05,
        date!(2024 - 01 - 01),
        date!(2024 - 07 - 01),
        PayReceive::PayFixed,
    );

    assert_eq!(swap.id.as_str(), "IRS-6M");
    assert!(swap.fixed.end.year() - swap.fixed.start.year() < 1);
}

#[test]
fn test_irs_long_maturity() {
    // 30-year swap
    let swap = InterestRateSwap::new(
        "IRS-30Y".into(),
        Money::new(10_000_000.0, Currency::USD),
        0.04,
        date!(2024 - 01 - 01),
        date!(2054 - 01 - 01),
        PayReceive::ReceiveFixed,
    );

    assert_eq!(swap.id.as_str(), "IRS-30Y");
    assert_eq!(swap.fixed.end.year() - swap.fixed.start.year(), 30);
}

#[test]
fn test_irs_zero_spread() {
    let swap = InterestRateSwap::new(
        "IRS-ZERO-SPREAD".into(),
        Money::new(1_000_000.0, Currency::USD),
        0.05,
        date!(2024 - 01 - 01),
        date!(2029 - 01 - 01),
        PayReceive::ReceiveFixed,
    );

    assert_eq!(swap.float.spread_bp, 0.0);
}

#[test]
fn test_irs_with_spread() {
    let mut swap = InterestRateSwap::new(
        "IRS-WITH-SPREAD".into(),
        Money::new(1_000_000.0, Currency::USD),
        0.05,
        date!(2024 - 01 - 01),
        date!(2029 - 01 - 01),
        PayReceive::ReceiveFixed,
    );

    swap.float.spread_bp = 50.0;

    assert_eq!(swap.float.spread_bp, 50.0);
}

#[test]
fn test_irs_large_notional() {
    // Test with large notional typical of institutional trades
    let swap = InterestRateSwap::new(
        "IRS-LARGE".into(),
        Money::new(1_000_000_000.0, Currency::USD), // $1B
        0.045,
        date!(2024 - 01 - 01),
        date!(2034 - 01 - 01),
        PayReceive::PayFixed,
    );

    assert_eq!(swap.notional.amount(), 1_000_000_000.0);
}

#[test]
fn test_irs_small_notional() {
    // Test with small notional
    let swap = InterestRateSwap::new(
        "IRS-SMALL".into(),
        Money::new(10_000.0, Currency::USD),
        0.05,
        date!(2024 - 01 - 01),
        date!(2029 - 01 - 01),
        PayReceive::ReceiveFixed,
    );

    assert_eq!(swap.notional.amount(), 10_000.0);
}

#[test]
fn test_irs_different_leg_frequencies() {
    // Fixed semiannual, float quarterly (standard)
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
                end: date!(2029 - 01 - 01),
                par_method: None,
                compounding_simple: true,
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
                stub: StubKind::None,
                reset_lag_days: 2,
            compounding: Default::default(),
                start: date!(2024 - 01 - 01),
                end: date!(2029 - 01 - 01),
            },
        )
        .build()
        .unwrap();

    assert_eq!(swap.fixed.freq, Frequency::semi_annual());
    assert_eq!(swap.float.freq, Frequency::quarterly());
}

#[test]
fn test_irs_attribute_management() {
    let mut swap = InterestRateSwap::new(
        "IRS-ATTRS".into(),
        Money::new(1_000_000.0, Currency::USD),
        0.05,
        date!(2024 - 01 - 01),
        date!(2029 - 01 - 01),
        PayReceive::PayFixed,
    );

    // Add attributes
    swap.attributes
        .meta
        .insert("book".to_string(), "trading".to_string());
    swap.attributes
        .meta
        .insert("desk".to_string(), "rates".to_string());

    assert_eq!(
        swap.attributes.meta.get("book"),
        Some(&"trading".to_string())
    );
    assert_eq!(swap.attributes.meta.get("desk"), Some(&"rates".to_string()));
}

#[test]
fn test_irs_calendar_specification() {
    let swap = InterestRateSwap::builder()
        .id("IRS-CAL".into())
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
                end: date!(2029 - 01 - 01),
                par_method: None,
                compounding_simple: true,
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
                stub: StubKind::None,
                reset_lag_days: 2,
            compounding: Default::default(),
                start: date!(2024 - 01 - 01),
                end: date!(2029 - 01 - 01),
            },
        )
        .build()
        .unwrap();

    assert_eq!(swap.fixed.calendar_id, Some("US-NY".to_string()));
    assert_eq!(swap.float.calendar_id, Some("US-NY".to_string()));
}

#[test]
fn test_irs_stub_specification() {
    let swap = InterestRateSwap::builder()
        .id("IRS-STUB".into())
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
                end: date!(2029 - 01 - 01),
                par_method: None,
                compounding_simple: true,
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
                stub: StubKind::ShortFront,
                reset_lag_days: 2,
            compounding: Default::default(),
                start: date!(2024 - 01 - 15),
                end: date!(2029 - 01 - 01),
            },
        )
        .build()
        .unwrap();

    assert_eq!(swap.fixed.stub, StubKind::ShortFront);
    assert_eq!(swap.float.stub, StubKind::ShortFront);
}
