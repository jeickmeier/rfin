//! CDS Tranche metrics (boilerplate registrations).
//!
//! Placeholder calculators for tranche-specific measures to enable integration
//! with the metrics registry. Real implementations should use an index loss
//! model (e.g., base correlation / Gaussian copula) and survival/loss curves.

use crate::instruments::Instrument;
use crate::metrics::{MetricCalculator, MetricContext, MetricId, MetricRegistry};
use finstack_core::F;

/// Upfront payment metric (placeholder). Returns 0.0 until model exists.
pub struct Upfront;

impl MetricCalculator for Upfront {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        match &*context.instrument {
            Instrument::CDSTranche(_) => Ok(0.0),
            _ => Err(finstack_core::Error::from(
                finstack_core::error::InputError::Invalid,
            )),
        }
    }
}

/// Spread DV01 (premium leg PV change for 1bp change in running coupon)
/// Placeholder returning 0.0.
pub struct SpreadDv01;

impl MetricCalculator for SpreadDv01 {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        match &*context.instrument {
            Instrument::CDSTranche(_) => Ok(0.0),
            _ => Err(finstack_core::Error::from(
                finstack_core::error::InputError::Invalid,
            )),
        }
    }
}

/// Expected loss of the tranche (placeholder 0.0)
pub struct ExpectedLoss;

impl MetricCalculator for ExpectedLoss {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        match &*context.instrument {
            Instrument::CDSTranche(_) => Ok(0.0),
            _ => Err(finstack_core::Error::from(
                finstack_core::error::InputError::Invalid,
            )),
        }
    }
}

/// Jump-to-default (instantaneous loss sensitivity, placeholder 0.0)
pub struct JumpToDefault;

impl MetricCalculator for JumpToDefault {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        match &*context.instrument {
            Instrument::CDSTranche(_) => Ok(0.0),
            _ => Err(finstack_core::Error::from(
                finstack_core::error::InputError::Invalid,
            )),
        }
    }
}

/// Registers placeholder CDS Tranche metrics
pub fn register_cds_tranche_metrics(registry: &mut MetricRegistry) {
    use std::sync::Arc;

    registry
        .register_metric(MetricId::custom("upfront"), Arc::new(Upfront), &["CDSTranche"]) 
        .register_metric(MetricId::custom("spread_dv01"), Arc::new(SpreadDv01), &["CDSTranche"]) 
        .register_metric(MetricId::ExpectedLoss, Arc::new(ExpectedLoss), &["CDSTranche"]) 
        .register_metric(MetricId::JumpToDefault, Arc::new(JumpToDefault), &["CDSTranche"]);
}


