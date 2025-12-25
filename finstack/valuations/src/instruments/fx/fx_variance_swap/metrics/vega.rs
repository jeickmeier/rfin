//! Vega metric for FX variance swaps (per 1% volatility move).

use super::super::types::FxVarianceSwap;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Calculate vega (sensitivity to 1% change in volatility).
pub struct VegaCalculator;

impl MetricCalculator for VegaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let swap = context.instrument_as::<FxVarianceSwap>()?;

        let current_vol = swap
            .remaining_forward_variance(&context.curves, context.as_of)
            .map(|v| v.sqrt())
            .unwrap_or_else(|_| swap.strike_variance.sqrt());

        let remaining_fraction = 1.0 - swap.realized_fraction_by_observations(context.as_of);

        let t = swap
            .day_count
            .year_fraction(context.as_of, swap.maturity, Default::default())?;
        let disc = context
            .curves
            .get_discount_ref(swap.domestic_discount_curve_id.as_str())?;
        let df = disc.df(t);

        let vega = df * 2.0 * swap.notional.amount() * current_vol * 0.01 * remaining_fraction;
        Ok(vega * swap.side.sign())
    }
}
