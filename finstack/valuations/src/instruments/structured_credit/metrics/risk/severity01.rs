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

        use crate::instruments::structured_credit::components::specs::RecoveryModelSpec;

        // Loss Severity = 1 - Recovery Rate
        // So bumping severity up means bumping recovery down, and vice versa
        let recovery_up = match &instrument.recovery_spec {
            RecoveryModelSpec::Constant { rate } => {
                RecoveryModelSpec::Constant {
                    rate: (rate - SEVERITY_BUMP).clamp(0.0, 1.0),
                }
            }
            RecoveryModelSpec::AssetDefault { asset_type: _ } => {
                // For asset-based recovery, bump the base_recovery_rate down (severity up)
                let mut assumptions_up = instrument.default_assumptions.clone();
                assumptions_up.base_recovery_rate =
                    (assumptions_up.base_recovery_rate - SEVERITY_BUMP).clamp(0.0, 1.0);
                let mut inst_up = instrument.clone();
                inst_up.default_assumptions = assumptions_up;
                let pv_up = inst_up.price(context.curves.as_ref(), as_of)?.amount();

                // Create down scenario (recovery up = severity down)
                let mut assumptions_down = instrument.default_assumptions.clone();
                assumptions_down.base_recovery_rate =
                    (assumptions_down.base_recovery_rate + SEVERITY_BUMP).clamp(0.0, 1.0);
                let mut inst_down = instrument.clone();
                inst_down.default_assumptions = assumptions_down;
                let pv_down = inst_down.price(context.curves.as_ref(), as_of)?.amount();

                // Severity01 = (PV_up - PV_down) / (2 * bump_size)
                // PV_up is with lower recovery (higher severity)
                // PV_down is with higher recovery (lower severity)
                // Positive severity01 means PV decreases as severity increases
                return Ok((pv_up - pv_down) / (2.0 * SEVERITY_BUMP));
            }
        };

        // For Constant recovery, create bumped instruments
        let mut inst_up = instrument.clone();
        inst_up.recovery_spec = recovery_up;
        let pv_up = inst_up.price(context.curves.as_ref(), as_of)?.amount();

        let recovery_down = match &instrument.recovery_spec {
            RecoveryModelSpec::Constant { rate } => {
                RecoveryModelSpec::Constant {
                    rate: (rate + SEVERITY_BUMP).clamp(0.0, 1.0),
                }
            }
            _ => unreachable!(), // Already handled above
        };

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

