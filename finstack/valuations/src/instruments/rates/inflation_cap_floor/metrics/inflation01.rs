//! Inflation01 calculator for inflation cap/floor options.
//!
//! Computes inflation sensitivity using central finite differences on the inflation curve.
//!
//! # Methodology
//!
//! Inflation01 measures the change in PV for a 1 basis point (0.01%) parallel shift
//! in the inflation curve. Uses central differences for O(h²) accuracy:
//!
//! ```text
//! Inflation01 = (PV_up - PV_down) / (2 × bump_size)
//! ```

use crate::instruments::common_impl::traits::Instrument;
use crate::instruments::rates::inflation_cap_floor::InflationCapFloor;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::market_data::bumps::{BumpSpec, MarketBump};
use finstack_core::Result;

/// Inflation curve bump size: 1bp = 0.01% for `BumpSpec::inflation_shift_pct`.
/// The BumpSpec expects percentage terms, so 0.01 means 0.01% = 1bp.
const INFLATION_BUMP_PCT: f64 = 0.01;

/// Scaling factor to convert from percentage bump to decimal for final result.
/// Since we bump by 0.01% (1bp), the denominator should normalize to per-1bp sensitivity.
const INFLATION_BUMP_DECIMAL: f64 = 0.0001;

/// Inflation01 calculator for inflation cap/floor options.
///
/// Computes the present value sensitivity to a 1bp parallel shift in inflation expectations.
pub(crate) struct Inflation01Calculator;

impl MetricCalculator for Inflation01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &InflationCapFloor = context.instrument_as()?;
        let as_of = context.as_of;

        // Bump up by 1bp (0.01%)
        let bump_spec_up = BumpSpec::inflation_shift_pct(INFLATION_BUMP_PCT);
        let curves_up = context.curves.as_ref().bump([MarketBump::Curve {
            id: option.inflation_index_id.clone(),
            spec: bump_spec_up,
        }])?;
        let pv_up = option.value(&curves_up, as_of)?.amount();

        // Bump down by 1bp (-0.01%)
        let bump_spec_down = BumpSpec::inflation_shift_pct(-INFLATION_BUMP_PCT);
        let curves_down = context.curves.as_ref().bump([MarketBump::Curve {
            id: option.inflation_index_id.clone(),
            spec: bump_spec_down,
        }])?;
        let pv_down = option.value(&curves_down, as_of)?.amount();

        // Central difference: (PV_up - PV_down) / (2 × bump_size)
        Ok((pv_up - pv_down) / (2.0 * INFLATION_BUMP_DECIMAL))
    }
}
