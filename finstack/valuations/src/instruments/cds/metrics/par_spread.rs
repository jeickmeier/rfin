//! CDS par spread metric calculator.
//!
//! Computes the fixed spread in basis points that sets the CDS NPV to zero,
//! using the pricing engine's par-spread calculation. This is independent
//! of the instrument's current quoted spread.

use crate::instruments::cds::CreditDefaultSwap;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::{Result, F};

/// Par spread calculator for CDS
pub struct ParSpreadCalculator;

impl MetricCalculator for ParSpreadCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let cds: &CreditDefaultSwap = context.instrument_as()?;
        let disc = context
            .curves
            .get_ref::<finstack_core::market_data::term_structures::discount_curve::DiscountCurve>(
            cds.premium.disc_id.clone(),
        )?;
        let surv = context
            .curves
            .get_ref::<finstack_core::market_data::term_structures::hazard_curve::HazardCurve>(
            cds.protection.credit_id.clone(),
        )?;
        cds.par_spread(disc, surv)
    }
}
