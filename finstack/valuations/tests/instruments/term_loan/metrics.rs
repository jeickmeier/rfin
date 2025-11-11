use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount, Frequency};
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::CurveId;
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::instruments::term_loan::{self, LoanCall, LoanCallSchedule, TermLoan};
use finstack_valuations::metrics::MetricId;
use time::macros::date;

fn build_flat_discount_curve(rate: f64, base_date: Date, curve_id: &str) -> DiscountCurve {
    DiscountCurve::builder(curve_id)
        .base_date(base_date)
        .day_count(DayCount::Act360)
        .knots([
            (0.0, 1.0),
            (1.0, (-rate).exp()),
            (5.0, (-rate * 5.0).exp()),
            (10.0, (-rate * 10.0).exp()),
        ])
        .build()
        .unwrap()
}

fn build_simple_term_loan(as_of: Date, maturity: Date) -> TermLoan {
    TermLoan::builder()
        .id("TL-METRICS".into())
        .currency(Currency::USD)
        .notional_limit(Money::new(10_000_000.0, Currency::USD))
        .issue(as_of)
        .maturity(maturity)
        .rate(term_loan::types::RateSpec::Fixed { rate_bp: 600 }) // 6%
        .pay_freq(Frequency::semi_annual())
        .day_count(DayCount::Act360)
        .bdc(finstack_core::dates::BusinessDayConvention::ModifiedFollowing)
        .calendar_id_opt(None)
        .stub(finstack_core::dates::StubKind::None)
        .discount_curve_id(CurveId::from("USD-OIS"))
        .amortization(term_loan::AmortizationSpec::None)
        .coupon_type(finstack_valuations::cashflow::builder::specs::CouponType::Cash)
        .upfront_fee_opt(None)
        .ddtl_opt(None)
        .covenants_opt(None)
        .pricing_overrides(Default::default())
        .attributes(Default::default())
        .build()
        .unwrap()
}

#[test]
fn test_term_loan_yields_with_callability() {
    let as_of = date!(2025 - 01 - 01);
    let maturity = date!(2029 - 01 - 01); // 4Y
    let mut loan = build_simple_term_loan(as_of, maturity);

    // Add first call at 2027-01-01 at 101% of outstanding
    loan.call_schedule = Some(LoanCallSchedule {
        calls: vec![LoanCall { date: date!(2027 - 01 - 01), price_pct_of_par: 101.0 }],
    });

    let disc_curve = build_flat_discount_curve(0.05, as_of, "USD-OIS");
    let market = MarketContext::new().insert_discount(disc_curve);

    // Compute YTM, YTC, YTW
    let result = loan
        .price_with_metrics(
            &market,
            as_of,
            &[MetricId::Ytm, MetricId::Ytw, MetricId::custom("ytc")],
        )
        .unwrap();

    let ytm = *result.measures.get("ytm").unwrap();
    let ytc = *result.measures.get("ytc").unwrap();
    let ytw = *result.measures.get("ytw").unwrap();

    // Sanity: yields are finite and positive in this setup
    assert!(ytm.is_finite() && ytm > 0.0);
    assert!(ytc.is_finite() && ytc > 0.0);
    assert!(ytw.is_finite() && ytw > 0.0);

    // YTW must be the min of YTM and YTC (maturity vs first call)
    assert!(ytw <= ytm + 1e-12);
    assert!(ytw <= ytc + 1e-12);
}

#[test]
fn test_term_loan_yields_to_horizons() {
    let as_of = date!(2025 - 01 - 01);
    let maturity = date!(2029 - 01 - 01); // 4Y
    let loan = build_simple_term_loan(as_of, maturity);

    let disc_curve = build_flat_discount_curve(0.05, as_of, "USD-OIS");
    let market = MarketContext::new().insert_discount(disc_curve);

    // Compute YT2Y, YT3Y, YT4Y and YTM for ordering checks
    let result = loan
        .price_with_metrics(
            &market,
            as_of,
            &[
                MetricId::Ytm,
                MetricId::custom("yt2y"),
                MetricId::custom("yt3y"),
                MetricId::custom("yt4y"),
            ],
        )
        .unwrap();

    let ytm = *result.measures.get("ytm").unwrap();
    let yt2y = *result.measures.get("yt2y").unwrap();
    let yt3y = *result.measures.get("yt3y").unwrap();
    let yt4y = *result.measures.get("yt4y").unwrap();

    // All finite and positive
    for y in [ytm, yt2y, yt3y, yt4y] { assert!(y.is_finite() && y > 0.0); }

    // Horizon yields should be non-decreasing with longer horizons in this setup
    assert!(yt2y <= yt3y + 1e-12);
    assert!(yt3y <= yt4y + 1e-12);

    // YT4Y equals YTM when maturity is 4 years
    assert!((yt4y - ytm).abs() < 1e-8);
}




