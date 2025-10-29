//! FX Swap DV01 metric calculator.
//!
//! Provides DV01 calculation for FX swap instruments using bump-and-reprice methodology.
//! Computes the change in PV for a +1bp parallel bump to the domestic discount curve.
//!
//! Sign convention: DV01 = base_pv - bumped_pv (positive when rates rise causes value to fall).

use crate::instruments::common::traits::Instrument;
use crate::instruments::fx_swap::FxSwap;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::market_data::context::BumpSpec;
use finstack_core::Result;
use hashbrown::HashMap;

/// DV01 calculator for FX swaps using domestic discount curve bump-and-reprice.
pub struct FxSwapDv01Calculator;

impl MetricCalculator for FxSwapDv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let fx_swap: &FxSwap = context.instrument_as()?;
        let as_of = context.as_of;

        if as_of >= fx_swap.far_date {
            return Ok(0.0);
        }

        // Base PV from context
        let base_pv = context.base_value;

        // Parallel +1bp bump on domestic discount curve
        let mut bumps = HashMap::new();
        bumps.insert(fx_swap.domestic_disc_id.clone(), BumpSpec::parallel_bp(1.0));
        let bumped_context = context.curves.bump(bumps)?;

        // Reprice with bumped curve
        let bumped_pv = fx_swap.value(&bumped_context, as_of)?;

        // DV01 = base_pv - bumped_pv
        let dv01 = base_pv.checked_sub(bumped_pv)?;

        Ok(dv01.amount())
    }
}
