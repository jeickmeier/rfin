//! Implied volatility metric for `CdsOption`.
//!
//! Computes the Black-on-spreads implied volatility that matches the
//! instrument's current PV (`context.base_value`) using the CDS option
//! pricer and core math solvers (HybridSolver).

use crate::instruments::credit_derivatives::cds_option::CdsOption;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::Result;

/// Implied Volatility calculator for credit options on CDS spreads.
pub struct ImpliedVolCalculator;

impl MetricCalculator for ImpliedVolCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &CdsOption = context.instrument_as()?;
        let target = context.base_value.amount();
        option.implied_vol(&context.curves, context.as_of, target, None)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}
