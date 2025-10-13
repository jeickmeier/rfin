//! Inflation Swap DV01 metric calculator.
//!
//! Provides DV01 calculation for Inflation Swap instruments:
//! DV01 ≈ Notional × Time to Maturity × 1bp

use crate::constants::ONE_BASIS_POINT;
use crate::instruments::InflationSwap;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// DV01 calculator for Inflation Swap instruments.
pub struct InflationSwapDv01Calculator;

impl MetricCalculator for InflationSwapDv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let swap: &InflationSwap = context.instrument_as()?;
        let as_of = context.as_of;

        if as_of >= swap.maturity {
            return Ok(0.0);
        }

        // Simple DV01 approximation: Notional × Time to Maturity × 1bp
        let time_to_maturity = swap
            .dc
            .year_fraction(
                as_of,
                swap.maturity,
                finstack_core::dates::DayCountCtx::default(),
            )?
            .max(0.0);

        let dv01 = swap.notional.amount() * time_to_maturity * ONE_BASIS_POINT;

        Ok(dv01)
    }
}
