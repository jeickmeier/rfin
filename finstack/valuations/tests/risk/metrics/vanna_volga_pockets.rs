//! Coverage tests for vanna/volga metric calculators that were previously uncovered.
//!
//! These tests are intentionally small and deterministic:
//! - Exercise the calculators through the standard registry (public API).
//! - Validate results against an explicit finite-difference reference implementation.

use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount, DayCountCtx};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::scalars::MarketScalar;
use finstack_core::money::fx::{FxConversionPolicy, FxMatrix, FxProvider};
use finstack_core::money::Money;
use finstack_valuations::instruments::market::{ExerciseStyle, OptionType};
use finstack_valuations::instruments::equity::equity_option::EquityOption;
use finstack_valuations::instruments::fx::fx_option::FxOption;
use finstack_valuations::instruments::Instrument;
use finstack_valuations::metrics::{bump_scalar_price, bump_sizes, scale_surface, standard_registry, MetricContext, MetricId};
use finstack_valuations::test_utils::{
    date, equity_option_european_call, flat_discount_with_tenor, flat_vol_surface,
    fx_option_european_call,
};
use std::sync::Arc;

fn approx_eq(actual: f64, expected: f64, tol: f64) {
    let diff = (actual - expected).abs();
    assert!(
        diff <= tol,
        "expected {expected}, got {actual} (diff {diff} > {tol})"
    );
}

fn equity_market(as_of: Date, spot: f64, vol: f64, rate: f64, div_yield: f64) -> MarketContext {
    // IDs chosen to align with EquityOption::european_call defaults.
    let expiries = [0.25, 0.5, 1.0, 2.0];
    let strikes = [80.0, 90.0, 100.0, 110.0, 120.0];

    MarketContext::new()
        .insert_discount(flat_discount_with_tenor("USD-OIS", as_of, rate, 5.0))
        .insert_surface(flat_vol_surface("EQUITY-VOL", &expiries, &strikes, vol))
        .insert_price("EQUITY-SPOT", MarketScalar::Unitless(spot))
        .insert_price("EQUITY-DIVYIELD", MarketScalar::Unitless(div_yield))
}

fn equity_option(as_of: Date, expiry: Date, strike: f64) -> EquityOption {
    let _ = as_of;
    equity_option_european_call(
        "EQ-VANNA-VOLGA",
        "SPX",
        strike,
        expiry,
        Money::new(1_000_000.0, Currency::USD),
        100.0,
    )
    .expect("equity option should build for vanna/volga tests")
}

fn equity_delta_fd(
    opt: &EquityOption,
    curves: &MarketContext,
    as_of: Date,
    spot_bump_pct: f64,
) -> finstack_core::Result<f64> {
    // Central delta via bump-and-reprice.
    let curves_up = bump_scalar_price(curves, &opt.spot_id, spot_bump_pct)?;
    let curves_dn = bump_scalar_price(curves, &opt.spot_id, -spot_bump_pct)?;
    let pv_up = opt.npv(&curves_up, as_of)?.amount();
    let pv_dn = opt.npv(&curves_dn, as_of)?.amount();

    // Convert bump_pct into absolute spot bump for denominator.
    let spot = match curves.price(&opt.spot_id)? {
        MarketScalar::Unitless(v) => *v,
        MarketScalar::Price(m) => m.amount(),
    };
    let h = spot * spot_bump_pct;
    Ok((pv_up - pv_dn) / (2.0 * h))
}

