//! CDS CS-Gamma metric calculator.
//!
//! Calculates the second derivative of the CDS value with respect to parallel
//! credit spread shifts. CS-Gamma measures how CS01 changes as spreads move.
//!
//! Uses numerical differentiation with 1bp bumps to the hazard curve.

use crate::instruments::cds::CreditDefaultSwap;
use crate::instruments::common::traits::Instrument;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::market_data::bumps::BumpSpec;
use hashbrown::HashMap;

/// Calculates CS-Gamma for credit default swaps.
pub struct CsGammaCalculator;

impl MetricCalculator for CsGammaCalculator {
    fn dependencies(&self) -> &[MetricId] {
        &[] // Independent metric
    }

    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let cds: &CreditDefaultSwap = context.instrument_as()?;
        let as_of = context.as_of;

        // Get base value
        let base_pv = context.base_value.amount();

        // Bump size: 1bp for numerical convexity
        let bump_bp = 1.0;

        // Get all hazard curves used by this CDS
        let curve_ids = cds.required_hazard_curves();

        if curve_ids.is_empty() {
            return Ok(0.0);
        }

        // Create bumped curves (up)
        let bump_spec_up = BumpSpec::parallel_bp(bump_bp);
        let mut bumps_up = HashMap::new();
        for curve_id in &curve_ids {
            bumps_up.insert(curve_id.clone(), bump_spec_up);
        }

        let curves_up = context.curves.bump(bumps_up)?;
        let pv_up = cds.value(&curves_up, as_of)?.amount();

        // Create bumped curves (down)
        let bump_spec_down = BumpSpec::parallel_bp(-bump_bp);
        let mut bumps_down = HashMap::new();
        for curve_id in &curve_ids {
            bumps_down.insert(curve_id.clone(), bump_spec_down);
        }

        let curves_down = context.curves.bump(bumps_down)?;
        let pv_down = cds.value(&curves_down, as_of)?.amount();

        if base_pv == 0.0 {
            return Ok(0.0);
        }

        // Convert bump from bp to decimal
        let bump_decimal = bump_bp / 10_000.0;

        // CS-Gamma = (PV_up + PV_down - 2×PV_base) / (bump²)
        // This gives the second derivative normalized per (basis point)²
        let cs_gamma = (pv_up + pv_down - 2.0 * base_pv) / (bump_decimal * bump_decimal);

        Ok(cs_gamma)
    }
}
