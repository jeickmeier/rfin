//! CDS Index DV01 metric calculator.
//!
//! Provides DV01 calculation for CDS Index instruments:
//! DV01 ≈ Notional × Time to Maturity × 1bp
//! Sign convention: positive for protection buyer (long protection).

use crate::constants::ONE_BASIS_POINT;
use crate::instruments::cds_index::CDSIndex;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// DV01 calculator for CDS Index instruments.
pub struct CdsIndexDv01Calculator;

impl MetricCalculator for CdsIndexDv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let cds_index: &CDSIndex = context.instrument_as()?;
        let as_of = context.as_of;

        // Get maturity from premium leg end date
        let maturity = cds_index.premium.end;
        if as_of >= maturity {
            return Ok(0.0);
        }

        // Simple DV01 approximation: Notional × Time to Maturity × 1bp
        let time_to_maturity = cds_index
            .premium
            .dc
            .year_fraction(
                as_of,
                maturity,
                finstack_core::dates::DayCountCtx::default(),
            )?
            .max(0.0);

        let dv01 = cds_index.notional.amount() * time_to_maturity * ONE_BASIS_POINT;

        Ok(dv01)
    }
}
