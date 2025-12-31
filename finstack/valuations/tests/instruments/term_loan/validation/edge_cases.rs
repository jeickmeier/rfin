//! Edge case and boundary condition tests.

use finstack_core::currency::Currency;
use finstack_core::dates::{BusinessDayConvention, DayCount, StubKind, Tenor};
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::CurveId;
use finstack_valuations::cashflow::builder::specs::CouponType;
use finstack_valuations::instruments::Instrument;
use finstack_valuations::instruments::fixed_income::term_loan::{AmortizationSpec, RateSpec, TermLoan};
use time::macros::date;

use crate::common::test_helpers::flat_discount_curve;

#[test]
fn test_zero_coupon_loan() {
    // Arrange
    let as_of = date!(2025 - 01 - 01);
    let loan = TermLoan::builder()
        .id("TL-ZERO".into())
        .currency(Currency::USD)
        .notional_limit(Money::new(10_000_000.0, Currency::USD))
        .issue(as_of)
        .maturity(date!(2030 - 01 - 01))
        .rate(RateSpec::Fixed { rate_bp: 0 }) // Zero coupon
        .pay_freq(Tenor::semi_annual())
        .day_count(DayCount::Act360)
        .bdc(BusinessDayConvention::ModifiedFollowing)
        .calendar_id_opt(None)
        .stub(StubKind::None)
        .discount_curve_id(CurveId::from("USD-OIS"))
        .amortization(AmortizationSpec::None)
        .coupon_type(CouponType::Cash)
        .upfront_fee_opt(None)
        .ddtl_opt(None)
        .covenants_opt(None)
        .pricing_overrides(Default::default())
        .attributes(Default::default())
        .build()
        .unwrap();

    let disc_curve = flat_discount_curve(0.05, as_of, "USD-OIS");
    let market = MarketContext::new().insert_discount(disc_curve);

    // Act
    let pv = loan.value(&market, as_of);

    // Assert
    assert!(pv.is_ok());
    let pv = pv.unwrap();
    // Should be deeply discounted
    assert!(pv.amount() < 10_000_000.0);
}

#[test]
fn test_very_short_maturity() {
    // Arrange
    let as_of = date!(2025 - 01 - 01);
    let loan = TermLoan::builder()
        .id("TL-SHORT".into())
        .currency(Currency::USD)
        .notional_limit(Money::new(10_000_000.0, Currency::USD))
        .issue(as_of)
        .maturity(date!(2025 - 07 - 01)) // 6 months
        .rate(RateSpec::Fixed { rate_bp: 500 })
        .pay_freq(Tenor::quarterly())
        .day_count(DayCount::Act360)
        .bdc(BusinessDayConvention::ModifiedFollowing)
        .calendar_id_opt(None)
        .stub(StubKind::None)
        .discount_curve_id(CurveId::from("USD-OIS"))
        .amortization(AmortizationSpec::None)
        .coupon_type(CouponType::Cash)
        .upfront_fee_opt(None)
        .ddtl_opt(None)
        .covenants_opt(None)
        .pricing_overrides(Default::default())
        .attributes(Default::default())
        .build()
        .unwrap();

    let disc_curve = flat_discount_curve(0.05, as_of, "USD-OIS");
    let market = MarketContext::new().insert_discount(disc_curve);

    // Act
    let pv = loan.value(&market, as_of);

    // Assert
    assert!(pv.is_ok());
}

#[test]
fn test_very_long_maturity() {
    // Arrange
    let as_of = date!(2025 - 01 - 01);
    let loan = TermLoan::builder()
        .id("TL-LONG".into())
        .currency(Currency::USD)
        .notional_limit(Money::new(10_000_000.0, Currency::USD))
        .issue(as_of)
        .maturity(date!(2055 - 01 - 01)) // 30 years
        .rate(RateSpec::Fixed { rate_bp: 600 })
        .pay_freq(Tenor::semi_annual())
        .day_count(DayCount::Act360)
        .bdc(BusinessDayConvention::ModifiedFollowing)
        .calendar_id_opt(None)
        .stub(StubKind::None)
        .discount_curve_id(CurveId::from("USD-OIS"))
        .amortization(AmortizationSpec::None)
        .coupon_type(CouponType::Cash)
        .upfront_fee_opt(None)
        .ddtl_opt(None)
        .covenants_opt(None)
        .pricing_overrides(Default::default())
        .attributes(Default::default())
        .build()
        .unwrap();

    let disc_curve = flat_discount_curve(0.05, as_of, "USD-OIS");
    let market = MarketContext::new().insert_discount(disc_curve);

    // Act
    let pv = loan.value(&market, as_of);

    // Assert
    assert!(pv.is_ok());
}

#[test]
fn test_negative_rate_environment() {
    // Arrange
    let as_of = date!(2025 - 01 - 01);
    let loan = TermLoan::builder()
        .id("TL-NEGRATE".into())
        .currency(Currency::USD)
        .notional_limit(Money::new(10_000_000.0, Currency::USD))
        .issue(as_of)
        .maturity(date!(2030 - 01 - 01))
        .rate(RateSpec::Fixed { rate_bp: 500 })
        .pay_freq(Tenor::semi_annual())
        .day_count(DayCount::Act360)
        .bdc(BusinessDayConvention::ModifiedFollowing)
        .calendar_id_opt(None)
        .stub(StubKind::None)
        .discount_curve_id(CurveId::from("USD-OIS"))
        .amortization(AmortizationSpec::None)
        .coupon_type(CouponType::Cash)
        .upfront_fee_opt(None)
        .ddtl_opt(None)
        .covenants_opt(None)
        .pricing_overrides(Default::default())
        .attributes(Default::default())
        .build()
        .unwrap();

    // Negative discount rate
    let disc_curve = flat_discount_curve(-0.01, as_of, "USD-OIS");
    let market = MarketContext::new().insert_discount(disc_curve);

    // Act
    let pv = loan.value(&market, as_of);

    // Assert
    assert!(pv.is_ok());
    let pv = pv.unwrap();
    // Should be valued above par in negative rate environment
    assert!(pv.amount() > 10_000_000.0);
}
