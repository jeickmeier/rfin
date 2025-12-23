//! Forward curve PV01 for interest rate options (per 1bp parallel bump of forward curve).

use crate::instruments::cap_floor::InterestRateOption;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::market_data::term_structures::ForwardCurve;
use finstack_core::Result;

/// Forward PV01 calculator (per 1bp parallel forward curve bump)
pub struct ForwardPv01Calculator;

impl MetricCalculator for ForwardPv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &InterestRateOption = context.instrument_as()?;

        // Base PV from context
        let base = context.base_value.amount();

        // Get the original forward curve
        let original_fwd = context.curves.get_forward_ref(&option.forward_id)?;

        // Create bumped curve with +1bp, keeping the SAME ID as original
        let bump_amount = 0.0001; // 1bp as fraction

        let bumped_rates: Vec<(f64, f64)> = original_fwd
            .knots()
            .iter()
            .copied()
            .zip(original_fwd.forwards().iter().copied())
            .map(|(t, r)| (t, r + bump_amount))
            .collect();

        // Build bumped curve with ORIGINAL ID so instrument can find it
        let bumped_fwd = ForwardCurve::builder(option.forward_id.clone(), original_fwd.tenor())
            .base_date(original_fwd.base_date())
            .reset_lag(original_fwd.reset_lag())
            .day_count(original_fwd.day_count())
            .knots(bumped_rates)
            .build()?;

        // Create new context with bumped curve (replaces original with same ID)
        let bumped_ctx = context.curves.as_ref().clone().insert_forward(bumped_fwd);

        // Reprice with bumped forward curve
        let bumped = option.npv(&bumped_ctx, context.as_of)?;

        Ok(bumped.amount() - base)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}
