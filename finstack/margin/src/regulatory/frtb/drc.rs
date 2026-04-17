//! Default Risk Charge (DRC) computation.
//!
//! DRC captures jump-to-default risk for credit and equity positions
//! that delta/vega/curvature cannot model. It is NOT subject to
//! correlation scenarios.

use super::types::{DrcPosition, DrcSeniority};
use finstack_core::HashMap;

/// Prescribed DRC risk weights by rating bucket.
///
/// Source: Basel Framework MAR22.24 (FRTB Standardised Approach, DRC for
/// non-securitisations). Unrated exposures receive the BB-equivalent 15%
/// weight, and defaulted exposures receive 100%.
pub const DRC_RISK_WEIGHTS: &[(u8, f64)] = &[
    (1, 0.005), // AAA
    (2, 0.02),  // AA
    (3, 0.03),  // A
    (4, 0.06),  // BBB
    (5, 0.15),  // BB
    (6, 0.30),  // B
    (7, 0.50),  // CCC
    (8, 0.15),  // Unrated
    (9, 1.00),  // Defaulted
];

/// LGD assumptions by seniority.
pub const DRC_LGD: &[(DrcSeniority, f64)] = &[
    (DrcSeniority::SeniorUnsecured, 0.75),
    (DrcSeniority::Subordinated, 0.75),
    (DrcSeniority::Equity, 1.00),
    (DrcSeniority::Securitization, 1.00),
];

/// Hedge benefit ratio for DRC netting across buckets.
///
/// HBR = sum(net_long_JTD) / (sum(net_long_JTD) + sum(|net_short_JTD|))
/// Applied to short positions to limit hedge recognition.
const DRC_HEDGE_BENEFIT_DISALLOWANCE: f64 = 0.5;

/// Compute the Default Risk Charge.
///
/// DRC is computed in four steps:
///   1. Gross JTD = LGD * |jtd_amount| (direction carried by sign)
///   2. Net JTD within obligor (long/short offsetting with constraints)
///   3. Bucket-level: `WtS = sum(net_JTD * risk_weight)`
///   4. Across buckets: `DRC = max(sum_b WtS_b, 0)`
pub fn drc_charge(positions: &[DrcPosition]) -> f64 {
    if positions.is_empty() {
        return 0.0;
    }

    // Step 1 & 2: Compute net JTD per obligor.
    let mut net_jtd_by_issuer: HashMap<String, (f64, u8)> = HashMap::default();
    for pos in positions {
        let lgd = drc_lgd(pos.seniority);
        let gross_jtd = lgd * pos.jtd_amount;
        let entry = net_jtd_by_issuer
            .entry(pos.issuer.clone())
            .or_insert((0.0, pos.rating_bucket));
        entry.0 += gross_jtd;
    }

    // Step 3: Aggregate by rating bucket.
    let mut long_total = 0.0;
    let mut short_total = 0.0;
    let mut bucket_charges: HashMap<u8, f64> = HashMap::default();

    for (_, (net_jtd, rating_bucket)) in &net_jtd_by_issuer {
        let rw = drc_risk_weight(*rating_bucket);
        let weighted = net_jtd * rw;
        *bucket_charges.entry(*rating_bucket).or_insert(0.0) += weighted;

        if *net_jtd > 0.0 {
            long_total += net_jtd * rw;
        } else {
            short_total += (net_jtd * rw).abs();
        }
    }

    // Step 4: Apply hedge benefit ratio.
    // HBR limits the offset benefit of short positions.
    let hbr = if long_total + short_total > 0.0 {
        long_total / (long_total + short_total)
    } else {
        DRC_HEDGE_BENEFIT_DISALLOWANCE
    };

    // DRC = max(sum of long weighted JTD - HBR * sum of |short weighted JTD|, 0)
    let drc = long_total - hbr * short_total;
    f64::max(drc, 0.0)
}

/// Look up DRC risk weight by rating bucket.
///
/// Unknown buckets fall back to the Unrated weight (15% per MAR22.24),
/// matching how the Basel text treats exposures that lack an external
/// rating. Callers who want a stricter policy should validate rating
/// assignment upstream and not rely on this fallback.
fn drc_risk_weight(rating_bucket: u8) -> f64 {
    DRC_RISK_WEIGHTS
        .iter()
        .find(|(b, _)| *b == rating_bucket)
        .map(|(_, w)| *w)
        .unwrap_or(0.15) // Default: Unrated per MAR22.24
}

/// Look up LGD by seniority.
fn drc_lgd(seniority: DrcSeniority) -> f64 {
    DRC_LGD
        .iter()
        .find(|(s, _)| *s == seniority)
        .map(|(_, lgd)| *lgd)
        .unwrap_or(0.75) // Default: senior unsecured
}
