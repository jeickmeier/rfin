//! CDS hazard-bump CS01 metric calculator.
//!
//! Computes PV sensitivity to a parallel additive bump in hazard rates of 1bp
//! across all pillar times, rebuilt via the curve bump API. Returns absolute
//! PV change per 1bp.

use crate::instruments::cds::CreditDefaultSwap;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::market_data::bumps::{BumpMode, BumpSpec, BumpUnits, Bumpable};
use finstack_core::Result;

/// Hazard CS01 calculator for CDS (parallel hazard bump)
pub struct HazardCs01Calculator;

impl MetricCalculator for HazardCs01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let cds: &CreditDefaultSwap = context.instrument_as()?;

        // Curves
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

        // Base PV
        let base = (cds.pv_protection_leg(disc, surv)? - cds.pv_premium_leg(disc, surv)?)?;

        // Build a +1bp hazard bump (additive in rate units)
        let spec = BumpSpec {
            mode: BumpMode::Additive,
            units: BumpUnits::RateBp,
            value: 1.0, // 1 bp
        };
        let bumped_surv = Bumpable::apply_bump(surv, spec).ok_or(finstack_core::Error::Internal)?;

        // PV with bumped hazard
        let bumped = (cds.pv_protection_leg(disc, &bumped_surv)?
            - cds.pv_premium_leg(disc, &bumped_surv)?)?;

        Ok((bumped.amount() - base.amount()).abs())
    }
}
