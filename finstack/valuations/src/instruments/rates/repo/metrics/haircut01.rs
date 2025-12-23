//! Haircut01 calculator for Repo.
//!
//! Computes Haircut01 (haircut sensitivity) using finite differences.
//! Haircut01 measures the change in PV for a 1bp (0.0001 = 0.01%) change in haircut.
//!
//! # Formula
//! ```text
//! Haircut01 = (PV(haircut + 1bp) - PV(haircut - 1bp)) / (2 * bump_size)
//! ```
//! Where bump_size is 1bp (0.0001).
//!
//! # Note
//! Haircut affects the required collateral value, which impacts margin requirements
//! but doesn't directly affect PV in a simple repo model. However, it may affect
//! valuation in more complex models with margin calls. This metric measures the
//! sensitivity by repricing with bumped haircut values.

use crate::instruments::common::traits::Instrument;
use crate::instruments::repo::Repo;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Standard haircut bump: 1bp (0.0001 = 0.01%)
const HAIRCUT_BUMP: f64 = 0.0001;

/// Haircut01 calculator for Repo.
pub struct Haircut01Calculator;

impl MetricCalculator for Haircut01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let repo: &Repo = context.instrument_as()?;
        let as_of = context.as_of;

        // Bump haircut up
        let mut repo_up = repo.clone();
        repo_up.haircut = (repo.haircut + HAIRCUT_BUMP).max(0.0);
        let pv_up = repo_up.value(context.curves.as_ref(), as_of)?.amount();

        // Bump haircut down
        let mut repo_down = repo.clone();
        repo_down.haircut = (repo.haircut - HAIRCUT_BUMP).max(0.0);
        let pv_down = repo_down.value(context.curves.as_ref(), as_of)?.amount();

        // Haircut01 = (PV_up - PV_down) / (2 * bump_size)
        let haircut01 = (pv_up - pv_down) / (2.0 * HAIRCUT_BUMP);

        Ok(haircut01)
    }
}
