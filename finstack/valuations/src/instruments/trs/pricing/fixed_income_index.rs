//! Fixed Income Index TRS pricing utilities.

use super::engine::{TotalReturnLegParams, TrsEngine, TrsReturnModel};
use crate::instruments::trs::FIIndexTotalReturnSwap;
use finstack_core::dates::Date;
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::Result;

fn extract_underlying_data(
    trs: &FIIndexTotalReturnSwap,
    context: &MarketContext,
) -> Result<(f64, f64, f64)> {
    // Try to get index spot price, default to 100.0 if not found (cancels out for % returns)
    let spot = match context.price(trs.underlying.index_id.as_str()) {
        Ok(s) => match s {
            finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
            finstack_core::market_data::scalars::MarketScalar::Price(p) => p.amount(),
        },
        Err(_) => 100.0,
    };

    let index_yield = trs
        .underlying
        .yield_id
        .as_ref()
        .and_then(|id| {
            context.price(id.as_str()).ok().map(|s| match s {
                finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
                finstack_core::market_data::scalars::MarketScalar::Price(p) => p.amount(),
            })
        })
        .unwrap_or(0.0);

    let duration = trs
        .underlying
        .duration_id
        .as_ref()
        .and_then(|id| {
            context.price(id.as_str()).ok().map(|s| match s {
                finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
                finstack_core::market_data::scalars::MarketScalar::Price(p) => p.amount(),
            })
        })
        .unwrap_or(0.0);

    Ok((spot, index_yield, duration))
}

struct FiIndexReturnModel<'a> {
    trs: &'a FIIndexTotalReturnSwap,
    spot: f64,
    index_yield: f64,
    #[allow(dead_code)] // Kept for future risk expansions or hybrid models
    duration: f64,
}

impl TrsReturnModel for FiIndexReturnModel<'_> {
    fn period_return(
        &self,
        _period_start: finstack_core::dates::Date,
        _period_end: finstack_core::dates::Date,
        t_start: f64,
        t_end: f64,
        _initial_level: f64,
        context: &MarketContext,
    ) -> Result<f64> {
        // Market Standard Model: Forward Price + Income
        // We model the index as an asset paying continuous income (yield).
        // F_t = S_0 * e^{(r - y)t}
        // Price Return = (F_end - F_start) / F_start
        // Income Return = y * dt

        let disc = context.get_discount_ref(self.trs.financing.discount_curve_id.as_str())?;
        let df_start = disc.df(t_start);
        let df_end = disc.df(t_end);

        // Use the captured spot price (S_0) for projection base
        // Note: for percentage returns, S_0 cancels out, but we use it for correctness
        let s0 = self.spot;

        // Projected Forward Prices
        // F_t = S_0 * (1/DF_t) * e^(-y*t)
        let fwd_start = s0 * df_start.recip() * (-self.index_yield * t_start).exp();
        let fwd_end = s0 * df_end.recip() * (-self.index_yield * t_end).exp();

        let price_return = (fwd_end - fwd_start) / fwd_start;

        // Income Return (Coupons/Carry)
        // Approx: y * dt
        let dt = t_end - t_start;
        let income_return = self.index_yield * dt;

        Ok(price_return + income_return)
    }
}

/// Calculates the present value of the total return leg for a fixed income index TRS.
///
/// # Arguments
/// * `trs` — The fixed income index TRS instrument
/// * `context` — Market context containing curves and market data
/// * `as_of` — Valuation date
///
/// # Returns
/// Present value of the total return leg in the instrument's currency.
///
/// # Note
/// This implementation uses a Forward Price + Income model (Arbitrage-Free),
/// treating the index yield as a continuous income rate.
pub fn pv_total_return_leg(
    trs: &FIIndexTotalReturnSwap,
    context: &MarketContext,
    as_of: Date,
) -> Result<Money> {
    let (spot, index_yield, duration) = extract_underlying_data(trs, context)?;

    let params = TotalReturnLegParams {
        schedule: &trs.schedule,
        notional: trs.notional,
        discount_curve_id: trs.financing.discount_curve_id.as_str(),
        contract_size: trs.underlying.contract_size,
        initial_level: trs.initial_level,
    };

    let model = FiIndexReturnModel {
        trs,
        spot,
        index_yield,
        duration,
    };
    TrsEngine::pv_total_return_leg_with_model(params, context, as_of, &model)
}
