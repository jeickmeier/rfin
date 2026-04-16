//! Per-asset-class add-on computation for SA-CCR.
//!
//! For each trade:
//!   `d_i = supervisory_delta * adjusted_notional * maturity_factor * SF`
//!
//! Trades within the same hedging set are aggregated with partial
//! offsetting; hedging sets are then aggregated per the asset-class rule.

use super::params::{supervisory_correlation, supervisory_factor};
use super::types::{SaCcrAssetClass, SaCcrTrade};
use finstack_core::HashMap;

/// Compute the add-on for a single asset class.
///
/// Groups trades by hedging set, computes the effective notional per
/// hedging set, then aggregates across hedging sets using the
/// supervisory correlation parameter.
pub fn asset_class_add_on(
    asset_class: SaCcrAssetClass,
    trades: &[SaCcrTrade],
    maturity_factor: f64,
) -> f64 {
    let sf = supervisory_factor(asset_class);
    let rho = supervisory_correlation(asset_class);

    // Group by hedging set and compute effective notional.
    let mut by_hedging_set: HashMap<String, f64> = HashMap::default();
    for trade in trades.iter().filter(|t| t.asset_class == asset_class) {
        let d_i = trade.supervisory_delta * trade.notional.abs() * maturity_factor;
        *by_hedging_set
            .entry(trade.hedging_set.clone())
            .or_insert(0.0) += d_i;
    }

    if by_hedging_set.is_empty() {
        return 0.0;
    }

    // Within each hedging set, the effective notional is the sum of d_i.
    // Across hedging sets, use the correlation-based aggregation.
    let hedging_set_values: Vec<f64> = by_hedging_set.values().copied().collect();

    // Add-on for the asset class per BCBS 279.
    // For asset classes with rho = 1 (IR, FX), trades fully offset within HS.
    // For other classes: systematic + idiosyncratic decomposition.
    let systematic: f64 = hedging_set_values.iter().sum::<f64>() * rho;
    let idiosyncratic: f64 = hedging_set_values
        .iter()
        .map(|hs| (1.0 - rho * rho) * hs * hs)
        .sum::<f64>();

    let add_on_raw = (systematic * systematic + idiosyncratic).sqrt();
    add_on_raw * sf
}
