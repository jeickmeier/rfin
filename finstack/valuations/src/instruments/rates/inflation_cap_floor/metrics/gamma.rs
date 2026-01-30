//! Gamma calculator for inflation cap/floor options.
//!
//! Computes gamma (second derivative of PV with respect to forward inflation rate)
//! using central finite differences:
//!
//! ```text
//! Gamma = (PV_up - 2×PV_base + PV_down) / (bump_size)²
//! ```
//!
//! Gamma measures the convexity of the option's value with respect to inflation,
//! which is important for hedging and risk management.
//!
//! # Units
//!
//! - Bump: 1bp = 0.01% applied to inflation curve via `BumpSpec::inflation_shift_pct`
//! - Result: Gamma per (basis point)² = per (0.01%)²

use crate::instruments::inflation_cap_floor::InflationCapFloor;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::market_data::bumps::{BumpSpec, MarketBump};
use finstack_core::Result;

/// Inflation curve bump size: 1bp = 0.01% (as percentage for BumpSpec).
const GAMMA_BUMP_PCT: f64 = 0.01;

/// Bump size in decimal terms for scaling the result: 1bp = 0.0001.
const GAMMA_BUMP_DECIMAL: f64 = 0.0001;

/// Gamma calculator for inflation cap/floor options.
///
/// Computes the second derivative of PV with respect to the forward inflation rate.
/// Uses central finite differences for accurate second derivative estimation.
pub struct GammaCalculator;

impl MetricCalculator for GammaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &InflationCapFloor = context.instrument_as()?;
        let as_of = context.as_of;
        let base_pv = context.base_value.amount();

        // Expired options have zero gamma
        if as_of >= option.end_date {
            return Ok(0.0);
        }

        // Bump up by 1bp = 0.01%
        let bump_spec_up = BumpSpec::inflation_shift_pct(GAMMA_BUMP_PCT);
        let curves_up = context.curves.as_ref().bump([MarketBump::Curve {
            id: option.inflation_index_id.clone(),
            spec: bump_spec_up,
        }])?;
        let pv_up = option.npv(&curves_up, as_of)?.amount();

        // Bump down by 1bp = 0.01%
        let bump_spec_down = BumpSpec::inflation_shift_pct(-GAMMA_BUMP_PCT);
        let curves_down = context.curves.as_ref().bump([MarketBump::Curve {
            id: option.inflation_index_id.clone(),
            spec: bump_spec_down,
        }])?;
        let pv_down = option.npv(&curves_down, as_of)?.amount();

        // Second derivative via central difference:
        // Gamma = (PV_up - 2*PV_base + PV_down) / h²
        //
        // Where h = 1bp = 0.0001 in decimal form.
        // This gives gamma per (basis point)².
        let h_squared = GAMMA_BUMP_DECIMAL * GAMMA_BUMP_DECIMAL;
        Ok((pv_up - 2.0 * base_pv + pv_down) / h_squared)
    }
}
