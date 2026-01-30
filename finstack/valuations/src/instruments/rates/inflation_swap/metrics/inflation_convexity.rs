//! Inflation convexity calculator for inflation swaps.
//!
//! Calculates the second derivative of the swap value with respect to parallel
//! inflation curve shifts. Inflation convexity measures how Inflation01 changes
//! as inflation rates move.
//!
//! Uses numerical differentiation with 1bp bumps to the inflation curve.
//!
//! # Mathematical Definition
//!
//! Convexity is the second derivative of PV with respect to inflation rate:
//! ```text
//! Convexity = d²PV / dπ² ≈ (PV_up + PV_down - 2×PV_base) / bump²
//! ```
//!
//! Note: Convexity is typically non-zero even for at-market (par) swaps where
//! PV = 0. This is because the curvature of the PV function exists regardless
//! of the current PV level.

use crate::instruments::common::traits::Instrument;
use crate::instruments::inflation_swap::InflationSwap;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::market_data::bumps::{BumpSpec, MarketBump};
use finstack_core::Result;

/// Standard inflation curve bump: 1bp (0.0001)
const INFLATION_BUMP_BP: f64 = 0.0001;

/// Calculates inflation convexity for inflation swaps.
///
/// Uses central finite differences for numerical stability:
/// `Convexity ≈ (PV(+bump) + PV(-bump) - 2×PV_base) / bump²`
///
/// Note: Returns non-zero convexity even for par swaps (where base PV = 0),
/// since convexity measures the curvature of the PV function, not its level.
pub struct InflationConvexityCalculator;

impl MetricCalculator for InflationConvexityCalculator {
    fn dependencies(&self) -> &[MetricId] {
        &[] // Independent metric
    }

    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let swap: &InflationSwap = context.instrument_as()?;
        let as_of = context.as_of;

        // Get base value
        let base_pv = context.base_value.amount();

        // Bump size: 1bp for numerical convexity
        let bump_bp = INFLATION_BUMP_BP;

        // Get the inflation index/curve ID
        let inflation_curve_id = &swap.inflation_index_id;

        // Create bumped curves (up)
        let bump_spec_up = BumpSpec::inflation_shift_pct(bump_bp * 100.0); // Convert bp to percent
        let curves_up = context.curves.bump([MarketBump::Curve {
            id: inflation_curve_id.clone(),
            spec: bump_spec_up,
        }])?;
        let pv_up = swap.value(&curves_up, as_of)?.amount();

        // Create bumped curves (down)
        let bump_spec_down = BumpSpec::inflation_shift_pct(-bump_bp * 100.0);
        let curves_down = context.curves.bump([MarketBump::Curve {
            id: inflation_curve_id.clone(),
            spec: bump_spec_down,
        }])?;
        let pv_down = swap.value(&curves_down, as_of)?.amount();

        // InflationConvexity = (PV_up + PV_down - 2×PV_base) / (bump²)
        // This gives the second derivative normalized per (basis point)²
        //
        // Note: This formula is valid even when base_pv = 0 (par swaps).
        // Convexity measures curvature, not absolute level.
        let inflation_convexity = (pv_up + pv_down - 2.0 * base_pv) / (bump_bp * bump_bp);

        Ok(inflation_convexity)
    }
}
