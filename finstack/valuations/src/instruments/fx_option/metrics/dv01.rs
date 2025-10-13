//! FX Option DV01 metric calculator.
//!
//! Provides DV01 calculation for FX Option instruments:
//! DV01 ≈ Notional × Time to Expiry × 1bp
//! Sign convention: positive for long positions.

use crate::constants::ONE_BASIS_POINT;
use crate::instruments::fx_option::FxOption;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// DV01 calculator for FX Option instruments.
pub struct FxOptionDv01Calculator;

impl MetricCalculator for FxOptionDv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let fx_option: &FxOption = context.instrument_as()?;
        let as_of = context.as_of;

        if as_of >= fx_option.expiry {
            return Ok(0.0);
        }

        // Simple DV01 approximation: Notional × Time to Expiry × 1bp
        let time_to_expiry = fx_option
            .day_count
            .year_fraction(
                as_of,
                fx_option.expiry,
                finstack_core::dates::DayCountCtx::default(),
            )?
            .max(0.0);
        
        let dv01 = fx_option.notional.amount() * time_to_expiry * ONE_BASIS_POINT;
        
        Ok(dv01)
    }
}
