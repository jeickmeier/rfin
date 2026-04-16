//! SA-CCR replacement cost computation.
//!
//! RC differs for margined vs. unmargined netting sets.

use super::types::{SaCcrNettingSetConfig, SaCcrTrade};

/// Compute replacement cost for a netting set.
///
/// Unmargined:
///   `RC = max(V - C, 0)`
///   where V = sum of trade MTMs, C = net collateral
///
/// Margined:
///   `RC = max(V - C, TH + MTA - NICA, 0)`
///   The second term captures the minimum possible exposure
///   given the margin agreement mechanics.
pub fn replacement_cost(config: &SaCcrNettingSetConfig, trades: &[SaCcrTrade]) -> f64 {
    let v: f64 = trades.iter().map(|t| t.mtm).sum();
    let c = config.collateral;

    if config.is_margined {
        let margin_term = config.threshold + config.mta - config.nica;
        f64::max(v - c, f64::max(margin_term, 0.0))
    } else {
        f64::max(v - c, 0.0)
    }
}
