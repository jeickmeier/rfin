//! Yield to maturity tests.

use finstack_core::currency::Currency;
use finstack_core::dates::{BusinessDayConvention, DayCount, Frequency, StubKind};
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
fn test_ytm_par_loan() {
    // Arrange
    let as_of = date!(2025 - 01 - 01);
    let loan = TermLoan::builder()
        .id("TL-YTM-PAR".into())
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
    let result = loan.price_with_metrics(&market, as_of, &[MetricId::Ytm]);

    // Assert
    assert!(result.is_ok());
    let result = result.unwrap();
    let ytm = *result.measures.get("ytm").unwrap();

    // YTM for par loan should approximately match coupon rate (5%)
    //
    // Sources of difference between YTM and coupon rate:
    // 1. Compounding convention mismatch:
    //    - Discount curve uses continuous compounding: DF = exp(-r*t)
    //    - XIRR uses annual compounding: DF = (1+r)^(-t)
    //    - For 5% rate: e^0.05 - 1 = 5.127% vs 5.0% → ~13bp difference
    //
    // 2. Act/360 day count effect:
    //    - Year fractions slightly exceed 1.0 for full years (365/360 ≈ 1.014)
    //    - This adds ~7bp to effective rate
    //
    // Total expected difference: ~20-30bp from par coupon rate
    assert!(ytm.is_finite() && ytm > 0.0);
    assert!(
        (ytm - 0.05).abs() < 0.003, // 30bp tolerance for documented compounding + day count effects
        "YTM {} should be close to coupon 0.05 (within 30bp)",
        ytm
    );
}

#[test]
fn test_ytm_discount_loan() {
    // Arrange
    let as_of = date!(2025 - 01 - 01);
    let loan = TermLoan::builder()
        .id("TL-YTM-DISC".into())
        .currency(Currency::USD)
        .notional_limit(Money::new(10_000_000.0, Currency::USD))
        .issue(as_of)
        .maturity(date!(2030 - 01 - 01))
        .rate(RateSpec::Fixed { rate_bp: 300 })
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

    let disc_curve = flat_discount_curve(0.06, as_of, "USD-OIS");
    let market = MarketContext::new().insert_discount(disc_curve);

    // Act
    let result = loan.price_with_metrics(&market, as_of, &[MetricId::Ytm]);

    // Assert
    assert!(result.is_ok());
    let result = result.unwrap();
    let ytm = *result.measures.get("ytm").unwrap();

    // YTM should be higher than coupon for discount loan
    assert!(ytm > 0.03);
}
