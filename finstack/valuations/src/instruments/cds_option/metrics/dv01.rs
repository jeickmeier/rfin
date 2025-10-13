//! CDS Option DV01 metric calculator.
//!
//! Provides DV01 calculation for CDS Option instruments:
//! DV01 ≈ Notional × Time to Expiry × 1bp

use crate::constants::ONE_BASIS_POINT;
use crate::instruments::cds_option::CdsOption;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// DV01 calculator for CDS Option instruments.
pub struct CdsOptionDv01Calculator;

impl MetricCalculator for CdsOptionDv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let cds_option: &CdsOption = context.instrument_as()?;
        let as_of = context.as_of;

        if as_of >= cds_option.expiry {
            return Ok(0.0);
        }

        // Simple DV01 approximation: Notional × Time to Expiry × 1bp
        let time_to_expiry = cds_option
            .day_count
            .year_fraction(
                as_of,
                cds_option.expiry,
                finstack_core::dates::DayCountCtx::default(),
            )?
            .max(0.0);
        
        let dv01 = cds_option.notional.amount() * time_to_expiry * ONE_BASIS_POINT;
        
        Ok(dv01)
    }
}
