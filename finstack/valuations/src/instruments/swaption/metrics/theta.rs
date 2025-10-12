//! Theta calculator for swaptions (customizable period bump-in-time).
//!
//! Computes theta via bump-and-reprice at `as_of + period` (default 1D),
//! holding market curves and vol surface fixed.

use crate::instruments::common::metrics::theta_utils;
use crate::instruments::swaption::Swaption;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::prelude::Result;

/// Theta calculator for swaptions (customizable period)
pub struct ThetaCalculator;

impl MetricCalculator for ThetaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        theta_utils::generic_theta_calculator::<Swaption>(context)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}
