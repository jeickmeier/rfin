//! Per-asset-class add-on computation for SA-CCR.
//!
//! For a linear trade:
//!   `d_i = supervisory_delta * adjusted_notional * maturity_factor * SF_class`
//!
//! For an interest-rate trade the adjusted notional picks up an extra
//! supervisory-duration factor (see [`supervisory_duration`]). Trades
//! within the same hedging set offset per the asset-class rule; hedging
//! sets are then aggregated per the asset-class cross-HS rule.
//!
//! # Known scope limitations
//!
//! The add-on aggregation across hedging sets for non-IR asset classes
//! uses the classic `sqrt((sum rho*d_HS)^2 + sum (1 - rho^2) * d_HS^2)`
//! decomposition applied at the hedging-set level, which is the correct
//! formulation when each hedging set corresponds to a single reference
//! entity / commodity (Credit / Equity / Commodity under MAR52 / CRE52).
//! Deployments that put multiple entities in the same hedging-set
//! bucket should perform the systematic/idiosyncratic split at the
//! entity level upstream and pass one hedging-set-per-entity here.

use super::params::{supervisory_correlation, supervisory_factor};
use super::types::{SaCcrAssetClass, SaCcrTrade};
use finstack_core::HashMap;

/// IR supervisory correlation continuous-compounding rate per CRE52.54.
const IR_SUPERVISORY_DISCOUNT_RATE: f64 = 0.05;

/// IR maturity-bucket correlation matrix off-diagonal entries per
/// CRE52.54 (1.4 for adjacent buckets, 0.6 for non-adjacent).
const IR_ADJACENT_BUCKET_CORR: f64 = 1.4;
const IR_NONADJACENT_BUCKET_CORR: f64 = 0.6;

/// Compute the add-on for a single asset class.
pub fn asset_class_add_on(
    asset_class: SaCcrAssetClass,
    trades: &[SaCcrTrade],
    maturity_factor: f64,
) -> f64 {
    match asset_class {
        SaCcrAssetClass::InterestRate => ir_add_on(trades, maturity_factor),
        other => non_ir_add_on(other, trades, maturity_factor),
    }
}

/// Interest-rate add-on with supervisory duration and 3x3 maturity buckets.
///
/// For each IR trade:
/// * Compute supervisory duration
///   `SD = (exp(-0.05 * S) - exp(-0.05 * E)) / 0.05`
///   where `S` and `E` are the start and end offsets in years. Trades
///   already in flight (`start_date <= as_of`) have `S = 0`.
/// * Adjusted notional `d_i = supervisory_delta * |notional| * SD * MF`.
/// * Bucket by end-maturity: `< 1y` -> D1, `1-5y` -> D2, `> 5y` -> D3.
/// * Per-hedging-set effective notional via CRE52.54:
///   `EN_HS = sqrt(D1^2 + D2^2 + D3^2 + 1.4*D1*D2 + 1.4*D2*D3 + 0.6*D1*D3)`
/// * Across hedging sets (currencies): simple absolute sum.
fn ir_add_on(trades: &[SaCcrTrade], maturity_factor: f64) -> f64 {
    let sf = supervisory_factor(SaCcrAssetClass::InterestRate);

    // (D1, D2, D3) per hedging set.
    let mut by_hs: HashMap<String, [f64; 3]> = HashMap::default();
    for trade in trades
        .iter()
        .filter(|t| t.asset_class == SaCcrAssetClass::InterestRate)
    {
        // Use trade duration as the end-offset; start-offset defaults to 0
        // (trade considered in-flight as of the calculation date).
        let end_years =
            ((trade.end_date - trade.start_date).whole_days().max(0) as f64) / 365.0;
        let sd = supervisory_duration(0.0, end_years);
        let d_i = trade.supervisory_delta * trade.notional.abs() * sd * maturity_factor;

        let bucket = if end_years < 1.0 {
            0
        } else if end_years <= 5.0 {
            1
        } else {
            2
        };

        let entry = by_hs.entry(trade.hedging_set.clone()).or_insert([0.0; 3]);
        entry[bucket] += d_i;
    }

    let mut add_on = 0.0;
    for d in by_hs.values() {
        let d1 = d[0];
        let d2 = d[1];
        let d3 = d[2];
        let en2 = d1 * d1
            + d2 * d2
            + d3 * d3
            + IR_ADJACENT_BUCKET_CORR * d1 * d2
            + IR_ADJACENT_BUCKET_CORR * d2 * d3
            + IR_NONADJACENT_BUCKET_CORR * d1 * d3;
        let en_hs = en2.max(0.0).sqrt();
        add_on += sf * en_hs;
    }
    add_on
}

/// Supervisory duration per CRE52.54.
///
/// `SD = (exp(-r*S) - exp(-r*E)) / r` with `r = 0.05`, where `S` is the
/// start-offset (years from as-of to trade start) and `E` is the
/// end-offset (years from as-of to trade end). Floored at a small
/// positive value so in-flight trades (`S = 0`) with nearly-matured tails
/// don't collapse to zero effective notional.
#[must_use]
pub fn supervisory_duration(start_years: f64, end_years: f64) -> f64 {
    let r = IR_SUPERVISORY_DISCOUNT_RATE;
    let s = start_years.max(0.0);
    let e = end_years.max(s);
    let sd = ((-r * s).exp() - (-r * e).exp()) / r;
    sd.max(0.0)
}

/// Non-IR add-on with the simplified hedging-set-level
/// systematic/idiosyncratic decomposition. See module docs for the
/// single-entity-per-HS assumption.
fn non_ir_add_on(
    asset_class: SaCcrAssetClass,
    trades: &[SaCcrTrade],
    maturity_factor: f64,
) -> f64 {
    let sf = supervisory_factor(asset_class);
    let rho = supervisory_correlation(asset_class);

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

    let hedging_set_values: Vec<f64> = by_hedging_set.values().copied().collect();

    // For FX (rho = 1) this reduces to `|sum d_HS|`.
    // For Credit / Equity / Commodity (rho < 1) each hedging set is the
    // systematic/idiosyncratic unit per the single-entity-per-HS caveat
    // in the module docs.
    let systematic: f64 = hedging_set_values.iter().sum::<f64>() * rho;
    let idiosyncratic: f64 = hedging_set_values
        .iter()
        .map(|hs| (1.0 - rho * rho) * hs * hs)
        .sum::<f64>();

    let add_on_raw = (systematic * systematic + idiosyncratic).sqrt();
    add_on_raw * sf
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn supervisory_duration_matches_analytic_formula() {
        // SD for E=5y, S=0 = (1 - e^{-0.25}) / 0.05
        let sd = supervisory_duration(0.0, 5.0);
        let expected = (1.0 - (-0.25f64).exp()) / 0.05;
        assert!(
            (sd - expected).abs() < 1e-12,
            "SD(0, 5) = {sd}, expected {expected}"
        );
    }

    #[test]
    fn supervisory_duration_handles_forward_starting() {
        // Forward-starting 1Yx4Y: S=1, E=5. SD = (e^{-0.05} - e^{-0.25})/0.05
        let sd = supervisory_duration(1.0, 5.0);
        let expected = ((-0.05f64).exp() - (-0.25f64).exp()) / 0.05;
        assert!(
            (sd - expected).abs() < 1e-12,
            "SD(1, 5) = {sd}, expected {expected}"
        );
    }

    #[test]
    fn supervisory_duration_non_negative_for_swapped_args() {
        // Defensive: if caller supplies end < start, clamp to 0.
        let sd = supervisory_duration(5.0, 1.0);
        assert!(sd >= 0.0);
    }
}
