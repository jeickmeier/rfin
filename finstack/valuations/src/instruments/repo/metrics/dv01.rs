//! Repo DV01 metric.
//!
//! Computes the change in PV for a +1bp parallel bump to the discount curve
//! associated with the repo's discount id.

use crate::instruments::common::traits::Instrument;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::market_data::context::BumpSpec;
use finstack_core::prelude::*;
use finstack_core::F;
use hashbrown::HashMap;

/// Calculate DV01 for repo (interest rate sensitivity).
pub struct RepoDv01Calculator;

impl MetricCalculator for RepoDv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let repo = context.instrument_as::<crate::instruments::repo::Repo>()?;

        // Base PV
        let base_pv = repo.value(&context.curves, context.as_of)?;

        // Parallel +1bp bump on discount curve
        let disc_curve_id = repo.disc_id.clone();
        let mut bumps = HashMap::new();
        bumps.insert(disc_curve_id, BumpSpec::parallel_bp(1.0));
        let bumped_context = context.curves.bump(bumps)?;
        let bumped_pv = repo.value(&bumped_context, context.as_of)?;

        // DV01 = base_pv - bumped_pv
        let dv01 = base_pv.checked_sub(bumped_pv)?;
        Ok(dv01.amount())
    }
}
