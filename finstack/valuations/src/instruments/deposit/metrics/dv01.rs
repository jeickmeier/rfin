//! Deposit DV01 metric calculator.
//!
//! Provides DV01 calculation for deposit instruments using bump-and-reprice methodology.
//! Computes the change in PV for a +1bp parallel bump to the discount curve.
//!
//! Sign convention: DV01 = base_pv - bumped_pv (positive for instruments that lose value when rates rise).

use crate::instruments::common::traits::Instrument;
use crate::instruments::deposit::Deposit;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::market_data::context::BumpSpec;
use finstack_core::Result;
use hashbrown::HashMap;

/// DV01 calculator for deposits using discount curve bump-and-reprice.
pub struct DepositDv01Calculator;

impl MetricCalculator for DepositDv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let deposit: &Deposit = context.instrument_as()?;
        let as_of = context.as_of;

        if as_of >= deposit.end {
            return Ok(0.0);
        }

        // Base PV from context
        let base_pv = context.base_value;

        // Parallel +1bp bump on discount curve
        let mut bumps = HashMap::new();
        bumps.insert(deposit.disc_id.clone(), BumpSpec::parallel_bp(1.0));
        let bumped_context = context.curves.bump(bumps)?;
        
        // Reprice with bumped curve
        let bumped_pv = deposit.value(&bumped_context, as_of)?;

        // DV01 = base_pv - bumped_pv (positive when rates rise causes value to fall)
        let dv01 = base_pv.checked_sub(bumped_pv)?;
        
        Ok(dv01.amount())
    }
}
