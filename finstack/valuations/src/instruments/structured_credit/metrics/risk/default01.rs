//! Default01 calculator for StructuredCredit.
//!
//! Computes Default01 (default rate sensitivity) using finite differences.
//! Default01 measures the change in PV for a 1bp (0.0001) change in default rate.
//!
//! # Formula
//! ```text
//! Default01 = (PV(default_rate + 1bp) - PV(default_rate - 1bp)) / (2 * bump_size)
//! ```
//! Where bump_size is 1bp (0.0001) for CDR-based bumps.
//!
//! # Note
//! Default rate can be specified via:
//! - DefaultModelSpec::Sda { multiplier } - bumps multiplier
//! - DefaultModelSpec::ConstantCdr { cdr } - bumps CDR
//! - DefaultModelSpec::ConstantMdr { mdr } - bumps MDR
//! - DefaultModelSpec::AssetDefault - bumps base_cdr_annual in DefaultAssumptions

use crate::instruments::structured_credit::StructuredCredit;
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

        use crate::instruments::structured_credit::components::specs::DefaultModelSpec;

        // Get current default spec and create bumped versions
        let default_up = match &instrument.default_spec {
            DefaultModelSpec::Sda { multiplier } => {
                DefaultModelSpec::Sda {
                    multiplier: (multiplier + DEFAULT_BUMP_CDR).max(0.0),
                }
            }
            DefaultModelSpec::ConstantCdr { cdr } => {
                DefaultModelSpec::ConstantCdr {
                    cdr: (cdr + DEFAULT_BUMP_CDR).max(0.0),
                }
            }
            DefaultModelSpec::ConstantMdr { mdr } => {
                // Convert MDR to equivalent CDR bump
                // CDR = 1 - (1 - MDR)^12, so dMDR ≈ dCDR / 12 for small bumps
                DefaultModelSpec::ConstantMdr {
                    mdr: (mdr + DEFAULT_BUMP_CDR / 12.0).max(0.0),
                }
            }
            DefaultModelSpec::AssetDefault { asset_type: _ } => {
                // For asset-based default, bump the base_cdr_annual in assumptions
                let mut assumptions_up = instrument.default_assumptions.clone();
                assumptions_up.base_cdr_annual =
                    (assumptions_up.base_cdr_annual + DEFAULT_BUMP_CDR).max(0.0);
                let mut inst_up = instrument.clone();
                inst_up.default_assumptions = assumptions_up;
                let pv_up = inst_up.price(context.curves.as_ref(), as_of)?.amount();

                // Create down scenario
                let mut assumptions_down = instrument.default_assumptions.clone();
                assumptions_down.base_cdr_annual =
                    (assumptions_down.base_cdr_annual - DEFAULT_BUMP_CDR).max(0.0);
                let mut inst_down = instrument.clone();
                inst_down.default_assumptions = assumptions_down;
                let pv_down = inst_down.price(context.curves.as_ref(), as_of)?.amount();

                // Default01 = (PV_up - PV_down) / (2 * bump_size)
                return Ok((pv_up - pv_down) / (2.0 * DEFAULT_BUMP_CDR));
            }
        };

        // For other spec types, create bumped instruments
        let mut inst_up = instrument.clone();
        inst_up.default_spec = default_up;
        let pv_up = inst_up.price(context.curves.as_ref(), as_of)?.amount();

        let default_down = match &instrument.default_spec {
            DefaultModelSpec::Sda { multiplier } => {
                DefaultModelSpec::Sda {
                    multiplier: (multiplier - DEFAULT_BUMP_CDR).max(0.0),
                }
            }
            DefaultModelSpec::ConstantCdr { cdr } => {
                DefaultModelSpec::ConstantCdr {
                    cdr: (cdr - DEFAULT_BUMP_CDR).max(0.0),
                }
            }
            DefaultModelSpec::ConstantMdr { mdr } => {
                DefaultModelSpec::ConstantMdr {
                    mdr: (mdr - DEFAULT_BUMP_CDR / 12.0).max(0.0),
                }
            }
            _ => unreachable!(), // Already handled above
        };

        let mut inst_down = instrument.clone();
        inst_down.default_spec = default_down;
        let pv_down = inst_down.price(context.curves.as_ref(), as_of)?.amount();

        // Default01 = (PV_up - PV_down) / (2 * bump_size)
        let default01 = (pv_up - pv_down) / (2.0 * DEFAULT_BUMP_CDR);

        Ok(default01)
    }
}

