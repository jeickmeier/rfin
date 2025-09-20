//! Implied volatility metric for `CdsOption`.
//!
//! Computes the Black-on-spreads implied volatility that matches the
//! instrument's current PV (`context.base_value`) using the CDS option
//! pricer and core math solvers (HybridSolver).

use crate::instruments::cds_option::CdsOption;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::{Result, F};

/// Implied Volatility calculator for credit options on CDS spreads.
pub struct ImpliedVolCalculator;

impl MetricCalculator for ImpliedVolCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let option: &CdsOption = context.instrument_as()?;
        let pricer = crate::instruments::cds_option::pricing::engine::CdsOptionPricer::default();
        let as_of = context.as_of;
        let target = context.base_value.amount();
        let iv = pricer.implied_vol(option, &context.curves, as_of, target, None)?;
        Ok(iv)
    }

    fn dependencies(&self) -> &[MetricId] { &[] }
}


