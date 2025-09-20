//! CDS risky PV01 metric calculator.
//!
//! Computes the change in present value for a one basis point change in
//! the premium spread, using the pricing engine's risky annuity.

use crate::instruments::cds::CreditDefaultSwap;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::{Result, F};

/// Risky PV01 calculator for CDS
pub struct RiskyPv01Calculator;

impl MetricCalculator for RiskyPv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let cds: &CreditDefaultSwap = context.instrument_as()?;
        let disc = context
            .curves
            .get_ref::<finstack_core::market_data::term_structures::discount_curve::DiscountCurve>(
            cds.premium.disc_id,
        )?;
        let surv = context
            .curves
            .get_ref::<finstack_core::market_data::term_structures::hazard_curve::HazardCurve>(
            cds.protection.credit_id,
        )?;
        cds.risky_pv01(disc, surv)
    }
}
