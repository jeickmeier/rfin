//! Tests for analytical Asian option pricers.
//!
//! These specifically target `instruments/asian_option/pricer.rs` (non-MC paths).

use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount, DayCountCtx};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::scalars::MarketScalar;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::exotics::asian_option::{AsianOption, AveragingMethod};
use finstack_valuations::instruments::common::models::closed_form::asian::{
    geometric_asian_call, geometric_asian_put,
};
use finstack_valuations::instruments::OptionType;
use finstack_valuations::pricer::Pricer;
use finstack_valuations::test_utils::{date, flat_discount_with_tenor, flat_vol_surface};

fn market(as_of: Date, spot: f64, vol: f64, rate: f64, div_yield: f64) -> MarketContext {
    let expiries = [0.25, 0.5, 1.0, 2.0];
    let strikes = [80.0, 90.0, 100.0, 110.0, 120.0];
    MarketContext::new()
        .insert_discount(flat_discount_with_tenor("USD-OIS", as_of, rate, 5.0))
        .insert_surface(flat_vol_surface("SPX-VOL", &expiries, &strikes, vol))
        .insert_price("SPX-SPOT", MarketScalar::Unitless(spot))
        .insert_price("SPX-DIV", MarketScalar::Unitless(div_yield))
}

fn asian_base(
    averaging: AveragingMethod,
    option_type: OptionType,
    as_of: Date,
    expiry: Date,
    strike: f64,
    fixing_dates: Vec<Date>,
) -> AsianOption {
    finstack_valuations::instruments::exotics::asian_option::AsianOptionBuilder::new()
        .id(InstrumentId::new("ASIAN-TEST"))
        .underlying_ticker("SPX".to_string())
        .strike(Money::new(strike, Currency::USD))
        .option_type(option_type)
        .averaging_method(averaging)
        .expiry(expiry)
        .fixing_dates(fixing_dates)
        .notional(Money::new(1.0, Currency::USD))
        .day_count(DayCount::Act365F)
        .discount_curve_id(CurveId::new("USD-OIS"))
        .spot_id("SPX-SPOT".to_string())
        .vol_surface_id(CurveId::new("SPX-VOL"))
        .div_yield_id_opt(Some(CurveId::new("SPX-DIV")))
        .pricing_overrides(Default::default())
        .attributes(Default::default())
        .build()
        .expect("asian builder should succeed")
        // Avoid unused warning: keep as_of in signature for clarity (fixing schedule is relative).
        .tap(|_| {
            let _ = as_of;
        })
}

trait Tap {
    fn tap<F: FnOnce(&Self)>(self, f: F) -> Self
    where
        Self: Sized,
    {
        f(&self);
        self
    }
}
impl<T> Tap for T {}

#[test]
fn geometric_analytical_matches_closed_form_unseasoned() -> finstack_core::Result<()> {
    use finstack_valuations::instruments::exotics::asian_option::AsianOptionAnalyticalGeometricPricer;

    let as_of = date(2025, 1, 2);
    let expiry = date(2026, 1, 2);
    let fixing_dates = vec![
        date(2025, 4, 2),
        date(2025, 7, 2),
        date(2025, 10, 2),
        date(2026, 1, 2),
    ];

    let spot = 100.0;
    let strike = 100.0;
    let vol = 0.20;
    let rate = 0.05;
    let div_yield = 0.00;

    let mkt = market(as_of, spot, vol, rate, div_yield);
    let asian = asian_base(
        AveragingMethod::Geometric,
        OptionType::Call,
        as_of,
        expiry,
        strike,
        fixing_dates.clone(),
    );

    let pricer = AsianOptionAnalyticalGeometricPricer::new();
    let res = pricer.price_dyn(&asian, &mkt, as_of)?;
    let pv = res.value.amount();

    let t = DayCount::Act365F.year_fraction(as_of, expiry, DayCountCtx::default())?;
    let expected = geometric_asian_call(spot, strike, t, rate, div_yield, vol, fixing_dates.len());

    assert!((pv - expected).abs() < 1e-10);
    Ok(())
}

#[test]
fn geometric_analytical_expired_uses_realized_average() -> finstack_core::Result<()> {
    use finstack_valuations::instruments::exotics::asian_option::AsianOptionAnalyticalGeometricPricer;

    let expiry = date(2025, 6, 30);
    let as_of = expiry; // expired
    let fixing_dates = vec![date(2025, 5, 31), date(2025, 6, 30)];

    let mut asian = asian_base(
        AveragingMethod::Geometric,
        OptionType::Call,
        as_of,
        expiry,
        100.0,
        fixing_dates.clone(),
    );
    // Realized geometric average of 100 and 121 is 110.
    asian.past_fixings = vec![(fixing_dates[0], 100.0), (fixing_dates[1], 121.0)];

    let mkt = market(as_of, 999.0, 0.20, 0.05, 0.0); // spot irrelevant with realized fixings
    let pricer = AsianOptionAnalyticalGeometricPricer::new();
    let pv = pricer.price_dyn(&asian, &mkt, as_of)?.value.amount();

    assert!((pv - 10.0).abs() < 1e-12); // (110 - 100) * notional(1)
    Ok(())
}

