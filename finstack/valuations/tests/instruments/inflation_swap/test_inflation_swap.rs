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
        .attributes(finstack_valuations::instruments::common::traits::Attributes::new())
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
        .attributes(finstack_valuations::instruments::common::traits::Attributes::new())
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
            .attributes(finstack_valuations::instruments::common::traits::Attributes::new())
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
        .attributes(finstack_valuations::instruments::common::traits::Attributes::new())
        .build()
        .unwrap();

    let pv = swap_be.value(&ctx, as_of).unwrap().amount();
    assert!(pv.abs() < 1e-4 * notional.amount());
}

#[test]
fn ir01_fd_matches_metric_sign() {
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

    let swap = InflationSwapBuilder::new()
        .id("ZCINF-IR01")
        .notional(notional)
        .start(as_of)
        .maturity(maturity)
        .fixed_rate(0.01) // 1%
        .inflation_id("US-CPI-U")
        .disc_id("USD-OIS")
        .dc(finstack_core::dates::DayCount::Act365F)
        .side(PayReceiveInflation::PayFixed)
        .attributes(finstack_valuations::instruments::common::traits::Attributes::new())
        .build()
        .unwrap();

    // Base PV
    let pv0 = swap.value(&ctx, as_of).unwrap().amount();

    // Bump discount curve by 1bp in DF space: df_bumped(t) = df(t) * exp(-bp * t)
    let disc_base = ctx
        .get_ref::<DiscountCurve>("USD-OIS")
        .unwrap();
    let base_date = disc_base.base_date();

    // Build bumped curve by transforming knot dfs
    let mut bumped_points: Vec<(F, F)> = Vec::new();
    for &t in disc_base.knots() {
        let df = disc_base.df(t);
        let df_b = df * (-0.0001 * t).exp();
        bumped_points.push((t, df_b));
    }
    let bumped_disc = DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .knots(bumped_points)
        .build()
        .unwrap();

    let ctx_b = ctx.clone().insert_discount(bumped_disc);
    let pv1 = swap.value(&ctx_b, as_of).unwrap().amount();

    let ir01_fd = pv1 - pv0; // per 1bp

    // Metric IR01
    let reg = finstack_valuations::metrics::standard_registry();
    let mut mctx = finstack_valuations::metrics::MetricContext::new(&ctx, &swap, as_of, &reg);
    let ir01 = reg
        .get(&finstack_valuations::metrics::MetricId::custom("ir01"))
        .unwrap()
        .calculate(&mut mctx)
        .unwrap();

    // Directional consistency check; allow factor differences due to approximation
    assert!(ir01.signum() == ir01_fd.signum());
}

#[test]
fn inflation01_fd_matches_metric_direction() {
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

    let swap = InflationSwapBuilder::new()
        .id("ZCINF-INFL01")
        .notional(notional)
        .start(as_of)
        .maturity(maturity)
        .fixed_rate(0.0)
        .inflation_id("US-CPI-U")
        .disc_id("USD-OIS")
        .dc(finstack_core::dates::DayCount::Act365F)
        .side(PayReceiveInflation::PayFixed)
        .attributes(finstack_valuations::instruments::common::traits::Attributes::new())
        .build()
        .unwrap();

    let pv0 = swap.value(&ctx, as_of).unwrap().amount();

    // Bump inflation curve CPI levels up by 1bp multiplicatively
    let ic = ctx
        .get_ref::<InflationCurve>("US-CPI-U")
        .unwrap();
    let mut bumped_knots: Vec<(F, F)> = Vec::new();
    for (&t, &cpi) in ic.knots().iter().zip(ic.cpi_levels().iter()) {
        bumped_knots.push((t, cpi * 1.0001));
    }
    let bumped_ic = InflationCurve::builder("US-CPI-U")
        .base_cpi(ic.base_cpi())
        .knots(bumped_knots)
        .build()
        .unwrap();

    let ctx_b = ctx.clone().insert_inflation(bumped_ic);
    let pv1 = swap.value(&ctx_b, as_of).unwrap().amount();

    let infl01_fd = pv1 - pv0; // per 1bp CPI level bump (approx)

    // Metric Inflation01
    let reg = finstack_valuations::metrics::standard_registry();
    let mut mctx = finstack_valuations::metrics::MetricContext::new(&ctx, &swap, as_of, &reg);
    let infl01 = reg
        .get(&finstack_valuations::metrics::MetricId::custom("inflation01"))
        .unwrap()
        .calculate(&mut mctx)
        .unwrap();

    assert!(infl01.signum() == infl01_fd.signum());
}


