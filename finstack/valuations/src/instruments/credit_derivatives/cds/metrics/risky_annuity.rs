//! CDS risky annuity metric calculator.
//!
//! Computes the risky annuity (premium leg PV per 1bp) using the CDS pricer.

use crate::instruments::credit_derivatives::cds::pricer::CDSPricer;
use crate::instruments::credit_derivatives::cds::CreditDefaultSwap;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Risky annuity calculator for CDS
pub struct RiskyAnnuityCalculator;

impl MetricCalculator for RiskyAnnuityCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let cds: &CreditDefaultSwap = context.instrument_as()?;
        let disc = context
            .curves
            .get_discount(&cds.premium.discount_curve_id)?;
        let surv = context.curves.get_hazard(&cds.protection.credit_curve_id)?;
        let pricer = CDSPricer::new();
        pricer.risky_annuity(cds, disc.as_ref(), surv.as_ref(), context.as_of)
    }
}