#[test]
fn geometric_analytical_errors_when_seasoned_and_not_expired() -> finstack_core::Result<()> {
    use finstack_valuations::instruments::exotics::asian_option::AsianOptionAnalyticalGeometricPricer;

    let as_of = date(2025, 1, 2);
    let expiry = date(2025, 7, 2);
    let fixing_dates = vec![date(2025, 1, 1), date(2025, 4, 1), date(2025, 7, 2)];

    let mut asian = asian_base(
        AveragingMethod::Geometric,
        OptionType::Put,
        as_of,
        expiry,
        100.0,
        fixing_dates.clone(),
    );
    // One fixing already observed (seasoned)
    asian.past_fixings = vec![(fixing_dates[0], 99.0)];

    let mkt = market(as_of, 100.0, 0.20, 0.05, 0.0);
    let pricer = AsianOptionAnalyticalGeometricPricer::new();
    let err = pricer
        .price_dyn(&asian, &mkt, as_of)
        .expect_err("seasoned geometric analytical should error");
    assert!(err.to_string().contains("Seasoned Geometric Asian not supported"));
    Ok(())
}

#[test]
fn tw_arithmetic_all_fixings_in_past_discounts_deterministic_payoff() -> finstack_core::Result<()> {
    use finstack_valuations::instruments::exotics::asian_option::AsianOptionSemiAnalyticalTwPricer;

    let as_of = date(2025, 7, 1);
    let expiry = date(2025, 12, 31);
    let fixing_dates = vec![
        date(2025, 1, 31),
        date(2025, 2, 28),
        date(2025, 3, 31),
        date(2025, 4, 30),
        date(2025, 5, 31),
        date(2025, 6, 30),
    ];

    let mut asian = asian_base(
        AveragingMethod::Arithmetic,
        OptionType::Call,
        as_of,
        expiry,
        100.0,
        fixing_dates.clone(),
    );

    // All fixings observed (as_of after last fixing date)
    asian.past_fixings = fixing_dates
        .iter()
        .copied()
        .zip([102.0, 101.0, 103.0, 104.0, 100.0, 105.0])
        .collect();

    let rate = 0.05;
    let mkt = market(as_of, 100.0, 0.20, rate, 0.0);
    let pricer = AsianOptionSemiAnalyticalTwPricer::new();
    let pv = pricer.price_dyn(&asian, &mkt, as_of)?.value.amount();

    let sum: f64 = [102.0, 101.0, 103.0, 104.0, 100.0, 105.0].iter().sum();
    let avg = sum / fixing_dates.len() as f64;
    let payoff = (avg - 100.0).max(0.0);
    let t = DayCount::Act365F.year_fraction(as_of, expiry, DayCountCtx::default())?;
    let df = (-rate * t).exp();

    assert!((pv - payoff * df).abs() < 1e-10);
    Ok(())
}

#[test]
fn tw_arithmetic_negative_k_eff_put_returns_zero() -> finstack_core::Result<()> {
    use finstack_valuations::instruments::exotics::asian_option::AsianOptionSemiAnalyticalTwPricer;

    let as_of = date(2025, 1, 2);
    let expiry = date(2025, 7, 2);
    let fixing_dates = vec![
        date(2025, 1, 2),
        date(2025, 2, 2),
        date(2025, 3, 2),
        date(2025, 4, 2),
        date(2025, 5, 2),
        date(2025, 6, 2),
        date(2025, 7, 2),
        date(2025, 8, 2),
        date(2025, 9, 2),
        date(2025, 10, 2),
    ];

    let mut asian = asian_base(
        AveragingMethod::Arithmetic,
        OptionType::Put,
        as_of,
        expiry,
        100.0,
        fixing_dates.clone(),
    );

    // Seasoned with very high fixings so that k_eff < 0.
    // n=10, k=100 => n*k=1000. sum_past=5*300=1500 => k_eff negative.
    asian.past_fixings = fixing_dates
        .iter()
        .take(5)
        .copied()
        .map(|d| (d, 300.0))
        .collect();

    let mkt = market(as_of, 100.0, 0.20, 0.05, 0.0);
    let pricer = AsianOptionSemiAnalyticalTwPricer::new();
    let pv = pricer.price_dyn(&asian, &mkt, as_of)?.value.amount();

    assert!((pv - 0.0).abs() < 1e-12);
    Ok(())
}

#[test]
fn geometric_closed_form_put_matches_helper() -> finstack_core::Result<()> {
    // Quick sanity: exercise the geometric put path in the pricer file by comparing
    // to the closed-form helper for puts.
    use finstack_valuations::instruments::exotics::asian_option::AsianOptionAnalyticalGeometricPricer;

    let as_of = date(2025, 1, 2);
    let expiry = date(2026, 1, 2);
    let fixing_dates = vec![date(2025, 6, 2), date(2026, 1, 2)];

    let spot = 100.0;
    let strike = 110.0;
    let vol = 0.25;
    let rate = 0.03;
    let div_yield = 0.01;

    let mkt = market(as_of, spot, vol, rate, div_yield);
    let asian = asian_base(
        AveragingMethod::Geometric,
        OptionType::Put,
        as_of,
        expiry,
        strike,
        fixing_dates.clone(),
    );

    let pricer = AsianOptionAnalyticalGeometricPricer::new();
    let pv = pricer.price_dyn(&asian, &mkt, as_of)?.value.amount();

    let t = DayCount::Act365F.year_fraction(as_of, expiry, DayCountCtx::default())?;
    let expected = geometric_asian_put(spot, strike, t, rate, div_yield, vol, fixing_dates.len());
    assert!((pv - expected).abs() < 1e-10);
    Ok(())
}
