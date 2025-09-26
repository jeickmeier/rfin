//! Variance swap pricing engine.
//!
//! Implements deterministic PV for `VarianceSwap` by blending realized and
//! forward variance according to elapsed time and discounting the payoff to
//! present using the specified discount curve.

use finstack_core::dates::Date;
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::Result;

use crate::instruments::variance_swap::types::VarianceSwap;

/// Price a variance swap as of `as_of` using curves in `context`.
pub fn price(inst: &VarianceSwap, context: &MarketContext, as_of: Date) -> Result<Money> {
    // Get discount curve
    let disc = context.get_discount_ref(inst.disc_id.as_str())?;

    // If expired, compute final payoff from realized variance (if any prices)
    if as_of >= inst.maturity {
        let prices = inst.get_historical_prices(context, as_of)?;
        if prices.is_empty() {
            return Ok(Money::new(0.0, inst.notional.currency()));
        }

        let realized_var = finstack_core::math::stats::realized_variance(
            &prices,
            inst.realized_var_method,
            inst.annualization_factor(),
        );
        return Ok(inst.payoff(realized_var));
    }

    // If not yet started, value using forward (implied) variance and discount
    if as_of < inst.start_date {
        let forward_var = inst.remaining_forward_variance(context, as_of)?;
        let undiscounted = inst.payoff(forward_var);
        let t = inst
            .day_count
            .year_fraction(as_of, inst.maturity, Default::default())?;
        let df = disc.df(t);
        return Ok(undiscounted * df);
    }

    // Partially observed: blend realized-to-date with forward for remaining using observation counts
    let realized = inst.partial_realized_variance(context, as_of)?;
    let forward = inst.remaining_forward_variance(context, as_of)?;
    let w = inst.realized_fraction_by_observations(as_of);
    let expected_var = realized * w + forward * (1.0 - w);
    let undiscounted = inst.payoff(expected_var);

    let t = inst
        .day_count
        .year_fraction(as_of, inst.maturity, Default::default())?;
    let df = disc.df(t);
    Ok(undiscounted * df)
}
