//! Carry01 calculator for PrivateMarketsFund.
//!
//! Computes Carry01 (GP carry sensitivity) using finite differences.
//! Carry01 measures the change in PV for a 1bp (0.0001 = 0.01%) change in GP share.
//!
//! # Formula
//! ```text
//! Carry01 = (PV(GP_share + 1bp) - PV(GP_share - 1bp)) / (2 * bump_size)
//! ```
//! Where bump_size is 1bp (0.0001) for GP share changes.
//!
//! # Note
//! GP carry is determined by the waterfall allocation (promote tiers, catch-up).
//! This metric bumps the GP share in all promote tiers and catch-up tranches
//! and measures the impact on LP valuation (lower carry = higher LP value).

use crate::instruments::common::traits::Instrument;
use crate::instruments::private_markets_fund::PrivateMarketsFund;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Standard carry bump: 1bp (0.0001 = 0.01%)
const CARRY_BUMP: f64 = 0.0001;

/// Carry01 calculator for PrivateMarketsFund.
pub struct Carry01Calculator;

impl MetricCalculator for Carry01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let fund: &PrivateMarketsFund = context.instrument_as()?;
        let as_of = context.as_of;

        use crate::instruments::private_markets_fund::waterfall::Tranche;

        // Bump GP share up in all promote tiers and catch-up tranches
        let mut spec_up = fund.spec.clone();
        for tranche in &mut spec_up.tranches {
            match tranche {
                Tranche::CatchUp { gp_share } => {
                    *gp_share = (*gp_share + CARRY_BUMP).clamp(0.0, 1.0);
                }
                Tranche::PromoteTier {
                    lp_share, gp_share, ..
                } => {
                    // Adjust GP share and renormalize LP share to sum to 1.0
                    let new_gp = (*gp_share + CARRY_BUMP).clamp(0.0, 1.0);
                    let new_lp = (1.0 - new_gp).max(0.0);
                    *gp_share = new_gp;
                    *lp_share = new_lp;
                }
                _ => {
                    // ReturnOfCapital, PreferredIrr don't affect carry directly
                }
            }
        }

        let mut fund_up = fund.clone();
        fund_up.spec = spec_up;
        let pv_up = fund_up.value(context.curves.as_ref(), as_of)?.amount();

        // Bump GP share down in all promote tiers and catch-up tranches
        let mut spec_down = fund.spec.clone();
        for tranche in &mut spec_down.tranches {
            match tranche {
                Tranche::CatchUp { gp_share } => {
                    *gp_share = (*gp_share - CARRY_BUMP).clamp(0.0, 1.0);
                }
                Tranche::PromoteTier {
                    lp_share, gp_share, ..
                } => {
                    let new_gp = (*gp_share - CARRY_BUMP).clamp(0.0, 1.0);
                    let new_lp = (1.0 - new_gp).max(0.0);
                    *gp_share = new_gp;
                    *lp_share = new_lp;
                }
                _ => {}
            }
        }

        let mut fund_down = fund.clone();
        fund_down.spec = spec_down;
        let pv_down = fund_down.value(context.curves.as_ref(), as_of)?.amount();

        // Carry01 = (PV_up - PV_down) / (2 * bump_size)
        // Higher GP carry means less for LPs, so PV_up < PV_down typically
        // Result is per 1bp change in GP share
        let carry01 = (pv_up - pv_down) / (2.0 * CARRY_BUMP);

        Ok(carry01)
    }
}
