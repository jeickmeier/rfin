use finstack_core::currency::Currency;
use finstack_core::dates::{DayCount, DayCountCtx};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::scalars::MarketScalar;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::barrier_option::types::BarrierType;
use finstack_valuations::instruments::common::models::closed_form::barrier::{
    barrier_rebate_continuous, BarrierType as AnalyticalBarrierType,
};
use finstack_valuations::instruments::common::traits::Attributes;
use finstack_valuations::instruments::fx_barrier_option::FxBarrierOption;
use finstack_valuations::instruments::{OptionType, PricingOverrides};
use finstack_valuations::test_utils::{date, flat_discount_with_tenor, flat_vol_surface};

#[test]
fn test_fx_barrier_rebate_added_to_closed_form_price() {
    let as_of = date(2024, 1, 1);
    let expiry = date(2025, 1, 1);
    let spot = 1.10;
    let strike = 1.10;
    let barrier = 1.20;
    let rebate = Money::new(0.02, Currency::USD);

    let dom_curve = flat_discount_with_tenor("USD-OIS", as_of, 0.03, 5.0);
    let for_curve = flat_discount_with_tenor("EUR-OIS", as_of, 0.01, 5.0);
    let expiries = [0.25, 0.5, 1.0, 2.0, 5.0];
    let strikes = [0.9, 1.0, 1.1, 1.2, 1.3];
    let vol_surface = flat_vol_surface("EURUSD-VOL", &expiries, &strikes, 0.15);

    let market = MarketContext::new()
        .insert_discount(dom_curve)
        .insert_discount(for_curve)
        .insert_surface(vol_surface)
        .insert_price("EURUSD-SPOT", MarketScalar::Unitless(spot));

    let base_option = FxBarrierOption::builder()
        .id(InstrumentId::new("FXBAR-REBATE-BASE"))
        .strike(Money::new(strike, Currency::USD))
        .barrier(Money::new(barrier, Currency::USD))
        .option_type(OptionType::Call)
        .barrier_type(BarrierType::UpAndOut)
        .expiry(expiry)
        .notional(Money::new(1_000_000.0, Currency::USD))
        .domestic_currency(Currency::USD)
        .foreign_currency(Currency::EUR)
        .day_count(DayCount::Act365F)
        .use_gobet_miri(false)
        .domestic_discount_curve_id(CurveId::new("USD-OIS"))
        .foreign_discount_curve_id(CurveId::new("EUR-OIS"))
        .fx_spot_id("EURUSD-SPOT".to_string())
        .fx_vol_id(CurveId::new("EURUSD-VOL"))
        .pricing_overrides(PricingOverrides::default())
        .attributes(Attributes::new())
        .build()
        .expect("Base FX barrier option should build");

    let mut rebate_option = base_option.clone();
    rebate_option.rebate = Some(rebate);

    let base_pv = base_option.npv(&market, as_of).expect("Base PV");
    let rebate_pv = rebate_option
        .npv(&market, as_of)
        .expect("Rebate PV");

    let t = DayCount::Act365F
        .year_fraction(as_of, expiry, DayCountCtx::default())
        .expect("Year fraction");
    let r_dom = market
        .get_discount_ref("USD-OIS")
        .expect("USD curve")
        .zero(t);
    let r_for = market
        .get_discount_ref("EUR-OIS")
        .expect("EUR curve")
        .zero(t);
    let sigma = market
        .surface_ref("EURUSD-VOL")
        .expect("FX vol")
        .value_clamped(t, strike);

    let expected_rebate = barrier_rebate_continuous(
        spot,
        barrier,
        rebate.amount(),
        t,
        r_dom,
        r_for,
        sigma,
        AnalyticalBarrierType::UpOut,
    ) * base_option.notional.amount();

    let rebate_delta = rebate_pv.amount() - base_pv.amount();
    // Allow small tolerance for floating point differences
    assert!(
        (rebate_delta - expected_rebate).abs() < 0.01,
        "Rebate PV mismatch: {} vs {}",
        rebate_delta,
        expected_rebate
    );
}
