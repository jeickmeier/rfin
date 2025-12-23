//! Severity01 calculator for StructuredCredit.
//!
//! Computes Severity01 (loss severity sensitivity) using finite differences.
//! Severity01 measures the change in PV for a 1% (0.01) change in loss severity.
//!
//! # Formula
//! ```text
//! Severity01 = (PV(severity + 1%) - PV(severity - 1%)) / (2 * bump_size)
//! ```
//! Where bump_size is 1% (0.01).
//!
//! # Note
//! Loss Severity = 1 - Recovery Rate (LGD = Loss Given Default)
//! This metric is related to Recovery01 but measures sensitivity to loss severity
//! rather than recovery. For constant recovery, Severity01 ≈ -Recovery01.

use crate::instruments::structured_credit::StructuredCredit;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Standard severity bump: 1% (0.01)
const SEVERITY_BUMP: f64 = 0.01;

/// Severity01 calculator for StructuredCredit.
pub struct Severity01Calculator;

impl MetricCalculator for Severity01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let instrument: &StructuredCredit = context.instrument_as()?;
        let as_of = context.as_of;

        use crate::cashflow::builder::RecoveryModelSpec;

        // Loss Severity = 1 - Recovery Rate
        // So bumping severity up means bumping recovery down, and vice versa
        let recovery_up = RecoveryModelSpec {
            rate: (instrument.recovery_spec.rate - SEVERITY_BUMP).clamp(0.0, 1.0),
            recovery_lag: instrument.recovery_spec.recovery_lag,
        };

        let recovery_down = RecoveryModelSpec {
            rate: (instrument.recovery_spec.rate + SEVERITY_BUMP).clamp(0.0, 1.0),
            recovery_lag: instrument.recovery_spec.recovery_lag,
        };

        // Calculate up scenario (lower recovery = higher severity)
        let mut inst_up = instrument.clone();
        inst_up.recovery_spec = recovery_up;
        let pv_up = inst_up.price(context.curves.as_ref(), as_of)?.amount();

        // Calculate down scenario (higher recovery = lower severity)
        let mut inst_down = instrument.clone();
        inst_down.recovery_spec = recovery_down;
        let pv_down = inst_down.price(context.curves.as_ref(), as_of)?.amount();

        // Severity01 = (PV_up - PV_down) / (2 * bump_size)
        // PV_up is with lower recovery (higher severity)
        // PV_down is with higher recovery (lower severity)
        let severity01 = (pv_up - pv_down) / (2.0 * SEVERITY_BUMP);

        Ok(severity01)
    }
}
