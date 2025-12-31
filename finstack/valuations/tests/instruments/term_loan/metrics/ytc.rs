//! Yield to call tests.

use finstack_core::currency::Currency;
use finstack_core::dates::{BusinessDayConvention, DayCount, StubKind, Tenor};
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::CurveId;
use finstack_valuations::cashflow::builder::specs::CouponType;
use finstack_valuations::instruments::Instrument;
use finstack_valuations::instruments::fixed_income::term_loan::{
    AmortizationSpec, LoanCall, LoanCallSchedule, RateSpec, TermLoan,
};
use finstack_valuations::metrics::MetricId;
use time::macros::date;

use crate::common::test_helpers::flat_discount_curve;

#[test]
fn test_ytc_callable_loan() {
    // Arrange
    let as_of = date!(2025 - 01 - 01);
    let mut loan = TermLoan::builder()
        .id("TL-YTC-001".into())
        .currency(Currency::USD)
        .notional_limit(Money::new(10_000_000.0, Currency::USD))
        .issue(as_of)
        .maturity(date!(2030 - 01 - 01))
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

    loan.call_schedule = Some(LoanCallSchedule {
        calls: vec![LoanCall {
            date: date!(2027 - 01 - 01),
            price_pct_of_par: 101.0,
        }],
    });

    let disc_curve = flat_discount_curve(0.05, as_of, "USD-OIS");
    let market = MarketContext::new().insert_discount(disc_curve);

    // Act
    let result = loan.price_with_metrics(&market, as_of, &[MetricId::custom("ytc")]);

    // Assert
    assert!(result.is_ok());
    let result = result.unwrap();
    let ytc = *result.measures.get("ytc").unwrap();

    assert!(ytc.is_finite() && ytc > 0.0);
}
