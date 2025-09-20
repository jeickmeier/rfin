#![cfg(test)]

use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::surfaces::vol_surface::VolSurface;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::money::Money;
use finstack_core::F;
use finstack_valuations::instruments::irs::PayReceive;
use finstack_valuations::instruments::swaption::parameters::SwaptionParams;
use finstack_valuations::instruments::swaption::Swaption;
use finstack_valuations::instruments::PricingOverrides;
use finstack_valuations::metrics::{standard_registry, MetricContext, MetricId};
use std::sync::Arc;
use time::Month;

fn test_discount(id: &'static str, base: Date) -> DiscountCurve {
    DiscountCurve::builder(id)
        .base_date(base)
        .knots(vec![(0.0, 1.0), (5.0, 0.85), (10.0, 0.70)])
        .build()
        .unwrap()
}

fn flat_vol_surface(id: &'static str, level: F) -> VolSurface {
    // Simple 3x3 grid with constant vol
    let expiries = vec![0.5, 1.0, 2.0];
    let strikes = vec![0.01, 0.03, 0.05];
    let mut grid = Vec::with_capacity(expiries.len() * strikes.len());
    for _ in 0..expiries.len() * strikes.len() {
        grid.push(level);
    }
    VolSurface::from_grid(id, &expiries, &strikes, &grid).unwrap()
}

fn build_swaption(as_of: Date) -> (Swaption, MarketContext) {
    let notional = Money::new(10_000_000.0, Currency::USD);
    let expiry = Date::from_calendar_date(as_of.year() + 1, Month::January, 1).unwrap();
    let swap_start = expiry;
    let swap_end = Date::from_calendar_date(as_of.year() + 6, Month::January, 1).unwrap();

    let params = SwaptionParams::payer(notional, 0.03, expiry, swap_start, swap_end);
    let disc_id = "USD-OIS";
    let fwd_id = "USD-LIBOR";
    let vol_id = "USD-SWAPTION-VOL";

    let swaption = Swaption::new_payer("SWP-TEST", &params, disc_id, fwd_id, vol_id);

    // Market
    let disc = test_discount(disc_id, as_of);
    let vs = flat_vol_surface(vol_id, 0.20);
    let ctx = MarketContext::new().insert_discount(disc).insert_surface(vs);

    (swaption, ctx)
}

#[test]
fn price_black_matches_manual_formula() {
    let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let (swaption, ctx) = build_swaption(as_of);
    let pv_inst = swaption.value(&ctx, as_of).unwrap().amount();

    // Manual Black using pricer helpers for forward and annuity
    let disc = ctx
        .get_ref::<DiscountCurve>("USD-OIS")
        .unwrap();
    let pricer = finstack_valuations::instruments::swaption::pricing::SwaptionPricer;
    let t = pricer
        .year_fraction(as_of, swaption.expiry, swaption.day_count)
        .unwrap();
    let fwd = pricer.forward_swap_rate(&swaption, disc, as_of).unwrap();
    let ann = pricer.swap_annuity(&swaption, disc, as_of).unwrap();
    let vol = ctx
        .surface_ref("USD-SWAPTION-VOL")
        .unwrap()
        .value_clamped(t, swaption.strike_rate);
    let var = vol * vol * t;
    assert!(var > 0.0);
    let d1 = ((fwd / swaption.strike_rate).ln() + 0.5 * var) / var.sqrt();
    let d2 = d1 - var.sqrt();
    let expected = match swaption.option_type {
        finstack_valuations::instruments::OptionType::Call => {
            ann * (fwd * finstack_core::math::norm_cdf(d1) - swaption.strike_rate * finstack_core::math::norm_cdf(d2))
        }
        finstack_valuations::instruments::OptionType::Put => {
            ann * (swaption.strike_rate * finstack_core::math::norm_cdf(-d2) - fwd * finstack_core::math::norm_cdf(-d1))
        }
    } * swaption.notional.amount();

    let rel_err = ((pv_inst - expected) / expected.max(1.0)).abs();
    assert!(rel_err < 1e-10, "pv_inst={} expected={} rel_err={}", pv_inst, expected, rel_err);
}

#[test]
fn vega_matches_finite_difference_per_pct_vol() {
    let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let (mut swaption, ctx) = build_swaption(as_of);

    // Base vol from surface at as_of
    let pricer = finstack_valuations::instruments::swaption::pricing::SwaptionPricer;
    let t = pricer
        .year_fraction(as_of, swaption.expiry, swaption.day_count)
        .unwrap();
    let base_vol = ctx
        .surface_ref("USD-SWAPTION-VOL")
        .unwrap()
        .value_clamped(t, swaption.strike_rate);

    // Metric via registry
    let pv = swaption.value(&ctx, as_of).unwrap();
    let mut mctx = MetricContext::new(Arc::new(swaption.clone()), Arc::new(ctx.clone()), as_of, pv);
    let reg = standard_registry();
    let res = reg.compute(&[MetricId::Vega], &mut mctx).unwrap();
    let vega = *res.get(&MetricId::Vega).unwrap();

    // FD with +/- 1% absolute vol change
    let h = 0.01;
    swaption.pricing_overrides = PricingOverrides { implied_volatility: Some(base_vol + h), ..Default::default() };
    let pv_up = swaption.value(&ctx, as_of).unwrap().amount();
    swaption.pricing_overrides = PricingOverrides { implied_volatility: Some(base_vol - h), ..Default::default() };
    let pv_dn = swaption.value(&ctx, as_of).unwrap().amount();
    let fd = (pv_up - pv_dn) / 2.0; // per 1% change, already in currency units

    let diff = (vega - fd).abs();
    let scale = (vega.abs() + fd.abs()).max(1.0);
    assert!(diff / scale < 5e-4, "vega={} fd={} rel_diff={}", vega, fd, diff / scale);
}

#[test]
fn rho_matches_parallel_bump_difference() {
    let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let (swaption, ctx) = build_swaption(as_of);
    let pv = swaption.value(&ctx, as_of).unwrap();
    let mut mctx = MetricContext::new(Arc::new(swaption.clone()), Arc::new(ctx.clone()), as_of, pv);
    let reg = standard_registry();
    let res = reg.compute(&[MetricId::Rho], &mut mctx).unwrap();
    let rho = *res.get(&MetricId::Rho).unwrap();

    // Manual bump: +1bp parallel bump then scale to per 1%
    let disc = ctx
        .get_ref::<DiscountCurve>("USD-OIS")
        .unwrap();
    let bumped = disc.with_parallel_bump(1.0);
    // Build a new context with bumped discount replacing existing curve
    let mut bumped_ctx = ctx.clone();
    bumped_ctx = bumped_ctx.insert_discount(bumped);
    let dv_1bp = swaption.value(&bumped_ctx, as_of).unwrap().amount() - pv.amount();
    let rho_fd = dv_1bp * 100.0; // per 1%

    let rel_err = ((rho - rho_fd) / rho_fd.max(1.0)).abs();
    assert!(rel_err < 1e-6, "rho={} fd={} rel_err={}", rho, rho_fd, rel_err);
}

#[test]
fn theta_daily_bump_is_finite_and_runs() {
    let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let (swaption, ctx) = build_swaption(as_of);
    let pv = swaption.value(&ctx, as_of).unwrap();
    let mut mctx = MetricContext::new(Arc::new(swaption.clone()), Arc::new(ctx.clone()), as_of, pv);
    let reg = standard_registry();
    let res = reg.compute(&[MetricId::Theta], &mut mctx).unwrap();
    let theta = *res.get(&MetricId::Theta).unwrap();
    assert!(theta.is_finite());
}


