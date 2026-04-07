//! CDS risky PV01 metric calculator.
//!
//! Computes the change in present value for a one basis point change in
//! the premium spread, using the pricing engine's risky annuity.

use crate::instruments::credit_derivatives::cds::pricer::CDSPricer;
use crate::instruments::credit_derivatives::cds::CreditDefaultSwap;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Risky PV01 calculator for CDS
pub(crate) struct RiskyPv01Calculator;

impl MetricCalculator for RiskyPv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let cds: &CreditDefaultSwap = context.instrument_as()?;
        let disc = context
            .curves
            .get_discount(&cds.premium.discount_curve_id)?;
        let surv = context.curves.get_hazard(&cds.protection.credit_curve_id)?;
        let pricer = CDSPricer::new();
        pricer.risky_pv01(cds, disc.as_ref(), surv.as_ref(), context.as_of)
    }
}
