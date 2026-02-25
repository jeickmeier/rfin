//! Default01 calculator for StructuredCredit.
//!
//! Computes Default01 (default rate sensitivity) using finite differences.
//! Default01 measures the change in PV for a 1bp (0.0001) change in default rate (CDR).
//!
//! # Formula
//! ```text
//! Default01 = (PV(CDR + 1bp) - PV(CDR - 1bp)) / (2 * bump_size)
//! ```
//! Where bump_size is 1bp (0.0001) for CDR-based bumps.

use crate::instruments::fixed_income::structured_credit::StructuredCredit;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Standard default bump: 1bp (0.0001) for CDR
const DEFAULT_BUMP_CDR: f64 = 0.0001;

/// Default01 calculator for StructuredCredit.
pub struct Default01Calculator;

impl MetricCalculator for Default01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let instrument: &StructuredCredit = context.instrument_as()?;
        let as_of = context.as_of;

        use crate::cashflow::builder::DefaultModelSpec;

        // Create bumped default specs
        let default_up = DefaultModelSpec {
            cdr: (instrument.credit_model.default_spec.cdr + DEFAULT_BUMP_CDR).max(0.0),
            curve: instrument.credit_model.default_spec.curve.clone(),
        };

        let default_down = DefaultModelSpec {
            cdr: (instrument.credit_model.default_spec.cdr - DEFAULT_BUMP_CDR).max(0.0),
            curve: instrument.credit_model.default_spec.curve.clone(),
        };

        // Calculate up scenario
        let mut inst_up = instrument.clone();
        inst_up.credit_model.default_spec = default_up;
        let pv_up = inst_up.price(context.curves.as_ref(), as_of)?.amount();

        // Calculate down scenario
        let mut inst_down = instrument.clone();
        inst_down.credit_model.default_spec = default_down;
        let pv_down = inst_down.price(context.curves.as_ref(), as_of)?.amount();

        // Default01 = (PV_up - PV_down) / (2 * bump_size)
        let default01 = (pv_up - pv_down) / (2.0 * DEFAULT_BUMP_CDR);

        Ok(default01)
    }
}
