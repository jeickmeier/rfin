//! Inflation01 (inflation rate sensitivity) metric for `InflationSwap`.

use crate::instruments::inflation_swap::{InflationSwap, PayReceiveInflation};
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::prelude::*;

/// Calculates Inflation01 (1bp inflation rate sensitivity) for inflation swaps.
pub struct Inflation01Calculator;

impl MetricCalculator for Inflation01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let s: &InflationSwap = context.instrument_as()?;

        let disc = context
            .curves
            .get_discount_ref(s.discount_curve_id.as_str())?;
        let base = disc.base_date();

        // Get Index Ratio using central logic
        let index_ratio = s.projected_index_ratio(&context.curves, base)?;

        // Calculate T (time to lagged maturity) for sensitivity
        // We need default lag from index if not overridden
        let inflation_index = context
            .curves
            .inflation_index_ref(s.inflation_index_id.as_str());

        let default_lag = inflation_index
            .map(|i| i.lag())
            .unwrap_or(finstack_core::market_data::scalars::inflation_index::InflationLag::None);

        let lagged_maturity = s.lagged_maturity_date(default_lag);

        let t_maturity = DayCount::Act365F
            .year_fraction(
                base,
                lagged_maturity,
                finstack_core::dates::DayCountCtx::default(),
            )
            .unwrap_or(0.0);

        let t_discount = DayCount::Act365F
            .year_fraction(
                base,
                s.maturity,
                finstack_core::dates::DayCountCtx::default(),
            )
            .unwrap_or(0.0);
        let df = disc.df(t_discount);

        let inflation_sensitivity = s.notional.amount() * index_ratio * df * t_maturity * 0.0001;

        let signed_sensitivity = match s.side {
            PayReceiveInflation::PayFixed => inflation_sensitivity,
            PayReceiveInflation::ReceiveFixed => -inflation_sensitivity,
        };

        Ok(signed_sensitivity)
    }
}
