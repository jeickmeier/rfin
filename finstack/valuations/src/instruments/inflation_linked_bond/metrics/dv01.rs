//! Inflation-Linked Bond DV01 metric calculator.
//!
//! Provides DV01 calculation for Inflation-Linked Bond instruments using bump-and-reprice methodology.
//! Bumps the real rate discount curve by +1bp and measures the impact on bond value.
//!
//! Sign convention: DV01 = base_pv - bumped_pv (positive when real rates rise causes value to fall).

use crate::instruments::common::traits::Instrument;
use crate::instruments::InflationLinkedBond;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::market_data::context::BumpSpec;
use finstack_core::Result;
use hashbrown::HashMap;

/// DV01 calculator for Inflation-Linked Bond instruments using real rate discount curve bump-and-reprice.
pub struct InflationLinkedBondDv01Calculator;

impl MetricCalculator for InflationLinkedBondDv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let bond: &InflationLinkedBond = context.instrument_as()?;
        let as_of = context.as_of;

        if as_of >= bond.maturity {
            return Ok(0.0);
        }

        // Base PV from context
        let base_pv = context.base_value;

        // Parallel +1bp bump on real rate discount curve
        let mut bumps = HashMap::new();
        bumps.insert(bond.disc_id.clone(), BumpSpec::parallel_bp(1.0));
        let bumped_context = context.curves.bump(bumps)?;
        
        // Reprice with bumped curve
        let bumped_pv = bond.value(&bumped_context, as_of)?;

        // DV01 = base_pv - bumped_pv
        let dv01 = base_pv.checked_sub(bumped_pv)?;
        
        Ok(dv01.amount())
    }
}
