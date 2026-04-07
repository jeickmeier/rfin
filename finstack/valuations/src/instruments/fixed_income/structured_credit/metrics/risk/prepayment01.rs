//! Prepayment01 calculator for StructuredCredit.
//!
//! Computes Prepayment01 (prepayment rate sensitivity) using finite differences.
//! Prepayment01 measures the change in PV for a 1bp (0.0001) change in prepayment rate (CPR).
//!
//! # Formula
//! ```text
//! Prepayment01 = (PV(CPR + 1bp) - PV(CPR - 1bp)) / (2 * bump_size)
//! ```
//! Where bump_size is 1bp (0.0001) for CPR-based bumps.

use crate::instruments::fixed_income::structured_credit::StructuredCredit;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Standard prepayment bump: 1bp (0.0001) for CPR
const PREPAYMENT_BUMP_CPR: f64 = 0.0001;

/// Prepayment01 calculator for StructuredCredit.
pub(crate) struct Prepayment01Calculator;

impl MetricCalculator for Prepayment01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let instrument: &StructuredCredit = context.instrument_as()?;
        let as_of = context.as_of;

        use crate::cashflow::builder::PrepaymentModelSpec;

        // Create bumped prepayment specs
        let prepayment_up = PrepaymentModelSpec {
            cpr: (instrument.credit_model.prepayment_spec.cpr + PREPAYMENT_BUMP_CPR).max(0.0),
            curve: instrument.credit_model.prepayment_spec.curve.clone(),
        };

        let prepayment_down = PrepaymentModelSpec {
            cpr: (instrument.credit_model.prepayment_spec.cpr - PREPAYMENT_BUMP_CPR).max(0.0),
            curve: instrument.credit_model.prepayment_spec.curve.clone(),
        };

        // Calculate up scenario
        let mut inst_up = instrument.clone();
        inst_up.credit_model.prepayment_spec = prepayment_up;
        let pv_up = inst_up.price(context.curves.as_ref(), as_of)?.amount();

        // Calculate down scenario
        let mut inst_down = instrument.clone();
        inst_down.credit_model.prepayment_spec = prepayment_down;
        let pv_down = inst_down.price(context.curves.as_ref(), as_of)?.amount();

        // Prepayment01 = (PV_up - PV_down) / (2 * bump_size)
        let prepayment01 = (pv_up - pv_down) / (2.0 * PREPAYMENT_BUMP_CPR);

        Ok(prepayment01)
    }
}
