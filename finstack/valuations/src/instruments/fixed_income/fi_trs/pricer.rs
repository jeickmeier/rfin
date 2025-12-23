//! Fixed Income Index TRS pricing - yield/carry model.
//!
//! This module implements the total return leg pricing for fixed income index TRS
//! using a forward price plus income model.

use super::types::FIIndexTotalReturnSwap;
use crate::instruments::common::pricing::{TotalReturnLegParams, TrsEngine, TrsReturnModel};
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::scalars::MarketScalar;
use finstack_core::money::Money;
use finstack_core::Result;

/// Extracts spot price and index yield from market data.
fn extract_underlying_data(
    trs: &FIIndexTotalReturnSwap,
    context: &MarketContext,
) -> Result<(f64, f64)> {
    // Try to get index spot price, default to 100.0 if not found (cancels out for % returns)
    let spot = match context.price(trs.underlying.index_id.as_str()) {
        Ok(s) => match s {
            MarketScalar::Unitless(v) => *v,
            MarketScalar::Price(p) => p.amount(),
        },
        Err(_) => 100.0,
    };

    let index_yield = trs
        .underlying
        .yield_id
        .as_ref()
        .and_then(|id| {
            context.price(id.as_str()).ok().map(|s| match s {
                MarketScalar::Unitless(v) => *v,
                MarketScalar::Price(p) => p.amount(),
            })
        })
        .unwrap_or(0.0);

    Ok((spot, index_yield))
}

/// Fixed income index return model using forward price plus income.
///
/// Models the total return as:
/// - **Price return**: Forward price change using F_t = S_0 * e^{(r-y)t}
/// - **Income return**: Continuous yield approximation (y * dt)
struct FiIndexReturnModel<'a> {
    trs: &'a FIIndexTotalReturnSwap,
    spot: f64,
    index_yield: f64,
}

impl TrsReturnModel for FiIndexReturnModel<'_> {
    fn period_return(
        &self,
        _period_start: Date,
        _period_end: Date,
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
/// Uses a forward price plus income model where:
/// ```text
/// F_t = S_0 * e^{(r - y) * t}
/// Total return = Price return + Income return
/// ```
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
/// This implementation uses an arbitrage-free Forward Price + Income model,
/// treating the index yield as a continuous income rate.
pub fn pv_total_return_leg(
    trs: &FIIndexTotalReturnSwap,
    context: &MarketContext,
    as_of: Date,
) -> Result<Money> {
    let (spot, index_yield) = extract_underlying_data(trs, context)?;

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
    };
    TrsEngine::pv_total_return_leg_with_model(params, context, as_of, &model)
}
