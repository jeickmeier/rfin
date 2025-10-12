//! Jump-to-Default metric for CDS Index.
//!
//! Calculates the instantaneous loss if one constituent in the index defaults immediately.
//!
//! ## Methodology
//!
//! ### When constituents data is available:
//! ```text
//! JTD = Σ(Weight_i × Notional × (1 - Recovery_i))
//! ```
//! Returns the impact of each constituent defaulting (can compute per-name JTD).
//!
//! ### When using index-level curve only (simplified):
//! ```text
//! JTD = (1 / N) × Notional × (1 - Avg_Recovery)
//! ```
//! Where N = number of constituents in the index (e.g., 125 for CDX IG)
//!
//! ## Interpretation
//! - For protection **buyer**: JTD is positive (gain on default)
//! - For protection **seller**: JTD is negative (loss on default)
//!
//! ## Example
//! - CDX IG (125 names): $10M index with 40% recovery → JTD ≈ $48K per name default

use crate::instruments::cds::PayReceive;
use crate::instruments::cds_index::CDSIndex;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Jump-to-default calculator for CDS Index.
pub struct JumpToDefaultCalculator;

impl MetricCalculator for JumpToDefaultCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let index: &CDSIndex = context.instrument_as()?;

        // Check if we have constituent data for more accurate calculation
        if !index.constituents.is_empty() {
            // Use constituent-specific weights and recoveries
            let mut total_jtd = 0.0;

            for constituent in &index.constituents {
                let lgd = 1.0 - constituent.credit.recovery_rate;
                let constituent_jtd = constituent.weight * index.notional.amount() * lgd;
                total_jtd += constituent_jtd;
            }

            // Apply sign based on position
            let signed_jtd = match index.side {
                PayReceive::PayFixed => total_jtd,
                PayReceive::ReceiveFixed => -total_jtd,
            };

            Ok(signed_jtd)
        } else {
            // Simplified calculation using index-level parameters
            // Assume equal-weighted constituents
            let num_constituents = 125.0; // Default for standard indices (CDX IG, iTraxx)
            let avg_weight = 1.0 / num_constituents;
            let lgd = 1.0 - index.protection.recovery_rate;

            // Single name default impact
            let single_name_jtd = avg_weight * index.notional.amount() * lgd;

            // Apply sign based on position
            let signed_jtd = match index.side {
                PayReceive::PayFixed => single_name_jtd,
                PayReceive::ReceiveFixed => -single_name_jtd,
            };

            Ok(signed_jtd)
        }
    }
}
