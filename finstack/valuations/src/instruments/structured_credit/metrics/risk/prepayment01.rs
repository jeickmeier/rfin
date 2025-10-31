//! Prepayment01 calculator for StructuredCredit.
//!
//! Computes Prepayment01 (prepayment rate sensitivity) using finite differences.
//! Prepayment01 measures the change in PV for a 1bp (0.0001) change in prepayment rate.
//!
//! # Formula
//! ```text
//! Prepayment01 = (PV(prepayment_rate + 1bp) - PV(prepayment_rate - 1bp)) / (2 * bump_size)
//! ```
//! Where bump_size is 1bp (0.0001) for CPR-based bumps.
//!
//! # Note
//! Prepayment rate can be specified via:
//! - PrepaymentModelSpec::Psa { multiplier } - bumps multiplier
//! - PrepaymentModelSpec::ConstantCpr { cpr } - bumps CPR
//! - PrepaymentModelSpec::ConstantSmm { smm } - bumps SMM
//! - PrepaymentModelSpec::AssetDefault - bumps base_cpr_annual in DefaultAssumptions

use crate::instruments::structured_credit::StructuredCredit;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Standard prepayment bump: 1bp (0.0001) for CPR
const PREPAYMENT_BUMP_CPR: f64 = 0.0001;

/// Prepayment01 calculator for StructuredCredit.
pub struct Prepayment01Calculator;

impl MetricCalculator for Prepayment01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let instrument: &StructuredCredit = context.instrument_as()?;
        let as_of = context.as_of;

        use crate::instruments::structured_credit::components::specs::PrepaymentModelSpec;

        // Get current prepayment spec and create bumped versions
        let prepayment_up = match &instrument.prepayment_spec {
            PrepaymentModelSpec::Psa { multiplier } => {
                PrepaymentModelSpec::Psa {
                    multiplier: multiplier + PREPAYMENT_BUMP_CPR,
                }
            }
            PrepaymentModelSpec::ConstantCpr { cpr } => {
                PrepaymentModelSpec::ConstantCpr {
                    cpr: (cpr + PREPAYMENT_BUMP_CPR).max(0.0),
                }
            }
            PrepaymentModelSpec::ConstantSmm { smm } => {
                // Convert SMM to equivalent CPR bump
                // CPR = 1 - (1 - SMM)^12, so dSMM ≈ dCPR / 12 for small bumps
                PrepaymentModelSpec::ConstantSmm {
                    smm: (smm + PREPAYMENT_BUMP_CPR / 12.0).max(0.0),
                }
            }
            PrepaymentModelSpec::AssetDefault { asset_type: _ } => {
                // For asset-based prepayment, bump the base_cpr_annual in assumptions
                let mut assumptions_up = instrument.default_assumptions.clone();
                assumptions_up.base_cpr_annual =
                    (assumptions_up.base_cpr_annual + PREPAYMENT_BUMP_CPR).max(0.0);
                let mut inst_up = instrument.clone();
                inst_up.default_assumptions = assumptions_up;
                let pv_up = inst_up.price(context.curves.as_ref(), as_of)?.amount();

                // Create down scenario
                let mut assumptions_down = instrument.default_assumptions.clone();
                assumptions_down.base_cpr_annual =
                    (assumptions_down.base_cpr_annual - PREPAYMENT_BUMP_CPR).max(0.0);
                let mut inst_down = instrument.clone();
                inst_down.default_assumptions = assumptions_down;
                let pv_down = inst_down.price(context.curves.as_ref(), as_of)?.amount();

                // Prepayment01 = (PV_up - PV_down) / (2 * bump_size)
                return Ok((pv_up - pv_down) / (2.0 * PREPAYMENT_BUMP_CPR));
            }
        };

        // For other spec types, create bumped instruments
        let mut inst_up = instrument.clone();
        inst_up.prepayment_spec = prepayment_up;
        let pv_up = inst_up.price(context.curves.as_ref(), as_of)?.amount();

        let prepayment_down = match &instrument.prepayment_spec {
            PrepaymentModelSpec::Psa { multiplier } => {
                PrepaymentModelSpec::Psa {
                    multiplier: (multiplier - PREPAYMENT_BUMP_CPR).max(0.0),
                }
            }
            PrepaymentModelSpec::ConstantCpr { cpr } => {
                PrepaymentModelSpec::ConstantCpr {
                    cpr: (cpr - PREPAYMENT_BUMP_CPR).max(0.0),
                }
            }
            PrepaymentModelSpec::ConstantSmm { smm } => {
                PrepaymentModelSpec::ConstantSmm {
                    smm: (smm - PREPAYMENT_BUMP_CPR / 12.0).max(0.0),
                }
            }
            _ => unreachable!(), // Already handled above
        };

        let mut inst_down = instrument.clone();
        inst_down.prepayment_spec = prepayment_down;
        let pv_down = inst_down.price(context.curves.as_ref(), as_of)?.amount();

        // Prepayment01 = (PV_up - PV_down) / (2 * bump_size)
        let prepayment01 = (pv_up - pv_down) / (2.0 * PREPAYMENT_BUMP_CPR);

        Ok(prepayment01)
    }
}

