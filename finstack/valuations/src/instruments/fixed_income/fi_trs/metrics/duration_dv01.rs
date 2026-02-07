//! Duration-based DV01 calculator for fixed income index TRS.

use crate::instruments::common_impl::parameters::trs_common::TrsSide;
use crate::instruments::fixed_income::fi_trs::FIIndexTotalReturnSwap;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::market_data::scalars::MarketScalar;
use finstack_core::Result;

/// Calculates duration-based DV01 for fixed income index TRS.
///
/// Measures the dollar value change for a 1 basis point yield shift:
///
/// ```text
/// DurationDv01 = Notional × Duration × 0.0001
/// ```
///
/// This is a yield sensitivity metric (not an index-level delta). For equity TRS,
/// use `IndexDelta` which measures `dV/dS` per unit of index level change.
///
/// # Errors
///
/// Returns an error if `duration_id` is configured but missing from market data.
/// When `duration_id` is `None`, defaults to 5.0 years (broad market assumption).
pub struct DurationDv01Calculator;

impl MetricCalculator for DurationDv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let trs: &FIIndexTotalReturnSwap = context.instrument_as()?;

        // Get duration from market data.
        // If duration_id is configured, the data MUST be present (error on missing).
        // If duration_id is None, default to 5.0 years (broad index assumption).
        let duration = match &trs.underlying.duration_id {
            Some(id) => {
                let scalar = context.curves.price(id.as_str()).map_err(|_| {
                    finstack_core::Error::Validation(format!(
                        "Index duration data '{}' is configured but not found in market context. \
                         Provide the duration scalar or remove duration_id to use 5.0Y default.",
                        id
                    ))
                })?;
                match scalar {
                    MarketScalar::Unitless(v) => *v,
                    MarketScalar::Price(_) => {
                        return Err(finstack_core::Error::Validation(format!(
                            "Market scalar '{}' for index duration has type Price, but duration \
                             is a unitless quantity. Use MarketScalar::Unitless instead.",
                            id
                        )));
                    }
                }
            }
            None => 5.0, // No duration configured — use broad market default
        };

        // DV01 = Notional × Duration × 1bp
        let dv01 = trs.notional.amount() * duration * 0.0001;

        Ok(match trs.side {
            TrsSide::ReceiveTotalReturn => dv01,
            TrsSide::PayTotalReturn => -dv01,
        })
    }
}
