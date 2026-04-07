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
//!
//! # Units
//!
//! The bump is applied as a percentage shift to the inflation curve (0.01% = 1bp).
//! The convexity result is normalized per (basis point)² = per (0.01%)².

use crate::instruments::common_impl::traits::Instrument;
use crate::instruments::rates::inflation_swap::InflationSwap;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::market_data::bumps::{BumpSpec, MarketBump};
use finstack_core::Result;

/// Standard inflation curve bump: 1bp = 0.01% (as percentage for BumpSpec)
const INFLATION_BUMP_PCT: f64 = 0.01;

/// Calculates inflation convexity for inflation swaps.
///
/// Uses central finite differences for numerical stability:
/// `Convexity ≈ (PV(+bump) + PV(-bump) - 2×PV_base) / bump²`
///
/// Note: Returns non-zero convexity even for par swaps (where base PV = 0),
/// since convexity measures the curvature of the PV function, not its level.
pub(crate) struct InflationConvexityCalculator;

impl MetricCalculator for InflationConvexityCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let swap: &InflationSwap = context.instrument_as()?;
        let as_of = context.as_of;

        // Get base value
        let base_pv = context.base_value.amount();

        // Get the inflation index/curve ID
        let inflation_curve_id = &swap.inflation_index_id;

        // Create bumped curves (up by 1bp = 0.01%)
        let bump_spec_up = BumpSpec::inflation_shift_pct(INFLATION_BUMP_PCT);
        let curves_up = context.curves.bump([MarketBump::Curve {
            id: inflation_curve_id.clone(),
            spec: bump_spec_up,
        }])?;
        let pv_up = swap.value(&curves_up, as_of)?.amount();

        // Create bumped curves (down by 1bp = 0.01%)
        let bump_spec_down = BumpSpec::inflation_shift_pct(-INFLATION_BUMP_PCT);
        let curves_down = context.curves.bump([MarketBump::Curve {
            id: inflation_curve_id.clone(),
            spec: bump_spec_down,
        }])?;
        let pv_down = swap.value(&curves_down, as_of)?.amount();

        // InflationConvexity = (PV_up + PV_down - 2×PV_base) / (bump²)
        //
        // The bump is 1bp = 0.01% = 0.0001 in decimal form.
        // For convexity per bp², we divide by (0.0001)² = 1e-8.
        //
        // Note: This formula is valid even when base_pv = 0 (par swaps).
        // Convexity measures curvature, not absolute level.
        const BUMP_DECIMAL: f64 = 0.0001; // 1bp in decimal
        let inflation_convexity = (pv_up + pv_down - 2.0 * base_pv) / (BUMP_DECIMAL * BUMP_DECIMAL);

        Ok(inflation_convexity)
    }
}