#[test]
fn equity_vanna_and_volga_match_reference_fd() -> finstack_core::Result<()> {
    let as_of = date(2025, 1, 2);
    let expiry = date(2025, 7, 2);
    let spot = 100.0;
    let strike = 100.0;
    let vol = 0.20;
    let rate = 0.03;
    let div_yield = 0.01;

    let opt = equity_option(as_of, expiry, strike);
    let market = equity_market(as_of, spot, vol, rate, div_yield);

    let pv = opt.value(&market, as_of)?;
    let mut ctx = MetricContext::new(Arc::new(opt.clone()), Arc::new(market.clone()), as_of, pv, MetricContext::default_config());
    let registry = standard_registry();

    let res = registry.compute(&[MetricId::Vanna, MetricId::Volga], &mut ctx)?;
    let vanna = *res.get(&MetricId::Vanna).expect("vanna present");
    let volga = *res.get(&MetricId::Volga).expect("volga present");

    // Reference finite differences using the same bump convention:
    // - Spot: ±1% (relative)
    // - Vol: surface scaled by 1±1% (relative), but *denominator uses absolute Δσ* at ATM
    let t = opt
        .day_count
        .year_fraction(as_of, expiry, DayCountCtx::default())?;
    let surf = market.surface(opt.vol_surface_id.as_str())?;
    let atm_sigma = surf.value_clamped(t, strike);
    let vol_bump_pct = bump_sizes::VOLATILITY;
    let delta_sigma = (atm_sigma * vol_bump_pct).abs().max(1e-12);

    let curves_vol_up = scale_surface(&market, opt.vol_surface_id.as_str(), 1.0 + vol_bump_pct)?;
    let curves_vol_dn = scale_surface(&market, opt.vol_surface_id.as_str(), 1.0 - vol_bump_pct)?;

    let spot_bump_pct = bump_sizes::SPOT;
    let delta_up = equity_delta_fd(&opt, &curves_vol_up, as_of, spot_bump_pct)?;
    let delta_dn = equity_delta_fd(&opt, &curves_vol_dn, as_of, spot_bump_pct)?;
    let vanna_ref = (delta_up - delta_dn) / (2.0 * delta_sigma);

    let pv_up = opt.npv(&curves_vol_up, as_of)?.amount();
    let pv_0 = opt.npv(&market, as_of)?.amount();
    let pv_dn = opt.npv(&curves_vol_dn, as_of)?.amount();
    let volga_ref = (pv_up - 2.0 * pv_0 + pv_dn) / (delta_sigma * delta_sigma);

    // Tolerances: these are FD-vs-FD comparisons, so keep them tight but not brittle.
    approx_eq(vanna, vanna_ref, 5e-6);
    approx_eq(volga, volga_ref, 5e-4);
    Ok(())
}

#[test]
fn equity_vanna_and_volga_are_zero_when_expired() -> finstack_core::Result<()> {
    let as_of = date(2025, 7, 2);
    let expiry = date(2025, 7, 2); // expired at as_of
    let opt = equity_option(as_of, expiry, 100.0);
    let market = equity_market(as_of, 100.0, 0.20, 0.03, 0.0);

    let pv = opt.value(&market, as_of)?;
    let mut ctx = MetricContext::new(Arc::new(opt), Arc::new(market), as_of, pv, MetricContext::default_config());
    let registry = standard_registry();

    let res = registry.compute(&[MetricId::Vanna, MetricId::Volga], &mut ctx)?;
    assert_eq!(*res.get(&MetricId::Vanna).unwrap(), 0.0);
    assert_eq!(*res.get(&MetricId::Volga).unwrap(), 0.0);
    Ok(())
}

// -------------------- FX option vanna/volga --------------------

struct TestFx {
    spot: f64,
}
impl FxProvider for TestFx {
    fn rate(
        &self,
        from: Currency,
        to: Currency,
        _on: Date,
        _policy: FxConversionPolicy,
    ) -> finstack_core::Result<f64> {
        // Support EUR/USD and USD/EUR for tests.
        if from == Currency::EUR && to == Currency::USD {
            Ok(self.spot)
        } else if from == Currency::USD && to == Currency::EUR {
            Ok(1.0 / self.spot)
        } else if from == to {
            Ok(1.0)
        } else {
            Err(finstack_core::Error::Validation("FX rate not found".to_string()))
        }
    }
}

