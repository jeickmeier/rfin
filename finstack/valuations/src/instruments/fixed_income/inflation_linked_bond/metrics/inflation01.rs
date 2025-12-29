//! Inflation01 calculator for inflation-linked bonds.
//!
//! Computes inflation sensitivity using finite differences.
//! Inflation01 measures the change in PV for a 1bp (0.0001) shift in the inflation curve.
//!
//! # Formula
//! ```text
//! Inflation01 = (PV(inflation_curve + 1bp) - PV(inflation_curve - 1bp)) / (2 * bump_size)
//! ```
//! Where bump_size is 1bp (0.0001).
//!
//! # Note
//! For bonds backed by inflation indices, this bumps the underlying inflation curve
//! (which drives projected CPI). For index-based sources, we bump the curve that's
//! implicitly constructed from the index.

use crate::instruments::common::traits::Instrument;
use crate::instruments::inflation_linked_bond::InflationLinkedBond;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::HashMap;
use finstack_core::market_data::bumps::BumpSpec;
use finstack_core::Result;

/// Standard inflation curve bump: 1bp (0.0001)
const INFLATION_BUMP_BP: f64 = 0.0001;

/// Inflation01 calculator for inflation-linked bonds.
pub struct Inflation01Calculator;

impl MetricCalculator for Inflation01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let bond: &InflationLinkedBond = context.instrument_as()?;
        let as_of = context.as_of;
        let _base_pv = context.base_value.amount();

        // Check if we have an inflation curve (preferred) or index
        let inflation_curve_id = &bond.inflation_index_id;

        // Use MarketContext::bump() API to bump the inflation curve
        // Bump by 1bp using parallel shift
        let bump_spec = BumpSpec::inflation_shift_pct(INFLATION_BUMP_BP * 100.0); // Convert bp to percent

        let mut bumps = HashMap::default();
        bumps.insert(inflation_curve_id.clone(), bump_spec);

        let curves_up = context.curves.as_ref().bump(bumps.clone())?;
        let pv_up = bond.value(&curves_up, as_of)?.amount();

        // Bump down
        let bump_spec_down = BumpSpec::inflation_shift_pct(-INFLATION_BUMP_BP * 100.0);
        let mut bumps_down = HashMap::default();
        bumps_down.insert(inflation_curve_id.clone(), bump_spec_down);

        let curves_down = context.curves.as_ref().bump(bumps_down)?;
        let pv_down = bond.value(&curves_down, as_of)?.amount();

        // Inflation01 = (PV_up - PV_down) / (2 * bump_size)
        // bump_size is in percent, so we need to normalize to 1bp
        // BumpSpec::inflation_shift_pct(0.01) = 1bp = 0.0001
        // So we divide by 0.0002 (2 * 0.0001) to get per 1bp
        let inflation01 = (pv_up - pv_down) / (2.0 * INFLATION_BUMP_BP);

        Ok(inflation01)
    }
}
