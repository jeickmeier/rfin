//! Utilization fee metric tests.

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
fn test_utilization_fee_above_threshold() {
    // Arrange
    let as_of = date!(2025 - 01 - 01);
    let facility = RevolvingCredit::builder()
        .id("RC-UTIL-FEE".into())
        .commitment_amount(Money::new(10_000_000.0, Currency::USD))
        .drawn_amount(Money::new(8_000_000.0, Currency::USD)) // 80% > threshold
        .commitment_date(as_of)
        .maturity_date(date!(2026 - 01 - 01))
        .base_rate_spec(BaseRateSpec::Fixed { rate: 0.05 })
        .day_count(DayCount::Act360)
        .payment_frequency(Tenor::quarterly())
        .fees(RevolvingCreditFees::flat(25.0, 0.0, 20.0)) // Utilization fee
        .draw_repay_spec(DrawRepaySpec::Scheduled(vec![]))
        .discount_curve_id("USD-OIS".into())
        .build()
        .unwrap();

    let disc_curve = flat_discount_curve(0.03, as_of, "USD-OIS");
    let market = MarketContext::new().insert_discount(disc_curve);

    // Act
    let pv = facility.value(&market, as_of);

    // Assert
    assert!(pv.is_ok());
    // Should include utilization fee in valuation
}
