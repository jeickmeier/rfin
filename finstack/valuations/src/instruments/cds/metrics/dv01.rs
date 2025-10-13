//! CDS DV01 metric calculator.
//!
//! Provides DV01 calculation for CDS instruments:
//! DV01 ≈ Notional × Time to Maturity × 1bp
//! Sign convention: positive for protection buyer (long protection).

use crate::constants::ONE_BASIS_POINT;
use crate::instruments::cds::CreditDefaultSwap;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// DV01 calculator for CDS instruments.
pub struct CdsDv01Calculator;

impl MetricCalculator for CdsDv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let cds: &CreditDefaultSwap = context.instrument_as()?;
        let as_of = context.as_of;

        // Get maturity from premium leg end date
        let maturity = cds.premium.end;
        if as_of >= maturity {
            return Ok(0.0);
        }

        // Simple DV01 approximation: Notional × Time to Maturity × 1bp
        let time_to_maturity = cds
            .premium
            .dc
            .year_fraction(
                as_of,
                maturity,
                finstack_core::dates::DayCountCtx::default(),
            )?
            .max(0.0);
        
        let dv01 = cds.notional.amount() * time_to_maturity * ONE_BASIS_POINT;
        
        Ok(dv01)
    }
}
