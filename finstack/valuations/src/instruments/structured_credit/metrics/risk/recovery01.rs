//! Recovery01 calculator for StructuredCredit.
//!
//! Computes Recovery01 (recovery rate sensitivity) using finite differences.
//! Recovery01 measures the change in PV for a 1% (100bp) change in recovery rate.
//!
//! # Note
//!
//! Recovery rate is model-based (RecoveryModelSpec). This calculator:
//! - For Constant recovery: bumps the constant rate
//! - For AssetDefault recovery: bumps the base_recovery_rate in DefaultAssumptions

use crate::instruments::structured_credit::StructuredCredit;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Standard recovery rate bump: 1% (0.01)
const RECOVERY_BUMP: f64 = 0.01;

/// Recovery01 calculator for StructuredCredit.
pub struct Recovery01Calculator;

impl MetricCalculator for Recovery01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let instrument: &StructuredCredit = context.instrument_as()?;
        let as_of = context.as_of;
        let _base_pv = context.base_value.amount();

        use crate::instruments::structured_credit::components::specs::RecoveryModelSpec;

        // Get current recovery spec and create bumped versions
        let recovery_up = match &instrument.recovery_spec {
            RecoveryModelSpec::Constant { rate } => {
                RecoveryModelSpec::Constant {
                    rate: (rate + RECOVERY_BUMP).clamp(0.0, 1.0),
                }
            }
            RecoveryModelSpec::AssetDefault { asset_type: _ } => {
                // For asset-based recovery, bump the base_recovery_rate in assumptions
                let mut assumptions_up = instrument.default_assumptions.clone();
                assumptions_up.base_recovery_rate =
                    (assumptions_up.base_recovery_rate + RECOVERY_BUMP).clamp(0.0, 1.0);
                let mut inst_up = instrument.clone();
                inst_up.default_assumptions = assumptions_up;
                let pv_up = inst_up.price(context.curves.as_ref(), as_of)?.amount();

                // Create down scenario
                let mut assumptions_down = instrument.default_assumptions.clone();
                assumptions_down.base_recovery_rate =
                    (assumptions_down.base_recovery_rate - RECOVERY_BUMP).clamp(0.0, 1.0);
                let mut inst_down = instrument.clone();
                inst_down.default_assumptions = assumptions_down;
                let pv_down = inst_down.price(context.curves.as_ref(), as_of)?.amount();

                // Recovery01 = (PV_up - PV_down) / (2 * bump_size)
                return Ok((pv_up - pv_down) / (2.0 * RECOVERY_BUMP));
            }
        };

        // For Constant recovery, create bumped instruments
        let mut inst_up = instrument.clone();
        inst_up.recovery_spec = recovery_up;
        let pv_up = inst_up.price(context.curves.as_ref(), as_of)?.amount();

        let recovery_down = match &instrument.recovery_spec {
            RecoveryModelSpec::Constant { rate } => {
                RecoveryModelSpec::Constant {
                    rate: (rate - RECOVERY_BUMP).clamp(0.0, 1.0),
                }
            }
            _ => unreachable!(), // Already handled above
        };

        let mut inst_down = instrument.clone();
        inst_down.recovery_spec = recovery_down;
        let pv_down = inst_down.price(context.curves.as_ref(), as_of)?.amount();

        // Recovery01 = (PV_up - PV_down) / (2 * bump_size)
        let recovery01 = (pv_up - pv_down) / (2.0 * RECOVERY_BUMP);

        Ok(recovery01)
    }
}

