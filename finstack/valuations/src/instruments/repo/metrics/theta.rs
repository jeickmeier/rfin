//! Theta calculator for repo instruments.

use crate::instruments::common::metrics::theta_utils;
use crate::instruments::repo::Repo;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::Result;

pub struct ThetaCalculator;

impl MetricCalculator for ThetaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        theta_utils::generic_theta_calculator::<Repo>(context)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}
