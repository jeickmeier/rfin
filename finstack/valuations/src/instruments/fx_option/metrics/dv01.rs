//! FX Option DV01 metric calculator.
//!
//! Provides DV01 calculation for FX Option instruments using bump-and-reprice methodology.
//! Bumps both domestic and foreign discount curves by +1bp and measures the combined impact.
//!
//! Sign convention: DV01 = base_pv - bumped_pv (positive when rates rise causes value to fall).

use crate::instruments::fx_option::FxOption;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::market_data::context::BumpSpec;
use finstack_core::Result;
use hashbrown::HashMap;

/// DV01 calculator for FX Option instruments using dual-curve bump-and-reprice.
pub struct FxOptionDv01Calculator;

impl MetricCalculator for FxOptionDv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let fx_option: &FxOption = context.instrument_as()?;
        let as_of = context.as_of;

        if as_of >= fx_option.expiry {
            return Ok(0.0);
        }

        // Base PV from context
        let base_pv = context.base_value;

        // Parallel +1bp bump on both domestic and foreign discount curves
        let mut bumps = HashMap::new();
        bumps.insert(
            fx_option.domestic_disc_id.clone(),
            BumpSpec::parallel_bp(1.0),
        );
        bumps.insert(
            fx_option.foreign_disc_id.clone(),
            BumpSpec::parallel_bp(1.0),
        );
        let bumped_context = context.curves.bump(bumps)?;

        // Reprice with bumped curves
        let bumped_pv = fx_option.value(&bumped_context, as_of)?;

        // DV01 = base_pv - bumped_pv
        let dv01 = base_pv.checked_sub(bumped_pv)?;

        Ok(dv01.amount())
    }
}
