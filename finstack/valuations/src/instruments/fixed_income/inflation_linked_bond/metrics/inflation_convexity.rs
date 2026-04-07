//! Inflation convexity calculator for inflation-linked bonds.
//!
//! Calculates the second derivative of the bond value with respect to parallel
//! inflation curve shifts. Inflation convexity measures how Inflation01 changes
//! as inflation rates move.
//!
//! Uses numerical differentiation with 1bp bumps to the inflation curve.

use crate::instruments::common_impl::traits::Instrument;
use crate::instruments::fixed_income::inflation_linked_bond::InflationLinkedBond;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::market_data::bumps::{BumpSpec, MarketBump};
use finstack_core::Result;

/// Standard inflation curve bump: 1bp (0.0001)
const INFLATION_BUMP_BP: f64 = 0.0001;

/// Calculates inflation convexity for inflation-linked bonds.
pub(crate) struct InflationConvexityCalculator;

impl MetricCalculator for InflationConvexityCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let bond: &InflationLinkedBond = context.instrument_as()?;
        let as_of = context.as_of;

        // Get base value
        let base_pv = context.base_value.amount();

        // Bump size: 1bp for numerical convexity
        let bump_bp = INFLATION_BUMP_BP;

        // Get the inflation index/curve ID
        let inflation_curve_id = &bond.inflation_index_id;

        // Create bumped curves (up)
        let bump_spec_up = BumpSpec::inflation_shift_pct(bump_bp * 100.0); // Convert bp to percent
        let curves_up = context.curves.bump([MarketBump::Curve {
            id: inflation_curve_id.clone(),
            spec: bump_spec_up,
        }])?;
        let pv_up = bond.value(&curves_up, as_of)?.amount();

        // Create bumped curves (down)
        let bump_spec_down = BumpSpec::inflation_shift_pct(-bump_bp * 100.0);
        let curves_down = context.curves.bump([MarketBump::Curve {
            id: inflation_curve_id.clone(),
            spec: bump_spec_down,
        }])?;
        let pv_down = bond.value(&curves_down, as_of)?.amount();

        if base_pv == 0.0 {
            return Ok(0.0);
        }

        // InflationConvexity = (PV_up + PV_down - 2×PV_base) / (bump²)
        // This gives the second derivative normalized per (basis point)²
        let inflation_convexity = (pv_up + pv_down - 2.0 * base_pv) / (bump_bp * bump_bp);

        Ok(inflation_convexity)
    }
}