fn fx_market(as_of: Date, spot: f64, vol: f64, r_d: f64, r_f: f64) -> MarketContext {
    let expiries = [0.25, 0.5, 1.0, 2.0];
    let strikes = [0.9, 1.0, 1.1, 1.2, 1.3];

    let fx = FxMatrix::new(Arc::new(TestFx { spot }));
    MarketContext::new()
        .insert_fx(fx)
        .insert_discount(flat_discount_with_tenor("USD-OIS", as_of, r_d, 5.0))
        .insert_discount(flat_discount_with_tenor("EUR-OIS", as_of, r_f, 5.0))
        .insert_surface(flat_vol_surface("EURUSD-VOL", &expiries, &strikes, vol))
}

#[test]
fn fx_vanna_and_volga_match_reference_fd() -> finstack_core::Result<()> {
    let as_of = date(2025, 1, 2);
    let expiry = date(2025, 7, 2);

    let spot = 1.10;
    let strike = 1.10;
    let vol = 0.12;
    let r_d = 0.04;
    let r_f = 0.02;

    let opt = fx_option_european_call(
        "FX-VANNA-VOLGA".into(),
        Currency::EUR,
        Currency::USD,
        strike,
        expiry,
        Money::new(1_000_000.0, Currency::EUR),
        "EURUSD-VOL",
    )
    .unwrap();

    let market = fx_market(as_of, spot, vol, r_d, r_f);
    let pv = opt.value(&market, as_of)?;

    let mut ctx = MetricContext::new(Arc::new(opt.clone()), Arc::new(market.clone()), as_of, pv, MetricContext::default_config());
    let registry = standard_registry();

    let res = registry.compute(&[MetricId::Vanna, MetricId::Volga], &mut ctx)?;
    let vanna = *res.get(&MetricId::Vanna).expect("vanna present");
    let volga = *res.get(&MetricId::Volga).expect("volga present");

    // Explicit reference using the calculator's own bump conventions:
    // bump a single surface point by ±1% and divide by the corresponding absolute Δσ.
    let t = opt
        .day_count
        .year_fraction(as_of, expiry, DayCountCtx::default())?;
    let surf = market.surface(opt.vol_surface_id.as_str())?;
    let sigma = surf.value_clamped(t, opt.strike);
    let vol_bump_pct = bump_sizes::VOLATILITY;
    let delta_sigma = (sigma * vol_bump_pct).abs().max(1e-12);

    let curves_up = {
        let bumped = surf.bump_point(t, opt.strike, vol_bump_pct)?;
        market.clone().insert_surface(bumped)
    };
    let curves_dn = {
        let bumped = surf.bump_point(t, opt.strike, -vol_bump_pct)?;
        market.clone().insert_surface(bumped)
    };

    let delta_up = opt.compute_greeks(&curves_up, as_of)?.delta;
    let delta_dn = opt.compute_greeks(&curves_dn, as_of)?.delta;
    let vanna_ref = (delta_up - delta_dn) / (2.0 * delta_sigma);

    let vega_up = opt.compute_greeks(&curves_up, as_of)?.vega;
    let vega_dn = opt.compute_greeks(&curves_dn, as_of)?.vega;
    let volga_ref = (vega_up - vega_dn) / (2.0 * delta_sigma) * 0.01;

    approx_eq(vanna, vanna_ref, 1e-10);
    approx_eq(volga, volga_ref, 1e-10);
    Ok(())
}

#[test]
fn fx_volga_returns_zero_when_surface_vol_is_zero() -> finstack_core::Result<()> {
    let as_of = date(2025, 1, 2);
    let expiry = date(2025, 7, 2);

    let opt = fx_option_european_call(
        "FX-ZERO-VOLGA".into(),
        Currency::EUR,
        Currency::USD,
        1.10,
        expiry,
        Money::new(1_000_000.0, Currency::EUR),
        "EURUSD-VOL",
    )
    .unwrap();

    let market = fx_market(as_of, 1.10, 0.0, 0.02, 0.01);
    let pv = opt.value(&market, as_of)?;
    let mut ctx = MetricContext::new(Arc::new(opt), Arc::new(market), as_of, pv, MetricContext::default_config());
    let registry = standard_registry();

    let res = registry.compute(&[MetricId::Volga], &mut ctx)?;
    assert_eq!(*res.get(&MetricId::Volga).unwrap(), 0.0);
    Ok(())
}
