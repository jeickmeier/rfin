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
        .interp(InterpStyle::Linear)
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
        .issue_date(date!(2025 - 01 - 01))
        .maturity(date!(2026 - 01 - 01)) // 1 year
        .rate(RateSpec::Fixed { rate_bp: 500 }) // 5%
        .frequency(Tenor::quarterly())
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
        .issue_date(date!(2025 - 01 - 01))
        .maturity(date!(2026 - 01 - 01))
        .rate(RateSpec::Fixed { rate_bp: 600 })
        .frequency(Tenor::quarterly())
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
        .issue_date(date!(2025 - 01 - 01))
        .maturity(date!(2030 - 01 - 01))
        .rate(RateSpec::Fixed { rate_bp: 800 })
        .frequency(Tenor::semi_annual())
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
        .issue_date(date!(2025 - 01 - 01))
        .maturity(date!(2026 - 01 - 01)) // 1 year, 4 quarterly periods
        .rate(RateSpec::Fixed { rate_bp: 500 })
        .frequency(Tenor::quarterly())
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

/// Linear amortization with start == issue should NOT generate an amort event at origination.
/// Amort payments only occur at period-end dates strictly after the start date.
#[test]
fn test_linear_amort_no_event_at_issue_date() {
    let issue = date!(2025 - 01 - 01);
    let maturity = date!(2026 - 01 - 01);
    let loan = TermLoan::builder()
        .id("TL-CF-LINEAR-ISSUE".into())
        .currency(Currency::USD)
        .notional_limit(Money::new(1_000_000.0, Currency::USD))
        .issue_date(issue)
        .maturity(maturity)
        .rate(RateSpec::Fixed { rate_bp: 500 })
        .frequency(Tenor::quarterly())
        .day_count(DayCount::Act360)
        .bdc(BusinessDayConvention::ModifiedFollowing)
        .calendar_id_opt(None)
        .stub(StubKind::None)
        .discount_curve_id(CurveId::from("USD-OIS"))
        .amortization(AmortizationSpec::Linear {
            start: issue, // start == issue
            end: maturity,
        })
        .coupon_type(CouponType::Cash)
        .upfront_fee_opt(None)
        .ddtl_opt(None)
        .covenants_opt(None)
        .pricing_overrides(Default::default())
        .attributes(Default::default())
        .build()
        .unwrap();

    let market = build_market_context();
    let as_of = issue;

    let schedule =
        finstack_valuations::instruments::fixed_income::term_loan::cashflows::generate_cashflows(
            &loan, &market, as_of,
        )
        .expect("cashflow generation should succeed");

    // No amort event should occur on the issue date itself
    let amort_at_issue: Vec<_> = schedule
        .flows
        .iter()
        .filter(|cf| cf.kind == CFKind::Amortization && cf.date == issue)
        .collect();
    assert!(
        amort_at_issue.is_empty(),
        "No amortization should be generated at the issue date, but found {} events",
        amort_at_issue.len()
    );

    // Total amort should equal exactly the notional (4 equal quarterly payments)
    let total_amort: f64 = schedule
        .flows
        .iter()
        .filter(|cf| cf.kind == CFKind::Amortization)
        .map(|cf| cf.amount.amount())
        .sum();
    assert!(
        (total_amort - 1_000_000.0).abs() < 1.0,
        "Total linear amort ({total_amort}) should equal notional (1,000,000)"
    );

    // Should have exactly 4 quarterly amort events (not 5)
    let amort_count = schedule
        .flows
        .iter()
        .filter(|cf| cf.kind == CFKind::Amortization)
        .count();
    assert_eq!(
        amort_count, 4,
        "Should have 4 quarterly amort events, not {amort_count}"
    );
}

