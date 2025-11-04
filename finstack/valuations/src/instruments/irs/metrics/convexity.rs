//! IRS convexity metric.
//!
//! Calculates the second derivative of the swap value with respect to parallel
//! interest rate shifts. Convexity measures how DV01 changes as rates move.
//!
//! Uses numerical differentiation with 1bp bumps to the discount curve.

use crate::instruments::common::traits::Instrument;
use crate::instruments::InterestRateSwap;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::market_data::bumps::BumpSpec;
use hashbrown::HashMap;

/// Calculates convexity for interest rate swaps.
pub struct ConvexityCalculator;

impl MetricCalculator for ConvexityCalculator {
    fn dependencies(&self) -> &[MetricId] {
        &[] // Independent metric
    }

    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let irs: &InterestRateSwap = context.instrument_as()?;
        let as_of = context.as_of;
        
        // Get base value
        let base_pv = context.base_value.amount();
        
        // Bump size: 1bp for numerical convexity
        let bump_bp = 1.0;
        
        // Get all discount curves used by this IRS
        let curve_ids = irs.required_discount_curves();
        
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
        let pv_up = irs.value(&curves_up, as_of)?.amount();
        
        // Create bumped curves (down)
        let bump_spec_down = BumpSpec::parallel_bp(-bump_bp);
        let mut bumps_down = HashMap::new();
        for curve_id in &curve_ids {
            bumps_down.insert(curve_id.clone(), bump_spec_down);
        }
        
        let curves_down = context.curves.bump(bumps_down)?;
        let pv_down = irs.value(&curves_down, as_of)?.amount();
        
        if base_pv == 0.0 {
            return Ok(0.0);
        }
        
        // Convert bump from bp to decimal
        let bump_decimal = bump_bp / 10_000.0;
        
        // Convexity = (PV_up + PV_down - 2×PV_base) / (bump²)
        // This gives the second derivative normalized per (basis point)²
        let convexity = (pv_up + pv_down - 2.0 * base_pv) / (bump_decimal * bump_decimal);
        
        Ok(convexity)
    }
}

