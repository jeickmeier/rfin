//! Hurdle01 calculator for PrivateMarketsFund.
//!
//! Computes Hurdle01 (hurdle rate sensitivity) using finite differences.
//! Hurdle01 measures the change in PV for a 1bp (0.0001) change in hurdle IRR rate.
//!
//! # Formula
//! ```text
//! Hurdle01 = (PV(hurdle_rate + 1bp) - PV(hurdle_rate - 1bp)) / (2 * bump_size)
//! ```
//! Where bump_size is 1bp (0.0001).
//!
//! # Note
//!
//! Hurdle rates appear in:
//! - `Tranche::PreferredIrr { irr }` - preferred return hurdle
//! - `Tranche::PromoteTier { hurdle: Hurdle::Irr { rate }, ... }` - promote tier hurdle
//!
//! Higher hurdle rates increase the LP preferred return, potentially reducing GP carry
//! and affecting LP valuation.

use crate::instruments::common::traits::Instrument;
use crate::instruments::equity::pe_fund::PrivateMarketsFund;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Standard hurdle bump: 1bp (0.0001)
const HURDLE_BUMP: f64 = 0.0001;

/// Hurdle01 calculator for PrivateMarketsFund.
pub struct Hurdle01Calculator;

impl MetricCalculator for Hurdle01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let fund: &PrivateMarketsFund = context.instrument_as()?;
        let as_of = context.as_of;

        use crate::instruments::equity::pe_fund::waterfall::{Hurdle, Tranche};

        // Bump hurdle rates up by 1bp
        let mut spec_up = fund.spec.clone();
        for tranche in &mut spec_up.tranches {
            match tranche {
                Tranche::PreferredIrr { irr } => {
                    *irr = (*irr + HURDLE_BUMP).max(0.0);
                }
                Tranche::PromoteTier { hurdle, .. } => {
                    let Hurdle::Irr { rate } = hurdle;
                    *rate = (*rate + HURDLE_BUMP).max(0.0);
                }
                _ => {}
            }
        }

        let mut fund_up = fund.clone();
        fund_up.spec = spec_up;
        let pv_up = fund_up.value(context.curves.as_ref(), as_of)?.amount();

        // Bump hurdle rates down by 1bp
        let mut spec_down = fund.spec.clone();
        for tranche in &mut spec_down.tranches {
            match tranche {
                Tranche::PreferredIrr { irr } => {
                    *irr = (*irr - HURDLE_BUMP).max(0.0);
                }
                Tranche::PromoteTier { hurdle, .. } => {
                    let Hurdle::Irr { rate } = hurdle;
                    *rate = (*rate - HURDLE_BUMP).max(0.0);
                }
                _ => {}
            }
        }

        let mut fund_down = fund.clone();
        fund_down.spec = spec_down;
        let pv_down = fund_down.value(context.curves.as_ref(), as_of)?.amount();

        // Hurdle01 = (PV_up - PV_down) / (2 * bump_size)
        // Higher hurdle typically benefits LPs (more preferred return before GP carry)
        // Result is per 1bp change in hurdle rate
        let hurdle01 = (pv_up - pv_down) / (2.0 * HURDLE_BUMP);

        Ok(hurdle01)
    }
}
