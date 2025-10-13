//! Swaption DV01 metric calculator.
//!
//! Provides DV01 calculation for Swaption instruments:
//! DV01 ≈ Notional × Time to Expiry × 1bp

use crate::constants::ONE_BASIS_POINT;
use crate::instruments::Swaption;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// DV01 calculator for Swaption instruments.
pub struct SwaptionDv01Calculator;

impl MetricCalculator for SwaptionDv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let swaption: &Swaption = context.instrument_as()?;
        let as_of = context.as_of;

        if as_of >= swaption.expiry {
            return Ok(0.0);
        }

        // Simple DV01 approximation: Notional × Time to Expiry × 1bp
        let time_to_expiry = swaption
            .day_count
            .year_fraction(
                as_of,
                swaption.expiry,
                finstack_core::dates::DayCountCtx::default(),
            )?
            .max(0.0);
        
        let dv01 = swaption.notional.amount() * time_to_expiry * ONE_BASIS_POINT;
        
        Ok(dv01)
    }
}
