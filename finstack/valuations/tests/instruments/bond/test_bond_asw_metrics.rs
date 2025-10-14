use finstack_core::currency::Currency;
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, Frequency, StubKind};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::{DiscountCurve, ForwardCurve};
use finstack_core::money::Money;
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::instruments::bond::Bond;
use finstack_valuations::metrics::MetricId;
use time::Month;

fn d(y: i32, m: u8, day: u8) -> Date {
    Date::from_calendar_date(y, Month::try_from(m).unwrap(), day).unwrap()
}

fn flat_disc(rate: f64, base: Date, id: &str) -> DiscountCurve {
    DiscountCurve::builder(id)
        .base_date(base)
        .day_count(DayCount::Act365F)
        .knots(vec![(0.0, 1.0), (1.0, (-rate).exp()), (5.0, (-rate * 5.0).exp())])
        .build()
        .unwrap()
}

fn flat_fwd(id: &str, base: Date, tenor: f64, rate: f64) -> ForwardCurve {
    ForwardCurve::builder(id, tenor)
        .base_date(base)
        .knots(vec![(0.0, rate), (5.0, rate)])
        .build()
        .unwrap()
}

fn standard_bond(as_of: Date) -> Bond {
    Bond::builder()
        .id("BOND-ASW".into())
        .notional(Money::new(100.0, Currency::USD))
        .coupon(0.05)
        .freq(Frequency::semi_annual())
        .dc(DayCount::Act365F)
        .bdc(BusinessDayConvention::Following)
        .calendar_id_opt(None)
        .stub(StubKind::None)
        .issue(as_of)
        .maturity(d(2030, 1, 2))
        .disc_id("USD-OIS".into())
        .pricing_overrides(Default::default())
        .call_put_opt(None)
        .amortization_opt(None)
        .custom_cashflows_opt(None)
        .attributes(Default::default())
        .build()
        .unwrap()
}

#[test]
fn asw_par_and_market_defined_and_reasonable() {
    let as_of = d(2025, 1, 2);
    let bond = standard_bond(as_of);
    let disc = flat_disc(0.04, as_of, "USD-OIS");
    let mut ctx = MarketContext::new().insert_discount(disc);
    // Provide a forward curve for ASW forward variants
    let fwd = flat_fwd("USD-SOFR-6M", as_of, 0.5, 0.04);
    ctx = ctx.insert_forward(fwd);

    let res = bond
        .price_with_metrics(
            &ctx,
            as_of,
            &[
                MetricId::ASWPar,
                MetricId::ASWMarket,
                MetricId::ASWParFwd,
                MetricId::ASWMarketFwd,
                MetricId::ZSpread,
                MetricId::CleanPrice,
            ],
        )
        .unwrap();

    let asw_par = res.measures[MetricId::ASWPar.as_str()];
    let asw_mkt = res.measures[MetricId::ASWMarket.as_str()];
    let asw_par_fwd = res.measures[MetricId::ASWParFwd.as_str()];
    let asw_mkt_fwd = res.measures[MetricId::ASWMarketFwd.as_str()];
    let z = res.measures[MetricId::ZSpread.as_str()];
    let clean = res.measures[MetricId::CleanPrice.as_str()];

    // Sanity: ASW metrics and z-spread finite and within plausible ranges
    for v in [asw_par, asw_mkt, asw_par_fwd, asw_mkt_fwd, z] {
        assert!(v.is_finite());
        assert!(v.abs() < 0.5);
    }
    // Clean price near par in this setup
    assert!(clean > 90.0 && clean < 110.0);
}


