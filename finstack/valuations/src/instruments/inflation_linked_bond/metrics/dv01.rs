//! Inflation-Linked Bond DV01 metric calculator.
//!
//! Provides DV01 calculation for Inflation-Linked Bond instruments:
//! DV01 ≈ Notional × Time to Maturity × 1bp

use crate::constants::ONE_BASIS_POINT;
use crate::instruments::InflationLinkedBond;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// DV01 calculator for Inflation-Linked Bond instruments.
pub struct InflationLinkedBondDv01Calculator;

impl MetricCalculator for InflationLinkedBondDv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let bond: &InflationLinkedBond = context.instrument_as()?;
        let as_of = context.as_of;

        if as_of >= bond.maturity {
            return Ok(0.0);
        }

        // Simple DV01 approximation: Notional × Time to Maturity × 1bp
        let time_to_maturity = bond
            .dc
            .year_fraction(
                as_of,
                bond.maturity,
                finstack_core::dates::DayCountCtx::default(),
            )?
            .max(0.0);
        
        let dv01 = bond.notional.amount() * time_to_maturity * ONE_BASIS_POINT;
        
        Ok(dv01)
    }
}
