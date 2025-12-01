//! Yield to worst tests.

use finstack_core::currency::Currency;
use finstack_core::dates::{BusinessDayConvention, DayCount, Frequency, StubKind};
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::CurveId;
use finstack_valuations::cashflow::builder::specs::CouponType;
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::instruments::term_loan::{
    AmortizationSpec, LoanCall, LoanCallSchedule, RateSpec, TermLoan,
};
use finstack_valuations::metrics::MetricId;
use time::macros::date;

use crate::common::test_helpers::flat_discount_curve;

#[test]
fn test_ytw_is_minimum_of_ytm_and_ytc() {
    // Arrange
    let as_of = date!(2025 - 01 - 01);
    let mut loan = TermLoan::builder()
        .id("TL-YTW-001".into())
        .currency(Currency::USD)
        .notional_limit(Money::new(10_000_000.0, Currency::USD))
        .issue(as_of)
        .maturity(date!(2029 - 01 - 01))
        .rate(RateSpec::Fixed { rate_bp: 600 })
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

    loan.call_schedule = Some(LoanCallSchedule {
        calls: vec![LoanCall {
            date: date!(2027 - 01 - 01),
            price_pct_of_par: 101.0,
        }],
    });

    let disc_curve = flat_discount_curve(0.05, as_of, "USD-OIS");
    let market = MarketContext::new().insert_discount(disc_curve);

    // Act
    let result = loan.price_with_metrics(
        &market,
        as_of,
        &[MetricId::Ytm, MetricId::Ytw, MetricId::custom("ytc")],
    );

    // Assert
    assert!(result.is_ok());
    let result = result.unwrap();
    let ytm = *result.measures.get("ytm").unwrap();
    let ytc = *result.measures.get("ytc").unwrap();
    let ytw = *result.measures.get("ytw").unwrap();

    // YTW must be minimum of YTM and YTC
    assert!(ytw <= ytm + 1e-12);
    assert!(ytw <= ytc + 1e-12);
}