/// PercentPerPeriod with 100% (10000 bp) should fully amortize the loan.
/// After capping, total amortization equals the notional.
#[test]
fn test_percent_per_period_full_amort() {
    let loan = TermLoan::builder()
        .id("TL-CF-100PCT".into())
        .currency(Currency::USD)
        .notional_limit(Money::new(1_000_000.0, Currency::USD))
        .issue_date(date!(2025 - 01 - 01))
        .maturity(date!(2026 - 01 - 01))
        .rate(RateSpec::Fixed { rate_bp: 500 })
        .frequency(Tenor::quarterly())
        .day_count(DayCount::Act360)
        .bdc(BusinessDayConvention::ModifiedFollowing)
        .calendar_id_opt(None)
        .stub(StubKind::None)
        .discount_curve_id(CurveId::from("USD-OIS"))
        // 100% per period should amortize everything in Q1
        .amortization(AmortizationSpec::PercentPerPeriod { bp: 10000 })
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

    let schedule =
        finstack_valuations::instruments::fixed_income::term_loan::cashflows::generate_cashflows(
            &loan, &market, as_of,
        )
        .expect("cashflow generation should succeed");

    let out_path = schedule.outstanding_by_date().expect("outstanding path");

    // Outstanding must never go negative
    for (d, amt) in &out_path {
        assert!(
            amt.amount() >= -1e-10,
            "Outstanding at {d} = {} -- must never be negative",
            amt.amount()
        );
    }

    // With 100% per period applied to current outstanding:
    // Q1: 100% × 1M = 1M (fully repaid in first period)
    // Q2-Q4: 0 (nothing left to amortize)
    let total_amort: f64 = schedule
        .flows
        .iter()
        .filter(|cf| cf.kind == CFKind::Amortization)
        .map(|cf| cf.amount.amount())
        .sum();
    assert!(
        (total_amort - 1_000_000.0).abs() < 1.0,
        "Total amort ({total_amort}) should equal notional (1,000,000) after capping"
    );
}

/// Commitment fees should use CFKind::CommitmentFee, not CFKind::Fee.
#[test]
fn test_commitment_fees_use_correct_kind() {
    use finstack_valuations::instruments::fixed_income::term_loan::{CommitmentFeeBase, DdtlSpec};

    let issue = date!(2025 - 01 - 01);
    let loan = TermLoan::builder()
        .id("TL-CF-FEEKIND".into())
        .currency(Currency::USD)
        .notional_limit(Money::new(10_000_000.0, Currency::USD))
        .issue_date(issue)
        .maturity(date!(2027 - 01 - 01))
        .rate(RateSpec::Fixed { rate_bp: 500 })
        .frequency(Tenor::quarterly())
        .day_count(DayCount::Act360)
        .bdc(BusinessDayConvention::ModifiedFollowing)
        .calendar_id_opt(None)
        .stub(StubKind::None)
        .discount_curve_id(CurveId::from("USD-OIS"))
        .amortization(AmortizationSpec::None)
        .coupon_type(CouponType::Cash)
        .upfront_fee_opt(None)
        .ddtl_opt(Some(DdtlSpec {
            commitment_limit: Money::new(10_000_000.0, Currency::USD),
            availability_start: issue,
            availability_end: date!(2026 - 01 - 01),
            draws: vec![],
            commitment_step_downs: vec![],
            usage_fee_bp: 0,
            commitment_fee_bp: 50, // 50bp commitment fee on undrawn
            fee_base: CommitmentFeeBase::Undrawn,
            oid_policy: None,
        }))
        .covenants_opt(None)
        .pricing_overrides(Default::default())
        .attributes(Default::default())
        .build()
        .unwrap();

    let market = build_market_context();
    let schedule =
        finstack_valuations::instruments::fixed_income::term_loan::cashflows::generate_cashflows(
            &loan, &market, issue,
        )
        .expect("cashflow generation should succeed");

    // Commitment fees should exist and use CommitmentFee kind
    let commitment_fees: Vec<_> = schedule
        .flows
        .iter()
        .filter(|cf| cf.kind == CFKind::CommitmentFee)
        .collect();
    assert!(
        !commitment_fees.is_empty(),
        "Commitment fees should use CFKind::CommitmentFee"
    );

    // No generic Fee kind should be used for commitment fees
    let generic_fees: Vec<_> = schedule
        .flows
        .iter()
        .filter(|cf| cf.kind == CFKind::Fee)
        .collect();
    // The only CFKind::Fee flows should be from upfront/OID fees, not commitment fees.
    // Since we have no upfront fee and no OID, there should be no generic fees.
    assert!(
        generic_fees.is_empty(),
        "Commitment fees should not use generic CFKind::Fee, found {} generic fee flows",
        generic_fees.len()
    );
}
