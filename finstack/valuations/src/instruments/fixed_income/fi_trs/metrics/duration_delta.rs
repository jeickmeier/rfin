//! Duration-based delta calculator for fixed income index TRS.

use crate::instruments::common::parameters::trs_common::TrsSide;
use crate::instruments::fixed_income::fi_trs::FIIndexTotalReturnSwap;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::market_data::scalars::MarketScalar;
use finstack_core::Result;

/// Calculates delta to the underlying index level using duration as a proxy.
///
/// For fixed income index TRS, delta is calculated as:
/// ```text
/// Delta = Notional × Duration × 0.0001
/// ```
///
/// This represents the sensitivity to a 1bp change in yields, assuming
/// the index moves proportionally with duration.
pub struct DurationDeltaCalculator;

impl MetricCalculator for DurationDeltaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let trs: &FIIndexTotalReturnSwap = context.instrument_as()?;

        // Get duration from market data, default to 5.0 years for broad indices
        let duration = trs
            .underlying
            .duration_id
            .as_ref()
            .and_then(|id| {
                context.curves.price(id.as_str()).ok().map(|s| match s {
                    MarketScalar::Unitless(v) => *v,
                    MarketScalar::Price(p) => p.amount(),
                })
            })
            .unwrap_or(5.0);

        // Delta = Notional × Duration × 1bp
        let delta = trs.notional.amount() * duration * 0.0001;

        Ok(match trs.side {
            TrsSide::ReceiveTotalReturn => delta,
            TrsSide::PayTotalReturn => -delta,
        })
    }
}
