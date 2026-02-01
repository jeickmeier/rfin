//! Jump-to-Default metric for CDS Index.
//!
//! Calculates the instantaneous loss if a single constituent defaults immediately.
//!
//! ## Methodology
//!
//! ### When constituents data is available:
//! ```text
//! JTD_avg = (1 / N) × Σ(Weight_i × Notional × (1 - Recovery_i))
//! ```
//! Returns the **average** per-name default impact (single-name JTD).
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

use crate::instruments::credit_derivatives::cds::PayReceive;
use crate::instruments::credit_derivatives::cds_index::CDSIndex;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::{Error, Result};

/// Jump-to-default calculator for CDS Index.
pub struct JumpToDefaultCalculator;

fn infer_constituent_count(index_name: &str) -> Option<f64> {
    let name = index_name.to_ascii_lowercase();
    if name.contains("cdx") && name.contains("na") && name.contains("ig") {
        Some(125.0)
    } else if name.contains("cdx") && name.contains("na") && name.contains("hy") {
        Some(100.0)
    } else if name.contains("itraxx") && name.contains("crossover") {
        Some(75.0)
    } else if name.contains("itraxx") {
        Some(125.0)
    } else if name.contains("cdx.em") || name.contains("cdx em") || name.contains("cdxem") {
        Some(40.0)
    } else {
        None
    }
}

impl MetricCalculator for JumpToDefaultCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let index: &CDSIndex = context.instrument_as()?;

        // Check if we have constituent data for more accurate calculation
        if !index.constituents.is_empty() {
            // Average per-name JTD using constituent-specific weights and recoveries
            let n = index.constituents.len() as f64;
            let sum_w: f64 = index.constituents.iter().map(|c| c.weight).sum();
            let norm = if sum_w > 0.0 { sum_w } else { 1.0 };

            let mut weighted_lgd = 0.0;
            for constituent in &index.constituents {
                let lgd = 1.0 - constituent.credit.recovery_rate;
                weighted_lgd += (constituent.weight / norm) * lgd;
            }

            let scale = index.index_factor;
            let avg_jtd = index.notional.amount() * scale * weighted_lgd / n;

            // Apply sign based on position
            let signed_jtd = match index.side {
                PayReceive::PayFixed => avg_jtd,
                PayReceive::ReceiveFixed => -avg_jtd,
            };

            Ok(signed_jtd)
        } else {
            // Simplified calculation using index-level parameters
            // Assume equal-weighted constituents
            let num_constituents = infer_constituent_count(&index.index_name).ok_or_else(|| {
                Error::Validation(format!(
                    "Cannot infer constituent count for index '{}'. Provide constituents or use \
                     a standard index name (e.g., CDX.NA.IG, CDX.NA.HY, iTraxx Europe).",
                    index.index_name
                ))
            })?;
            let avg_weight = 1.0 / num_constituents;
            let lgd = 1.0 - index.protection.recovery_rate;

            // Single name default impact
            let scale = index.index_factor;
            let single_name_jtd = avg_weight * index.notional.amount() * scale * lgd;

            // Apply sign based on position
            let signed_jtd = match index.side {
                PayReceive::PayFixed => single_name_jtd,
                PayReceive::ReceiveFixed => -single_name_jtd,
            };

            Ok(signed_jtd)
        }
    }
}
