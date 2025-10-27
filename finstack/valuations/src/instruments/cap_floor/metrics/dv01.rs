//! Cap/Floor DV01 metric calculator.
//!
//! Provides DV01 calculation for Cap/Floor instruments using bump-and-reprice methodology.
//! Bumps the discount curve by +1bp and measures the impact on option value.
//!
//! Sign convention: DV01 = base_pv - bumped_pv (positive when rates rise causes value to fall).

use crate::instruments::common::traits::Instrument;
use crate::instruments::cap_floor::InterestRateOption;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::market_data::context::BumpSpec;
use finstack_core::Result;
use hashbrown::HashMap;

/// DV01 calculator for Cap/Floor instruments using discount curve bump-and-reprice.
pub struct CapFloorDv01Calculator;

impl MetricCalculator for CapFloorDv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &InterestRateOption = context.instrument_as()?;
        let as_of = context.as_of;

        if as_of >= option.end_date {
            return Ok(0.0);
        }

        // Base PV from context
        let base_pv = context.base_value;

        // Parallel +1bp bump on discount curve
        let mut bumps = HashMap::new();
        bumps.insert(option.disc_id.clone(), BumpSpec::parallel_bp(1.0));
        let bumped_context = context.curves.bump(bumps)?;
        
        // Reprice with bumped curve
        let bumped_pv = option.value(&bumped_context, as_of)?;

        // DV01 = base_pv - bumped_pv
        let dv01 = base_pv.checked_sub(bumped_pv)?;
        
        Ok(dv01.amount())
    }
}
