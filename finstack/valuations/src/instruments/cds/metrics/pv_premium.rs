//! CDS premium leg PV metric calculator.
//!
//! Computes present value of the premium leg using discount and hazard curves
//! via the engine. The value is returned in currency units.

use crate::instruments::cds::CreditDefaultSwap;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Premium leg PV calculator
pub struct PremiumLegPvCalculator;

impl MetricCalculator for PremiumLegPvCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let cds: &CreditDefaultSwap = context.instrument_as()?;
        let disc = context.curves.get_discount_ref(&cds.premium.discount_curve_id)?;
        let surv = context.curves.get_hazard_ref(&cds.protection.credit_curve_id)?;
        let pv = cds.pv_premium_leg(disc, surv, context.as_of)?;
        Ok(pv.amount())
    }
}
