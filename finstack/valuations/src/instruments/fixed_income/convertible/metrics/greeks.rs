//! Greeks metrics for `ConvertibleBond`.
//!
//! Provides Delta, Gamma, Vega, Rho, Theta using the tree-based greeks from
//! the pricing engine.

use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

use crate::instruments::convertible::types::ConvertibleBond;

pub struct DeltaCalculator;
impl MetricCalculator for DeltaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let bond = context.instrument_as::<ConvertibleBond>()?;
        bond.delta(&context.curves, context.as_of)
    }
}

pub struct GammaCalculator;
impl MetricCalculator for GammaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let bond = context.instrument_as::<ConvertibleBond>()?;
        bond.gamma(&context.curves, context.as_of)
    }
}

pub struct VegaCalculator;
impl MetricCalculator for VegaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let bond = context.instrument_as::<ConvertibleBond>()?;
        bond.vega(&context.curves, context.as_of)
    }
}

pub struct RhoCalculator;
impl MetricCalculator for RhoCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let bond = context.instrument_as::<ConvertibleBond>()?;
        bond.rho(&context.curves, context.as_of)
    }
}

// Theta calculator is implemented in `metrics/theta.rs` to share a generic implementation.
