//! CDS protection leg PV metric calculator.
//!
//! Computes present value of the protection leg using the configured curves
//! and the engine's implementation. The measure is returned in currency units.

use crate::instruments::cds::CreditDefaultSwap;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Protection leg PV calculator
pub struct ProtectionLegPvCalculator;

impl MetricCalculator for ProtectionLegPvCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let cds: &CreditDefaultSwap = context.instrument_as()?;
        let disc = context
            .curves
            .get_discount_ref(&cds.premium.disc_id)?;
        let surv = context
            .curves
            .get_hazard_ref(&cds.protection.credit_id)?;
        let pv = cds.pv_protection_leg(disc, surv)?;
        Ok(pv.amount())
    }
}
