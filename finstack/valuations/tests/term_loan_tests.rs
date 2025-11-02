use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount, Frequency};
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::CurveId;
use finstack_valuations::instruments::term_loan::{self, DdtlSpec, DrawEvent, OidPolicy, TermLoan};
use finstack_valuations::pricer::Pricer;
use finstack_valuations::cashflow::traits::CashflowProvider;
use finstack_valuations::cashflow::primitives::CFKind;

fn mc() -> MarketContext {
    let as_of = Date::from_calendar_date(2025, time::Month::January, 1).unwrap();
    let disc = DiscountCurve::builder("USD-OIS")
        .base_date(as_of)
        .knots(vec![(0.0, 1.0), (1.0, 0.97), (5.0, 0.85)])
        .build()
        .unwrap();
    MarketContext::new().insert_discount(disc)
}

#[test]
fn term_loan_fixed_with_draws_and_fees() {
    let issue = Date::from_calendar_date(2025, time::Month::January, 1).unwrap();
    let maturity = Date::from_calendar_date(2027, time::Month::January, 1).unwrap();

    let loan = TermLoan::builder()
        .id("TL-001".into())
        .currency(Currency::USD)
        .notional_limit(Money::new(10_000_000.0, Currency::USD))
        .issue(issue)
        .maturity(maturity)
        .rate(term_loan::types::RateSpec::Fixed { rate_bp: 800 })
        .pay_freq(Frequency::quarterly())
        .day_count(DayCount::Act360)
        .bdc(finstack_core::dates::BusinessDayConvention::ModifiedFollowing)
        .calendar_id_opt(None)
        .stub(finstack_core::dates::StubKind::None)
        .discount_curve_id(CurveId::from("USD-OIS"))
        .amortization(term_loan::AmortizationSpec::None)
        .coupon_type(finstack_valuations::cashflow::builder::types::CouponType::Cash)
        .upfront_fee_opt(Some(Money::new(25_000.0, Currency::USD)))
        .ddtl_opt(Some(DdtlSpec {
            commitment_limit: Money::new(10_000_000.0, Currency::USD),
            availability_start: issue,
            availability_end: Date::from_calendar_date(2026, time::Month::January, 1).unwrap(),
            draws: vec![
                DrawEvent { date: issue, amount: Money::new(3_000_000.0, Currency::USD) },
                DrawEvent { date: Date::from_calendar_date(2025, time::Month::July, 1).unwrap(), amount: Money::new(2_000_000.0, Currency::USD) },
            ],
            commitment_step_downs: vec![],
            usage_fee_bp: 10,
            commitment_fee_bp: 25,
            fee_base: term_loan::CommitmentFeeBase::Undrawn,
            oid_policy: Some(OidPolicy::WithheldPct(100)), // 1% withheld OID on draws
        }))
        .covenants_opt(None)
        .pricing_overrides(Default::default())
        .attributes(Default::default())
        .build()
        .unwrap();

    let market = mc();
    let pricer = finstack_valuations::instruments::term_loan::pricing::TermLoanDiscountingPricer;
    let as_of = issue;

    // Ensure pricing runs and PV is finite
    let result = pricer.price_dyn(&loan, &market, as_of).expect("pricing ok");
    assert_eq!(result.instrument_id, "TL-001");
    assert!(result.value.amount().is_finite());

    // Build full schedule and verify flows exist and are ordered
    let sched = loan.build_full_schedule(&market, as_of).unwrap();
    assert!(!sched.flows.is_empty());
    let mut last = sched.flows[0].date;
    for cf in &sched.flows {
        assert!(cf.date >= last);
        last = cf.date;
    }
}

#[test]
fn term_loan_pik_toggle_and_cash_sweep() {
    let issue = Date::from_calendar_date(2025, time::Month::January, 1).unwrap();
    let maturity = Date::from_calendar_date(2026, time::Month::January, 1).unwrap();

    let cov = term_loan::CovenantSpec {
        margin_stepups: vec![],
        pik_toggles: vec![term_loan::PikToggle { date: Date::from_calendar_date(2025, time::Month::July, 1).unwrap(), enable_pik: true }],
        cash_sweeps: vec![term_loan::CashSweepEvent { date: Date::from_calendar_date(2025, time::Month::October, 1).unwrap(), amount: Money::new(500_000.0, Currency::USD) }],
        draw_stop_dates: vec![],
    };

    let loan = TermLoan::builder()
        .id("TL-PIK".into())
        .currency(Currency::USD)
        .notional_limit(Money::new(5_000_000.0, Currency::USD))
        .issue(issue)
        .maturity(maturity)
        .rate(term_loan::types::RateSpec::Fixed { rate_bp: 600 })
        .pay_freq(Frequency::quarterly())
        .day_count(DayCount::Act360)
        .bdc(finstack_core::dates::BusinessDayConvention::ModifiedFollowing)
        .calendar_id_opt(None)
        .stub(finstack_core::dates::StubKind::None)
        .discount_curve_id(CurveId::from("USD-OIS"))
        .amortization(term_loan::AmortizationSpec::None)
        .coupon_type(finstack_valuations::cashflow::builder::types::CouponType::Cash)
        .upfront_fee_opt(None)
        .ddtl_opt(None)
        .covenants_opt(Some(cov))
        .pricing_overrides(Default::default())
        .attributes(Default::default())
        .build()
        .unwrap();

    let market = mc();
    let sched = loan.build_full_schedule(&market, issue).unwrap();
    assert!(!sched.flows.is_empty());
    // Ensure a PIK flow exists on/after toggle
    let has_pik = sched.flows.iter().any(|cf| cf.kind == CFKind::PIK);
    assert!(has_pik);
}


