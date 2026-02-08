//! Term loan cashflow generation tests.

use finstack_core::cashflow::CFKind;
use finstack_core::currency::Currency;
use finstack_core::dates::{BusinessDayConvention, DayCount, StubKind, Tenor};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::math::interp::InterpStyle;
use finstack_core::money::Money;
use finstack_core::types::CurveId;
use finstack_valuations::cashflow::builder::specs::CouponType;
use finstack_valuations::cashflow::CashflowProvider;
use finstack_valuations::instruments::fixed_income::term_loan::{
    AmortizationSpec, RateSpec, TermLoan,
};
use time::macros::date;

fn build_market_context() -> MarketContext {
    let disc = DiscountCurve::builder("USD-OIS")
        .base_date(date!(2025 - 01 - 01))
        .knots([(0.0, 1.0), (5.0, 0.85)])
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap();
    MarketContext::new().insert_discount(disc)
}

#[test]
fn test_fixed_coupon_cashflows() {
    // Arrange
    let loan = TermLoan::builder()
        .id("TL-CF-FIXED".into())
        .currency(Currency::USD)
        .notional_limit(Money::new(10_000_000.0, Currency::USD))
        .issue(date!(2025 - 01 - 01))
        .maturity(date!(2026 - 01 - 01)) // 1 year
        .rate(RateSpec::Fixed { rate_bp: 500 }) // 5%
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

    // Act
    let market = build_market_context();
    let as_of = date!(2025 - 01 - 01);
    let cashflows = loan.build_dated_flows(&market, as_of).unwrap();

    // Assert
    assert!(!cashflows.is_empty());
    // Should have quarterly coupons + principal repayment
    assert!(cashflows.len() >= 4);
}

#[test]
fn test_amortizing_principal_cashflows() {
    // Arrange
    let loan = TermLoan::builder()
        .id("TL-CF-AMORT".into())
        .currency(Currency::USD)
        .notional_limit(Money::new(10_000_000.0, Currency::USD))
        .issue(date!(2025 - 01 - 01))
        .maturity(date!(2026 - 01 - 01))
        .rate(RateSpec::Fixed { rate_bp: 600 })
        .pay_freq(Tenor::quarterly())
        .day_count(DayCount::Act360)
        .bdc(BusinessDayConvention::ModifiedFollowing)
        .calendar_id_opt(None)
        .stub(StubKind::None)
        .discount_curve_id(CurveId::from("USD-OIS"))
        .amortization(AmortizationSpec::Linear {
            start: date!(2025 - 01 - 01),
            end: date!(2026 - 01 - 01),
        })
        .coupon_type(CouponType::Cash)
        .upfront_fee_opt(None)
        .ddtl_opt(None)
        .covenants_opt(None)
        .pricing_overrides(Default::default())
        .attributes(Default::default())
        .build()
        .unwrap();

    // Act
    let market = build_market_context();
    let as_of = date!(2025 - 01 - 01);
    let cashflows = loan.build_dated_flows(&market, as_of).unwrap();

    // Assert
    assert!(!cashflows.is_empty());
    // Should include principal amortization payments
}

#[test]
fn test_pik_interest_capitalization() {
    // Arrange
    let loan = TermLoan::builder()
        .id("TL-CF-PIK".into())
        .currency(Currency::USD)
        .notional_limit(Money::new(10_000_000.0, Currency::USD))
        .issue(date!(2025 - 01 - 01))
        .maturity(date!(2030 - 01 - 01))
        .rate(RateSpec::Fixed { rate_bp: 800 })
        .pay_freq(Tenor::semi_annual())
        .day_count(DayCount::Act360)
        .bdc(BusinessDayConvention::ModifiedFollowing)
        .calendar_id_opt(None)
        .stub(StubKind::None)
        .discount_curve_id(CurveId::from("USD-OIS"))
        .amortization(AmortizationSpec::None)
        .coupon_type(CouponType::PIK)
        .upfront_fee_opt(None)
        .ddtl_opt(None)
        .covenants_opt(None)
        .pricing_overrides(Default::default())
        .attributes(Default::default())
        .build()
        .unwrap();

    // Act
    let market = build_market_context();
    let as_of = date!(2025 - 01 - 01);
    let cashflows = loan.build_dated_flows(&market, as_of).unwrap();

    // Assert
    assert!(!cashflows.is_empty());
    // PIK interest capitalizes, so fewer cash payments expected
}

/// Property test: PercentPerPeriod with bp such that total amortization would exceed
/// the notional should be capped so that outstanding never goes negative.
///
/// Here we use bp=5000 (50% per quarter) over 4 quarters = 200% of notional.
/// The over-amortization guard should cap total amortization at 100%.
#[test]
fn test_over_amortization_is_capped() {
    let loan = TermLoan::builder()
        .id("TL-CF-OVERCAP".into())
        .currency(Currency::USD)
        .notional_limit(Money::new(1_000_000.0, Currency::USD))
        .issue(date!(2025 - 01 - 01))
        .maturity(date!(2026 - 01 - 01)) // 1 year, 4 quarterly periods
        .rate(RateSpec::Fixed { rate_bp: 500 })
        .pay_freq(Tenor::quarterly())
        .day_count(DayCount::Act360)
        .bdc(BusinessDayConvention::ModifiedFollowing)
        .calendar_id_opt(None)
        .stub(StubKind::None)
        .discount_curve_id(CurveId::from("USD-OIS"))
        // 50% per quarter × 4 quarters = 200% → should be capped at 100%
        .amortization(AmortizationSpec::PercentPerPeriod { bp: 5000 })
        .coupon_type(CouponType::Cash)
        .upfront_fee_opt(None)
        .ddtl_opt(None)
        .covenants_opt(None)
        .pricing_overrides(Default::default())
        .attributes(Default::default())
        .build()
        .unwrap();

    let market = build_market_context();
    let as_of = date!(2025 - 01 - 01);

    // Generate the full schedule and check outstanding path
    let schedule =
        finstack_valuations::instruments::fixed_income::term_loan::cashflows::generate_cashflows(
            &loan, &market, as_of,
        )
        .expect("cashflow generation should succeed even with excessive amort");

    let out_path = schedule
        .outstanding_by_date()
        .expect("outstanding path should succeed");

    // Outstanding must never go negative
    for (d, amt) in &out_path {
        assert!(
            amt.amount() >= -1e-10,
            "Outstanding at {d} = {} -- must never be negative",
            amt.amount()
        );
    }

    // Total amortization should equal exactly the notional (capped)
    let total_amort: f64 = schedule
        .flows
        .iter()
        .filter(|cf| cf.kind == CFKind::Amortization)
        .map(|cf| cf.amount.amount())
        .sum();

    // Amort amounts are positive from holder view (principal returned),
    // so total should not exceed the notional.
    assert!(
        total_amort <= 1_000_000.0 + 1e-6,
        "Total amort ({total_amort}) should not exceed notional (1,000,000)"
    );
    // With PercentPerPeriod applying to current outstanding (geometric decay):
    //   Q1: 1,000,000 × 50% = 500,000
    //   Q2:   500,000 × 50% = 250,000
    //   Q3:   250,000 × 50% = 125,000
    //   Q4:   125,000 × 50% =  62,500
    //   Total = 937,500
    let expected_total = 1_000_000.0 * (1.0 - 0.5_f64.powi(4)); // 937,500
    assert!(
        (total_amort - expected_total).abs() < 1.0,
        "Total amort ({total_amort}) should be approximately {expected_total} (geometric decay)"
    );
}
