//! Implied volatility calculator for equity options.
//!
//! Solves for σ such that model price(σ) equals a provided market price. The
//! market price can be supplied via instrument attributes:
//! - `market_price`: numeric value as string
//! - `market_price_id`: id of a scalar in `MarketContext`

use crate::instruments::equity_option::EquityOption;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::{Result, F};

const MIN_VOL: F = 1e-6;
const MAX_VOL_BRACKET: F = 10.0;
const SOLVER_TOL: F = 1e-8;
const SOLVER_MAX_ITER: usize = 100;

pub struct ImpliedVolCalculator;

impl MetricCalculator for ImpliedVolCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let option: &EquityOption = context.instrument_as()?;

        let t = option.day_count.year_fraction(
            context.as_of,
            option.expiry,
            finstack_core::dates::DayCountCtx::default(),
        )?;
        if t <= 0.0 { return Ok(0.0); }

        // Collect inputs except vol
        let (spot, r, q, _sigma, _t) = {
            let (spot, r, q, sigma, t) = crate::instruments::equity_option::pricing::engine::collect_inputs(
                option,
                &context.curves,
                context.as_of,
            )?;
            (spot, r, q, sigma, t)
        };

        // Market price
        let market_price: F = if let Some(p) = option.attributes.get_meta("market_price") {
            p.parse().unwrap_or(0.0)
        } else if let Some(price_id) = option.attributes.get_meta("market_price_id") {
            match context.curves.price(price_id) {
                Ok(ms) => match ms {
                    finstack_core::market_data::scalars::MarketScalar::Unitless(val) => *val,
                    finstack_core::market_data::scalars::MarketScalar::Price(m) => m.amount(),
                },
                Err(_) => 0.0,
            }
        } else { 0.0 };

        if market_price <= 0.0 { return Ok(0.0); }

        // Solve for sigma using bracketed bisection with guarded Newton improvement
        let k = option.strike.amount();
        let price_at = |sigma: F| -> F {
            if sigma <= 0.0 { return 0.0; }
            crate::instruments::equity_option::pricing::engine::price_bs_unit(
                spot, k, r, q, sigma, t, option.option_type,
            ) * option.contract_size
        };

        let mut lo = MIN_VOL;
        let mut hi = 3.0;
        let tol = SOLVER_TOL;
        let max_iter = SOLVER_MAX_ITER;

        let mut f_lo = price_at(lo) - market_price;
        let mut f_hi = price_at(hi) - market_price;
        if f_lo * f_hi > 0.0 {
            let mut tries = 0;
            while f_lo * f_hi > 0.0 && hi < MAX_VOL_BRACKET && tries < 10 {
                hi *= 1.5;
                f_hi = price_at(hi) - market_price;
                tries += 1;
            }
            if f_lo * f_hi > 0.0 { return Ok(0.0); }
        }

        let mut mid = 0.5 * (lo + hi);
        for _ in 0..max_iter {
            mid = 0.5 * (lo + hi);
            let f_mid = price_at(mid) - market_price;
            if f_mid.abs() < tol || (hi - lo) < tol { return Ok(mid); }

            // Guarded Newton step using closed-form vega
            let vega_per_1pct = {
                let d1 = crate::instruments::models::d1(spot, k, r, mid, t, q);
                let exp_q_t = (-q * t).exp();
                let sqrt_t = t.sqrt();
                spot * exp_q_t * finstack_core::math::norm_pdf(d1) * sqrt_t / 100.0
            } * option.contract_size;
            let vega_abs = vega_per_1pct * 100.0;
            if vega_abs.abs() > 1e-12 {
                let newton = mid - f_mid / vega_abs;
                if newton.is_finite() && newton > lo && newton < hi {
                    mid = newton;
                    let f_new = price_at(mid) - market_price;
                    if f_lo * f_new <= 0.0 { hi = mid; } else { lo = mid; f_lo = f_new; }
                    continue;
                }
            }

            if f_lo * f_mid <= 0.0 { hi = mid; } else { lo = mid; f_lo = f_mid; }
        }

        Ok(mid)
    }

    fn dependencies(&self) -> &[MetricId] { &[] }
}


