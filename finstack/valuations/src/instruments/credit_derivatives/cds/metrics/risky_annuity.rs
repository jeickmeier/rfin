//! CDS risky annuity metric calculator.
//!
//! Computes the risky annuity (premium leg PV per 1bp) using the CDS pricer.

use crate::constants::ONE_BASIS_POINT;
use crate::instruments::credit_derivatives::cds::pricer::CDSPricer;
use crate::instruments::credit_derivatives::cds::CreditDefaultSwap;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Risky annuity calculator for CDS
pub(crate) struct RiskyAnnuityCalculator;

impl MetricCalculator for RiskyAnnuityCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let cds: &CreditDefaultSwap = context.instrument_as()?;
        let disc = context
            .curves
            .get_discount(&cds.premium.discount_curve_id)?;
        let surv = context.curves.get_hazard(&cds.protection.credit_curve_id)?;
        let pricer = CDSPricer::new();
        if cds.uses_full_premium_par_spread_denominator() {
            return pricer
                .premium_leg_pv_per_bp(cds, disc.as_ref(), surv.as_ref(), context.as_of)
                .map(|pv_per_bp| pv_per_bp / ONE_BASIS_POINT);
        }
        pricer.risky_annuity(cds, disc.as_ref(), surv.as_ref(), context.as_of)
    }
}
