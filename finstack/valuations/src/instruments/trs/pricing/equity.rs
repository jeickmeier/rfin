//! Equity TRS pricing utilities.

use super::engine::{TotalReturnLegParams, TrsEngine, TrsReturnModel};
use crate::instruments::trs::EquityTotalReturnSwap;
use finstack_core::dates::Date;
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::Result;

fn extract_underlying_data(
    trs: &EquityTotalReturnSwap,
    context: &MarketContext,
) -> Result<(f64, f64)> {
    let spot = match context.price(&trs.underlying.spot_id)? {
        finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
        finstack_core::market_data::scalars::MarketScalar::Price(p) => p.amount(),
    };

    let div_yield = trs
        .underlying
        .dividend_yield_id
        .as_ref()
        .and_then(|id| {
            context.price(id.as_str()).ok().map(|s| match s {
                finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
                finstack_core::market_data::scalars::MarketScalar::Price(p) => p.amount(),
            })
        })
        .unwrap_or(0.0);

    Ok((spot, div_yield))
}

struct EquityReturnModel<'a> {
    trs: &'a EquityTotalReturnSwap,
    div_yield: f64,
}

impl TrsReturnModel for EquityReturnModel<'_> {
    fn period_return(
        &self,
        _period_start: finstack_core::dates::Date,
        _period_end: finstack_core::dates::Date,
        t_start: f64,
        t_end: f64,
        initial_level: f64,
        context: &MarketContext,
    ) -> Result<f64> {
        let disc = context
            .get_ref::<finstack_core::market_data::term_structures::discount_curve::DiscountCurve>(
            self.trs.financing.disc_id.as_str(),
        )?;
        let df_start = disc.df(t_start);
        let df_end = disc.df(t_end);
        let fwd_start = initial_level * df_start.recip() * (-self.div_yield * t_start).exp();
        let fwd_end = initial_level * df_end.recip() * (-self.div_yield * t_end).exp();
        Ok((fwd_end - fwd_start) / fwd_start)
    }
}

/// Calculates the present value of the total return leg for an equity TRS.
///
/// # Arguments
/// * `trs` — The equity TRS instrument
/// * `context` — Market context containing curves and market data
/// * `as_of` — Valuation date
///
/// # Returns
/// Present value of the total return leg in the instrument's currency.
pub fn pv_total_return_leg(
    trs: &EquityTotalReturnSwap,
    context: &MarketContext,
    as_of: Date,
) -> Result<Money> {
    let (spot, div_yield) = extract_underlying_data(trs, context)?;
    let initial = trs.initial_level.unwrap_or(spot);

    let params = TotalReturnLegParams {
        schedule: &trs.schedule,
        notional: trs.notional,
        disc_id: trs.financing.disc_id.as_str(),
        contract_size: trs.underlying.contract_size,
        initial_level: Some(initial),
    };

    let model = EquityReturnModel { trs, div_yield };
    TrsEngine::pv_total_return_leg_with_model(params, context, as_of, &model)
}
