//! Edge case and boundary condition tests for revolving credit.

use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount, Tenor};
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_valuations::instruments::Instrument;
use finstack_valuations::instruments::fixed_income::revolving_credit::{
    BaseRateSpec, DrawRepaySpec, RevolvingCredit, RevolvingCreditFees,
};
use time::macros::date;

use crate::common::test_helpers::flat_discount_curve;

#[test]
fn test_zero_utilization() {
    // Arrange
    let as_of = date!(2025 - 01 - 01);
    let facility = RevolvingCredit::builder()
        .id("RC-EDGE-ZERO".into())
        .commitment_amount(Money::new(10_000_000.0, Currency::USD))
        .drawn_amount(Money::new(0.0, Currency::USD)) // Zero drawn
        .commitment_date(as_of)
        .maturity_date(date!(2030 - 01 - 01))
        .base_rate_spec(BaseRateSpec::Fixed { rate: 0.05 })
        .day_count(DayCount::Act360)
        .payment_frequency(Tenor::quarterly())
        .fees(RevolvingCreditFees::flat(25.0, 10.0, 5.0))
        .draw_repay_spec(DrawRepaySpec::Scheduled(vec![]))
        .discount_curve_id("USD-OIS".into())
        .build()
        .unwrap();

    let disc_curve = flat_discount_curve(0.04, as_of, "USD-OIS");
    let market = MarketContext::new().insert_discount(disc_curve);

    // Act
    let pv = facility.value(&market, as_of);

    // Assert
    assert!(pv.is_ok());
}

#[test]
fn test_full_utilization() {
    // Arrange
    let as_of = date!(2025 - 01 - 01);
    let facility = RevolvingCredit::builder()
        .id("RC-EDGE-FULL".into())
        .commitment_amount(Money::new(10_000_000.0, Currency::USD))
        .drawn_amount(Money::new(10_000_000.0, Currency::USD)) // 100% drawn
        .commitment_date(as_of)
        .maturity_date(date!(2030 - 01 - 01))
        .base_rate_spec(BaseRateSpec::Fixed { rate: 0.06 })
        .day_count(DayCount::Act360)
        .payment_frequency(Tenor::quarterly())
        .fees(RevolvingCreditFees::flat(30.0, 15.0, 10.0))
        .draw_repay_spec(DrawRepaySpec::Scheduled(vec![]))
        .discount_curve_id("USD-OIS".into())
        .build()
        .unwrap();

    let disc_curve = flat_discount_curve(0.04, as_of, "USD-OIS");
    let market = MarketContext::new().insert_discount(disc_curve);

    // Act
    let pv = facility.value(&market, as_of);

    // Assert
    assert!(pv.is_ok());
}

#[test]
fn test_very_short_commitment_period() {
    // Arrange
    let as_of = date!(2025 - 01 - 01);
    let facility = RevolvingCredit::builder()
        .id("RC-EDGE-SHORT".into())
        .commitment_amount(Money::new(10_000_000.0, Currency::USD))
        .drawn_amount(Money::new(5_000_000.0, Currency::USD))
        .commitment_date(as_of)
        .maturity_date(date!(2025 - 07 - 01)) // 6 months
        .base_rate_spec(BaseRateSpec::Fixed { rate: 0.05 })
        .day_count(DayCount::Act360)
        .payment_frequency(Tenor::quarterly())
        .fees(RevolvingCreditFees::flat(25.0, 10.0, 5.0))
        .draw_repay_spec(DrawRepaySpec::Scheduled(vec![]))
        .discount_curve_id("USD-OIS".into())
        .build()
        .unwrap();

    let disc_curve = flat_discount_curve(0.04, as_of, "USD-OIS");
    let market = MarketContext::new().insert_discount(disc_curve);

    // Act
    let pv = facility.value(&market, as_of);

    // Assert
    assert!(pv.is_ok());
}

#[test]
fn test_very_long_commitment_period() {
    // Arrange
    let as_of = date!(2025 - 01 - 01);
    let facility = RevolvingCredit::builder()
        .id("RC-EDGE-LONG".into())
        .commitment_amount(Money::new(10_000_000.0, Currency::USD))
        .drawn_amount(Money::new(5_000_000.0, Currency::USD))
        .commitment_date(as_of)
        .maturity_date(date!(2040 - 01 - 01)) // 15 years
        .base_rate_spec(BaseRateSpec::Fixed { rate: 0.06 })
        .day_count(DayCount::Act360)
        .payment_frequency(Tenor::quarterly())
        .fees(RevolvingCreditFees::flat(30.0, 12.0, 8.0))
        .draw_repay_spec(DrawRepaySpec::Scheduled(vec![]))
        .discount_curve_id("USD-OIS".into())
        .build()
        .unwrap();

    let disc_curve = flat_discount_curve(0.04, as_of, "USD-OIS");
    let market = MarketContext::new().insert_discount(disc_curve);

    // Act
    let pv = facility.value(&market, as_of);

    // Assert
    assert!(pv.is_ok());
}
