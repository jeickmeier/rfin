//! Vega metric for variance swaps (per 1% volatility move).

use super::super::types::VarianceSwap;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::{Result};

/// Calculate vega (sensitivity to 1% change in volatility).
pub struct VegaCalculator;

impl MetricCalculator for VegaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let swap = context.instrument_as::<VarianceSwap>()?;

        // Try to get current implied vol; otherwise approximate using strike
        let current_vol = if let Ok(scalar) = context
            .curves
            .price(format!("{}_IMPL_VOL", swap.underlying_id))
        {
            match scalar {
                finstack_core::market_data::scalars::MarketScalar::Unitless(vol) => *vol,
                finstack_core::market_data::scalars::MarketScalar::Price(price) => price.amount(),
            }
        } else {
            swap.strike_variance.sqrt()
        };

        // Remaining fraction of observations
        let remaining_fraction = 1.0 - swap.realized_fraction_by_observations(context.as_of);

        // Discount factor to maturity
        let t = swap
            .day_count
            .year_fraction(context.as_of, swap.maturity, Default::default())?;
        let disc = context.curves.get_discount_ref(swap.disc_id.as_str())?;
        let df = disc.df(t);

        // Vega per 1% vol move: DF * 2 * Notional * sigma * 0.01 * remaining_fraction
        let vega = df * 2.0 * swap.notional.amount() * current_vol * 0.01 * remaining_fraction;
        Ok(vega * swap.side.sign())
    }
}
