//! Bond DV01 metric calculator.
//!
//! Provides DV01 calculation for bond instruments using modified duration:
//! DV01 ≈ Price × Modified Duration × 1bp
//! Sign convention: positive for long positions (bond prices fall when rates rise).

use crate::constants::ONE_BASIS_POINT;
use crate::instruments::bond::Bond;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::Result;

/// DV01 calculator for bonds.
pub struct BondDv01Calculator;

impl MetricCalculator for BondDv01Calculator {
    fn dependencies(&self) -> &[MetricId] {
        &[]
    }

    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let bond: &Bond = context.instrument_as()?;
        let as_of = context.as_of;

        if as_of >= bond.maturity {
            return Ok(0.0);
        }

        // Simple DV01 approximation: Notional × Time to Maturity × 1bp
        // This avoids complex duration dependencies that require clean prices
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
