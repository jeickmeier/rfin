//! Fixed Income Index TRS pricing utilities.

use super::engine::{TotalReturnLegParams, TrsEngine, TrsReturnModel};
use crate::instruments::trs::FIIndexTotalReturnSwap;
use finstack_core::dates::{Date, DayCountCtx};
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::Result;

fn extract_underlying_data(
    trs: &FIIndexTotalReturnSwap,
    context: &MarketContext,
) -> Result<(f64, f64)> {
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

    Ok((index_yield, duration))
}

struct FiIndexReturnModel<'a> {
    trs: &'a FIIndexTotalReturnSwap,
    index_yield: f64,
    duration: f64,
}

impl TrsReturnModel for FiIndexReturnModel<'_> {
    fn period_return(
        &self,
        period_start: finstack_core::dates::Date,
        period_end: finstack_core::dates::Date,
        _t_start: f64,
        _t_end: f64,
        _initial_level: f64,
        _context: &MarketContext,
    ) -> Result<f64> {
        let ctx = DayCountCtx::default();
        let yf = self
            .trs
            .schedule
            .params
            .dc
            .year_fraction(period_start, period_end, ctx)?;
        let carry_return = self.index_yield * yf;
        let roll_return = if self.duration > 0.0 {
            let yield_change_estimate = -0.0001 * yf;
            self.duration * yield_change_estimate
        } else {
            0.0
        };
        Ok(carry_return + roll_return)
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
/// This implementation uses a carry-only approximation for the index return calculation.
pub fn pv_total_return_leg(
    trs: &FIIndexTotalReturnSwap,
    context: &MarketContext,
    as_of: Date,
) -> Result<Money> {
    let (index_yield, duration) = extract_underlying_data(trs, context)?;

    let params = TotalReturnLegParams {
        schedule: &trs.schedule,
        notional: trs.notional,
        discount_curve_id: trs.financing.discount_curve_id.as_str(),
        contract_size: trs.underlying.contract_size,
        initial_level: trs.initial_level,
    };

    let model = FiIndexReturnModel {
        trs,
        index_yield,
        duration,
    };
    TrsEngine::pv_total_return_leg_with_model(params, context, as_of, &model)
}
