use finstack_core::currency::Currency;
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, Frequency, StubKind};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::money::Money;
use finstack_valuations::instruments::bond::pricing::helpers::{
    compute_accrued_interest, compute_accrued_interest_with_context, df_from_yield, periods_per_year,
    price_from_asw_spread, price_from_i_spread, price_from_ytm_compounded_params, YieldCompounding,
};
use finstack_valuations::instruments::bond::Bond;
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

fn std_bond(as_of: Date) -> Bond {
    Bond::builder()
        .id("BOND-HELP".into())
        .notional(Money::new(100.0, Currency::USD))
        .coupon(0.06)
        .freq(Frequency::semi_annual())
        .dc(DayCount::Act365F)
        .bdc(BusinessDayConvention::Following)
        .calendar_id_opt(None)
        .stub(StubKind::None)
        .issue(as_of)
        .maturity(d(2028, 1, 2))
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
fn periods_per_year_and_df_from_yield_behave_reasonably() {
    // Frequency mapping
    assert!((periods_per_year(Frequency::monthly()).unwrap() - 12.0).abs() < 1e-12);
    assert!((periods_per_year(Frequency::semi_annual()).unwrap() - 2.0).abs() < 1e-12);
    assert!((periods_per_year(Frequency::annual()).unwrap_err().to_string().len()) > 0);

    // Discount factor shapes
    let t = 2.5; // years
    let y = 0.05;
    let df_simple = df_from_yield(y, t, YieldCompounding::Simple, Frequency::annual()).unwrap();
    let df_annual = df_from_yield(y, t, YieldCompounding::Annual, Frequency::annual()).unwrap();
    let df_cont = df_from_yield(y, t, YieldCompounding::Continuous, Frequency::annual()).unwrap();
    assert!(df_cont > df_annual && df_annual > df_simple); // higher compounding ⇒ higher df given same nominal y
}

#[test]
fn price_from_ytm_compounded_params_matches_intuition() {
    let as_of = d(2025, 1, 2);
    let flows = vec![(d(2026, 1, 2), Money::new(6.0, Currency::USD)), (d(2027, 1, 2), Money::new(106.0, Currency::USD))];
    let price = price_from_ytm_compounded_params(
        DayCount::Act365F,
        Frequency::annual(),
        &flows,
        as_of,
        0.05,
        YieldCompounding::Street,
    )
    .unwrap();
    assert!(price > 100.0 * 0.9 && price < 110.0); // reasonable bound
}

#[test]
fn accrued_interest_fixed_and_frn_paths() {
    let as_of = d(2025, 1, 2);
    let disc = flat_disc(0.04, as_of, "USD-OIS");
    let market = MarketContext::new().insert_discount(disc);
    let bond = std_bond(as_of);

    // Fixed path accrual positive shortly after issue
    let accr = compute_accrued_interest(&bond, d(2025, 4, 2)).unwrap();
    assert!(accr > 0.0);

    // FRN path falls back to fixed when no float spec
    let accr2 = compute_accrued_interest_with_context(&bond, &market, d(2025, 4, 2)).unwrap();
    assert!((accr - accr2).abs() < 1e-12);
}

#[test]
fn price_from_spread_annuity_helpers_monotonic_in_spread() {
    let as_of = d(2025, 1, 2);
    let disc = flat_disc(0.04, as_of, "USD-OIS");
    let market = MarketContext::new().insert_discount(disc);
    let bond = std_bond(as_of);

    let p0 = price_from_i_spread(&bond, &market, as_of, 0.0).unwrap();
    let p_wider = price_from_i_spread(&bond, &market, as_of, 0.01).unwrap();
    assert!(p_wider < p0);

    let a0 = price_from_asw_spread(&bond, &market, as_of, 0.0).unwrap();
    let a_wider = price_from_asw_spread(&bond, &market, as_of, 0.01).unwrap();
    assert!(a_wider < a0);
}


