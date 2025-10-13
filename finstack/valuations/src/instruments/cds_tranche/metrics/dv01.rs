//! CDS Tranche DV01 metric calculator.
//!
//! Provides DV01 calculation for CDS Tranche instruments:
//! DV01 ≈ Notional × Time to Maturity × 1bp

use crate::constants::ONE_BASIS_POINT;
use crate::instruments::cds_tranche::CdsTranche;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// DV01 calculator for CDS Tranche instruments.
pub struct CdsTrancheDv01Calculator;

impl MetricCalculator for CdsTrancheDv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let tranche: &CdsTranche = context.instrument_as()?;
        let as_of = context.as_of;

        // Get maturity from tranche maturity field
        let maturity = tranche.maturity;
        if as_of >= maturity {
            return Ok(0.0);
        }

        // Simple DV01 approximation: Notional × Time to Maturity × 1bp
        let time_to_maturity = tranche
            .day_count
            .year_fraction(
                as_of,
                maturity,
                finstack_core::dates::DayCountCtx::default(),
            )?
            .max(0.0);

        let dv01 = tranche.notional.amount() * time_to_maturity * ONE_BASIS_POINT;

        Ok(dv01)
    }
}
