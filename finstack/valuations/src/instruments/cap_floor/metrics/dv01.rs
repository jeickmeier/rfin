//! Cap/Floor DV01 metric calculator.
//!
//! Provides DV01 calculation for Cap/Floor instruments:
//! DV01 ≈ Notional × Time to Expiry × 1bp

use crate::constants::ONE_BASIS_POINT;
use crate::instruments::cap_floor::InterestRateOption;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// DV01 calculator for Cap/Floor instruments.
pub struct CapFloorDv01Calculator;

impl MetricCalculator for CapFloorDv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &InterestRateOption = context.instrument_as()?;
        let as_of = context.as_of;

        if as_of >= option.end_date {
            return Ok(0.0);
        }

        // Simple DV01 approximation: Notional × Time to End × 1bp
        let time_to_end = option
            .day_count
            .year_fraction(
                as_of,
                option.end_date,
                finstack_core::dates::DayCountCtx::default(),
            )?
            .max(0.0);
        
        let dv01 = option.notional.amount() * time_to_end * ONE_BASIS_POINT;
        
        Ok(dv01)
    }
}
