//! CDS par spread metric calculator.
//!
//! Computes the fixed spread in basis points that sets the CDS NPV to zero,
//! using the pricing engine's par-spread calculation. This is independent
//! of the instrument's current quoted spread.

use crate::instruments::cds::CreditDefaultSwap;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Par spread calculator for CDS
pub struct ParSpreadCalculator;

impl MetricCalculator for ParSpreadCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let cds: &CreditDefaultSwap = context.instrument_as()?;
        let disc = context.curves.get_discount_ref(&cds.premium.discount_curve_id)?;
        let surv = context.curves.get_hazard_ref(&cds.protection.credit_curve_id)?;
        cds.par_spread(disc, surv, context.as_of)
    }
}
