//! Jump-to-Default metric for single-name CDS.
//!
//! Calculates the instantaneous loss if the reference entity defaults immediately.
//! This is a key risk metric that measures the immediate P&L impact of a credit event.
//!
//! ## Formula
//! ```text
//! JTD = Notional × (1 - Recovery Rate)
//! ```
//!
//! ## Interpretation
//! - For protection **buyer** (long credit risk): JTD is positive (gain on default)
//! - For protection **seller** (short credit risk): JTD is negative (loss on default)
//!
//! ## Example
//! - $10M CDS with 40% recovery → JTD = $6M

use crate::instruments::cds::{CreditDefaultSwap, PayReceive};
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Jump-to-default calculator for single-name CDS.
pub struct JumpToDefaultCalculator;

impl MetricCalculator for JumpToDefaultCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let cds: &CreditDefaultSwap = context.instrument_as()?;

        // Loss given default
        let lgd = 1.0 - cds.protection.recovery_rate;

        // Jump-to-default amount (unsigned)
        let jtd_amount = cds.notional.amount() * lgd;

        // Apply sign based on position:
        // - Protection buyer: positive JTD (gains on default)
        // - Protection seller: negative JTD (loses on default)
        let signed_jtd = match cds.side {
            PayReceive::PayFixed => jtd_amount,      // Buyer gains
            PayReceive::ReceiveFixed => -jtd_amount, // Seller loses
        };

        Ok(signed_jtd)
    }
}
