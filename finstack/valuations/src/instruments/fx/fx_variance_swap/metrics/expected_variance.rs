//! Expected variance metric (blend of realized and forward).

use super::super::types::FxVarianceSwap;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Calculate the expected variance (blend of realized and forward).
pub struct ExpectedVarianceCalculator;

impl MetricCalculator for ExpectedVarianceCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let swap = context.instrument_as::<FxVarianceSwap>()?;
        let as_of = context.as_of;

        if as_of >= swap.maturity {
            return swap.partial_realized_variance(&context.curves, as_of);
        }

        if as_of < swap.start_date {
            return swap.remaining_forward_variance(&context.curves, as_of);
        }

        let realized = swap.partial_realized_variance(&context.curves, as_of)?;
        let forward = swap.remaining_forward_variance(&context.curves, as_of)?;
        let w = swap.realized_fraction_by_observations(as_of);

        Ok(realized * w + forward * (1.0 - w))
    }
}
