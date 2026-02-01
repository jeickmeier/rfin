//! Rho calculator for interest rate sensitivity.
//!
//! Calculates the sensitivity of the instrument's value to a parallel shift
//! in the discount curve (and optionally forward curves).
//!
//! This is a specialized wrapper around [`UnifiedDv01Calculator`] configured
//! for parallel bumps (1bp default).

use crate::instruments::common::traits::{CurveDependencies, Instrument};
use crate::metrics::sensitivities::dv01::{Dv01CalculatorConfig, UnifiedDv01Calculator};
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use std::marker::PhantomData;

/// Generic Rho calculator using parallel finite difference.
///
/// Wraps [`UnifiedDv01Calculator`] with a configuration optimized for Rho:
/// - Mode: Parallel Combined (single scalar result)
/// - Curves: All rate curves (Discount + Forward)
/// - Bump: 1bp (0.0001) default
pub struct GenericRho<I> {
    inner: UnifiedDv01Calculator<I>,
    _phantom: PhantomData<I>,
}

impl<I> GenericRho<I> {
    /// Create a new Generic Rho calculator.
    pub fn new() -> Self {
        Self {
            inner: UnifiedDv01Calculator::new(Dv01CalculatorConfig::parallel_combined()),
            _phantom: PhantomData,
        }
    }
}

impl<I> Default for GenericRho<I> {
    fn default() -> Self {
        Self::new()
    }
}

impl<I> MetricCalculator for GenericRho<I>
where
    I: Instrument + CurveDependencies + 'static,
{
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        self.inner.calculate(context)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}
