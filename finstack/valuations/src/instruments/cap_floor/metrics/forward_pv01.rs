//! Forward curve PV01 for interest rate options (per 1bp parallel bump of forward curve).

use crate::instruments::cap_floor::InterestRateOption;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::market_data::bumps::BumpSpec;
use finstack_core::Result;

/// Forward PV01 calculator (per 1bp parallel forward curve bump)
pub struct ForwardPv01Calculator;

impl MetricCalculator for ForwardPv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &InterestRateOption = context.instrument_as()?;

        // Base PV from context
        let base = context.base_value.amount();

        // Bump the forward curve by +1bp
        let mut bumps = hashbrown::HashMap::new();
        bumps.insert(option.forward_id.clone(), BumpSpec::parallel_bp(1.0));
        let bumped_ctx = context.curves.bump(bumps)?;

        // Reprice with bumped forward curve
        let bumped = option.npv(&bumped_ctx, context.as_of)?;

        Ok(bumped.amount() - base)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}
