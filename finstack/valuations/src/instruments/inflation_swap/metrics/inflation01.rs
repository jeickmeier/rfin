//! Inflation01 (inflation rate sensitivity) metric for `InflationSwap`.

use crate::instruments::inflation_swap::{InflationSwap, PayReceiveInflation};
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::market_data::scalars::inflation_index::InflationLag;
use finstack_core::prelude::*;

/// Calculates Inflation01 (1bp inflation rate sensitivity) for inflation swaps.
///
/// Computes the change in PV for a 1bp parallel shift in inflation expectations.
/// Uses the curve's day count for time calculations.
pub struct Inflation01Calculator;

impl MetricCalculator for Inflation01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let s: &InflationSwap = context.instrument_as()?;

        let disc = context
            .curves
            .get_discount_ref(s.discount_curve_id.as_str())?;
        let base = disc.base_date();
        let curve_dc = disc.day_count();

        // Get Index Ratio using central logic
        let index_ratio = s.projected_index_ratio(&context.curves, base)?;

        // Calculate T (time to lagged maturity) for sensitivity
        // Use the effective lag (instrument override or index default)
        let inflation_index = context
            .curves
            .inflation_index_ref(s.inflation_index_id.as_str());

        let default_lag = s
            .lag_override
            .or_else(|| inflation_index.map(|i| i.lag()))
            .unwrap_or(InflationLag::Months(3)); // Standard 3-month lag default

        let lagged_maturity = s.apply_lag(s.maturity, default_lag);

        let t_maturity = DayCount::Act365F.year_fraction(
            base,
            lagged_maturity,
            finstack_core::dates::DayCountCtx::default(),
        )?;

        // Use curve day count for discounting
        let t_discount = curve_dc.year_fraction(
            base,
            s.maturity,
            finstack_core::dates::DayCountCtx::default(),
        )?;
        let df = disc.df(t_discount);

        let inflation_sensitivity = s.notional.amount() * index_ratio * df * t_maturity * 0.0001;

        let signed_sensitivity = match s.side {
            PayReceiveInflation::PayFixed => inflation_sensitivity,
            PayReceiveInflation::ReceiveFixed => -inflation_sensitivity,
        };

        Ok(signed_sensitivity)
    }
}
