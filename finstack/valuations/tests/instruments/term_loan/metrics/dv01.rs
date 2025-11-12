//! DV01 (interest rate sensitivity) tests.

use finstack_core::currency::Currency;
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, Frequency, StubKind};
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::CurveId;
use finstack_valuations::cashflow::builder::specs::CouponType;
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::instruments::term_loan::{AmortizationSpec, RateSpec, TermLoan};
use finstack_valuations::metrics::MetricId;
use time::macros::date;

use crate::common::test_helpers::flat_discount_curve;

#[test]
fn test_dv01_positive_for_asset() {
    // Arrange
    let as_of = date!(2025 - 01 - 01);
    let loan = TermLoan::builder()
        .id("TL-DV01-001".into())
        .currency(Currency::USD)
        .notional_limit(Money::new(10_000_000.0, Currency::USD))
        .issue(as_of)
        .maturity(date!(2030 - 01 - 01))
        .rate(RateSpec::Fixed { rate_bp: 500 })
        .pay_freq(Frequency::semi_annual())
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
    let result = loan.price_with_metrics(&market, as_of, &[MetricId::Dv01]);

    // Assert
    assert!(result.is_ok());
    let result = result.unwrap();
    let dv01 = *result.measures.get("dv01").unwrap();
    
    // DV01 should be positive (value decreases when rates rise)
    assert!(dv01 > 0.0);
}

#[test]
fn test_dv01_increases_with_maturity() {
    // Arrange
    let as_of = date!(2025 - 01 - 01);
    
    // Short maturity loan
    let loan_short = TermLoan::builder()
        .id("TL-DV01-SHORT".into())
        .currency(Currency::USD)
        .notional_limit(Money::new(10_000_000.0, Currency::USD))
        .issue(as_of)
        .maturity(date!(2027 - 01 - 01)) // 2 years
        .rate(RateSpec::Fixed { rate_bp: 500 })
        .pay_freq(Frequency::semi_annual())
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

    // Long maturity loan
    let loan_long = TermLoan::builder()
        .id("TL-DV01-LONG".into())
        .currency(Currency::USD)
        .notional_limit(Money::new(10_000_000.0, Currency::USD))
        .issue(as_of)
        .maturity(date!(2035 - 01 - 01)) // 10 years
        .rate(RateSpec::Fixed { rate_bp: 500 })
        .pay_freq(Frequency::semi_annual())
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
    let result_short = loan_short.price_with_metrics(&market, as_of, &[MetricId::Dv01]).unwrap();
    let result_long = loan_long.price_with_metrics(&market, as_of, &[MetricId::Dv01]).unwrap();
    
    let dv01_short = *result_short.measures.get("dv01").unwrap();
    let dv01_long = *result_long.measures.get("dv01").unwrap();

    // Assert
    // Longer maturity should have higher DV01
    assert!(dv01_long > dv01_short);
}

