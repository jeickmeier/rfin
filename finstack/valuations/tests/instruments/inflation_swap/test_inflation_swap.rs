use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::scalars::inflation_index::{InflationIndex, InflationInterpolation, InflationLag};
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::market_data::term_structures::inflation::InflationCurve;
use finstack_core::money::Money;
use finstack_core::F;
use time::Month;

use finstack_valuations::instruments::inflation_swap::{InflationSwap, PayReceiveInflation};

fn flat_discount(id: &'static str, base: Date) -> DiscountCurve {
    DiscountCurve::builder(id)
        .base_date(base)
        .knots(vec![(0.0, 1.0), (10.0, 1.0)])
        .build()
        .unwrap()
}

fn simple_inflation_curve(id: &'static str, base_cpi: F) -> InflationCurve {
    InflationCurve::builder(id)
        .base_cpi(base_cpi)
        .knots([(0.0, base_cpi), (10.0, base_cpi * 1.2)]) // ~ cumulative inflation over 10y
        .build()
        .unwrap()
}

fn simple_index(base: Date, base_cpi: F, ccy: Currency) -> InflationIndex {
    let observations = vec![
        (base, base_cpi),
        (base + time::Duration::days(30), base_cpi * 1.001),
        (base + time::Duration::days(60), base_cpi * 1.002),
    ];
    InflationIndex::new("US-CPI-U", observations, ccy)
        .unwrap()
        .with_interpolation(InflationInterpolation::Step)
}

#[test]
fn pv_uses_lag_override_when_present() {
    let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let maturity = Date::from_calendar_date(2030, Month::January, 1).unwrap();
    let notional = Money::new(1_000_000.0, Currency::USD);

    let disc = flat_discount("USD-OIS", as_of);
    let infl_curve = simple_inflation_curve("US-CPI-U", 300.0);
    // Index with 3M lag
    let index = simple_index(as_of, 300.0, Currency::USD).with_lag(InflationLag::Months(3));

    let ctx = MarketContext::new()
        .insert_discount(disc)
        .insert_inflation(infl_curve)
        .insert_inflation_index("US-CPI-U", index);

    // Swap with override of 0 lag
    let swap_no_lag = InflationSwapBuilder::new()
        .id("ZCINF-0")
        .notional(notional)
        .start(as_of)
        .maturity(maturity)
        .fixed_rate(0.0)
        .inflation_id("US-CPI-U")
        .disc_id("USD-OIS")
        .dc(finstack_core::dates::DayCount::Act365F)
        .side(PayReceiveInflation::PayFixed)
        .lag_override(InflationLag::None)
        .attributes(finstack_valuations::instruments::traits::Attributes::new())
        .build()
        .unwrap();

    // Swap relying on index 3M lag
    let swap_idx_lag = InflationSwapBuilder::new()
        .id("ZCINF-3M")
        .notional(notional)
        .start(as_of)
        .maturity(maturity)
        .fixed_rate(0.0)
        .inflation_id("US-CPI-U")
        .disc_id("USD-OIS")
        .dc(finstack_core::dates::DayCount::Act365F)
        .side(PayReceiveInflation::PayFixed)
        .attributes(finstack_valuations::instruments::traits::Attributes::new())
        .build()
        .unwrap();

    let pv_no_lag = swap_no_lag.value(&ctx, as_of).unwrap().amount();
    let pv_idx_lag = swap_idx_lag.value(&ctx, as_of).unwrap().amount();

    // With positive forward inflation, a longer lag reduces I(T-L), reducing payout for PayFixed
    // hence pv_no_lag (L=0) should be >= pv_idx_lag (L=3M)
    assert!(pv_no_lag >= pv_idx_lag);
}

#[test]
fn breakeven_parity_near_zero_with_correct_k() {
    let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let maturity = Date::from_calendar_date(2030, Month::January, 1).unwrap();
    let notional = Money::new(1_000_000.0, Currency::USD);

    let disc = flat_discount("USD-OIS", as_of);
    let infl_curve = simple_inflation_curve("US-CPI-U", 300.0);
    let index = simple_index(as_of, 300.0, Currency::USD).with_lag(InflationLag::Months(3));

    let ctx = MarketContext::new()
        .insert_discount(disc)
        .insert_inflation(infl_curve)
        .insert_inflation_index("US-CPI-U", index);

    // Compute breakeven via metric
    let registry = finstack_valuations::metrics::standard_registry();
    let mut mctx = finstack_valuations::metrics::MetricContext::new(
        &ctx,
        &InflationSwapBuilder::new()
            .id("ZCINF-BE")
            .notional(notional)
            .start(as_of)
            .maturity(maturity)
            .fixed_rate(0.0)
            .inflation_id("US-CPI-U")
            .disc_id("USD-OIS")
            .dc(finstack_core::dates::DayCount::Act365F)
            .side(PayReceiveInflation::PayFixed)
            .attributes(finstack_valuations::instruments::traits::Attributes::new())
            .build()
            .unwrap(),
        as_of,
        &registry,
    );
    let k_be = registry
        .get(&finstack_valuations::metrics::MetricId::custom("breakeven"))
        .unwrap()
        .calculate(&mut mctx)
        .unwrap();

    // Set fixed rate to breakeven and verify PV ~ 0
    let swap_be = InflationSwapBuilder::new()
        .id("ZCINF-BE2")
        .notional(notional)
        .start(as_of)
        .maturity(maturity)
        .fixed_rate(k_be)
        .inflation_id("US-CPI-U")
        .disc_id("USD-OIS")
        .dc(finstack_core::dates::DayCount::Act365F)
        .side(PayReceiveInflation::PayFixed)
        .attributes(finstack_valuations::instruments::traits::Attributes::new())
        .build()
        .unwrap();

    let pv = swap_be.value(&ctx, as_of).unwrap().amount();
    assert!(pv.abs() < 1e-4 * notional.amount());
}


