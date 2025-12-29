//! Equity TRS pricing - dividend yield forward model.
//!
//! This module implements the total return leg pricing for equity TRS using
//! a cost-of-carry forward model with dividend yield.

use super::types::EquityTotalReturnSwap;
use crate::instruments::common::pricing::{TotalReturnLegParams, TrsEngine, TrsReturnModel};
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::scalars::MarketScalar;
use finstack_core::money::Money;
use finstack_core::Result;

/// Extracts spot price and dividend yield from market data.
fn extract_underlying_data(
    trs: &EquityTotalReturnSwap,
    context: &MarketContext,
) -> Result<(f64, f64)> {
    let spot = match context.price(&trs.underlying.spot_id)? {
        MarketScalar::Unitless(v) => *v,
        MarketScalar::Price(p) => p.amount(),
    };

    let div_yield = trs
        .underlying
        .div_yield_id
        .as_ref()
        .and_then(|id| {
            context.price(id.as_str()).ok().map(|s| match s {
                MarketScalar::Unitless(v) => *v,
                MarketScalar::Price(p) => p.amount(),
            })
        })
        .unwrap_or(0.0);

    Ok((spot, div_yield))
}

/// Equity-specific return model using cost-of-carry forward pricing.
///
/// Models the total return as:
/// - **Price return**: Forward price change using F_t = S_0 * e^{(r-q)t}
/// - **Dividend return**: Continuous dividend yield approximation (q * dt)
struct EquityReturnModel<'a> {
    trs: &'a EquityTotalReturnSwap,
    div_yield: f64,
}

impl TrsReturnModel for EquityReturnModel<'_> {
    fn period_return(
        &self,
        _period_start: Date,
        _period_end: Date,
        t_start: f64,
        t_end: f64,
        initial_level: f64,
        context: &MarketContext,
    ) -> Result<f64> {
        let disc = context.get_discount_ref(self.trs.financing.discount_curve_id.as_str())?;
        let df_start = disc.df(t_start);
        let df_end = disc.df(t_end);

        // Price return component (Forward Price change)
        // F_t = S_0 * e^{(r-q)t}
        let fwd_start = initial_level * df_start.recip() * (-self.div_yield * t_start).exp();
        let fwd_end = initial_level * df_end.recip() * (-self.div_yield * t_end).exp();
        let price_return = (fwd_end - fwd_start) / fwd_start;

        // Dividend return component (Income)
        // Approx: q * dt
        // For a Total Return Swap, we pay Price Change + Dividends.
        // This simplistic model assumes 100% dividend pass-through (Gross Return).
        let dt = t_end - t_start;
        let dividend_return = self.div_yield * dt;

        Ok(price_return + dividend_return)
    }
}

/// Calculates the present value of the total return leg for an equity TRS.
///
/// Uses a dividend yield forward model where the forward price is:
/// ```text
/// F_t = S_0 * e^{(r - q) * t}
/// ```
///
/// Total return = Price return + Dividend return
///
/// # Arguments
/// * `trs` — The equity TRS instrument
/// * `context` — Market context containing curves and market data
/// * `as_of` — Valuation date
///
/// # Returns
/// Present value of the total return leg in the instrument's currency.
///
/// # Errors
/// Returns an error if:
/// - The spot price cannot be fetched from market data
/// - The initial level is non-positive or non-finite
/// - The discount curve is not found
pub fn pv_total_return_leg(
    trs: &EquityTotalReturnSwap,
    context: &MarketContext,
    as_of: Date,
) -> Result<Money> {
    let (spot, div_yield) = extract_underlying_data(trs, context)?;
    let initial = trs.initial_level.unwrap_or(spot);

    if !initial.is_finite() || initial <= 0.0 {
        return Err(finstack_core::InputError::Invalid.into());
    }

    let params = TotalReturnLegParams {
        schedule: &trs.schedule,
        notional: trs.notional,
        discount_curve_id: trs.financing.discount_curve_id.as_str(),
        contract_size: trs.underlying.contract_size,
        initial_level: Some(initial),
    };

    let model = EquityReturnModel { trs, div_yield };
    TrsEngine::pv_total_return_leg_with_model(params, context, as_of, &model)
}
