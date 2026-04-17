//! FRTB delta risk charge computation.
//!
//! Two-level aggregation (intra-bucket then inter-bucket) with
//! correlation scenario scaling.

use super::params::{commodity, csr, equity, fx, girr};
use super::types::{CorrelationScenario, FrtbRiskClass, FrtbSensitivities};
use finstack_core::HashMap;

/// Compute the delta risk charge for a single risk class under one
/// correlation scenario.
///
/// Formula (two-level aggregation):
///
/// 1. Weighted sensitivity: `WS_k = s_k * RW_k`
/// 2. Intra-bucket: `K_b = sqrt(max(sum_k sum_l rho_kl * WS_k * WS_l, 0))`
/// 3. Inter-bucket: `Delta = sqrt(max(sum_b sum_c gamma_bc * S_b * S_c, 0))`
///    where `S_b = sum_k(WS_k)` within bucket b, capped by `K_b`.
pub fn delta_charge(
    risk_class: FrtbRiskClass,
    sensitivities: &FrtbSensitivities,
    scenario: CorrelationScenario,
) -> f64 {
    match risk_class {
        FrtbRiskClass::Girr => girr_delta(sensitivities, scenario),
        FrtbRiskClass::CsrNonSec => csr_nonsec_delta(sensitivities, scenario),
        FrtbRiskClass::CsrSecCtp => csr_sec_ctp_delta(sensitivities, scenario),
        FrtbRiskClass::CsrSecNonCtp => csr_sec_nonctp_delta(sensitivities, scenario),
        FrtbRiskClass::Equity => equity_delta(sensitivities, scenario),
        FrtbRiskClass::Commodity => commodity_delta(sensitivities, scenario),
        FrtbRiskClass::Fx => fx_delta(sensitivities, scenario),
    }
}

// ---------------------------------------------------------------------------
// GIRR delta
// ---------------------------------------------------------------------------

fn girr_delta(sens: &FrtbSensitivities, scenario: CorrelationScenario) -> f64 {
    if sens.girr_delta.is_empty()
        && sens.girr_inflation_delta.is_empty()
        && sens.girr_xccy_basis_delta.is_empty()
    {
        return 0.0;
    }

    // Group by currency (bucket = currency for GIRR).
    let mut by_currency: HashMap<_, Vec<(f64, f64)>> = HashMap::default();
    for ((ccy, tenor), delta) in &sens.girr_delta {
        let rw = girr_risk_weight(tenor);
        let ws = delta * rw;
        let tenor_years = girr::tenor_to_years(tenor).unwrap_or(5.0);
        by_currency.entry(*ccy).or_default().push((ws, tenor_years));
    }

    // Add inflation and xccy basis to their currency bucket.
    for (ccy, delta) in &sens.girr_inflation_delta {
        let ws = delta * girr::GIRR_INFLATION_RISK_WEIGHT;
        by_currency.entry(*ccy).or_default().push((ws, -1.0)); // -1 = inflation sentinel
    }
    for (ccy, delta) in &sens.girr_xccy_basis_delta {
        let ws = delta * girr::GIRR_XCCY_BASIS_RISK_WEIGHT;
        by_currency.entry(*ccy).or_default().push((ws, -2.0)); // -2 = xccy basis sentinel
    }

    // Intra-bucket aggregation per currency.
    let mut bucket_results: Vec<(f64, f64)> = Vec::new(); // (K_b, S_b)
    for entries in by_currency.values() {
        let mut k_squared = 0.0;
        for (i, &(ws_i, tenor_i)) in entries.iter().enumerate() {
            for (j, &(ws_j, tenor_j)) in entries.iter().enumerate() {
                let rho = if i == j {
                    1.0
                } else {
                    intra_girr_correlation(tenor_i, tenor_j, scenario)
                };
                k_squared += rho * ws_i * ws_j;
            }
        }
        let k_b = k_squared.max(0.0).sqrt();
        let s_b: f64 = entries.iter().map(|(ws, _)| ws).sum();
        // Cap S_b by K_b.
        let s_b_capped = s_b.max(-k_b).min(k_b);
        bucket_results.push((k_b, s_b_capped));
    }

    // Inter-bucket aggregation across currencies.
    let gamma = scenario.scale_correlation(girr::GIRR_INTER_BUCKET_CORRELATION);
    inter_bucket_aggregate(&bucket_results, gamma)
}

/// Compute intra-GIRR correlation between two risk factors.
fn intra_girr_correlation(tenor_i: f64, tenor_j: f64, scenario: CorrelationScenario) -> f64 {
    // Special risk factors (inflation = -1, xccy basis = -2).
    let base_rho = match (tenor_i < 0.0, tenor_j < 0.0) {
        (true, true) => {
            // Both are special: inflation-inflation = 1, inflation-xccy = 0, xccy-xccy = 1.
            if (tenor_i - tenor_j).abs() < 0.5 {
                1.0
            } else {
                girr::GIRR_XCCY_BASIS_CORRELATION
            }
        }
        (true, false) | (false, true) => {
            // One special, one tenor: use inflation or xccy basis correlation.
            let special = if tenor_i < 0.0 { tenor_i } else { tenor_j };
            if special > -1.5 {
                // Inflation.
                girr::GIRR_INFLATION_CORRELATION
            } else {
                // Xccy basis.
                girr::GIRR_XCCY_BASIS_CORRELATION
            }
        }
        (false, false) => girr::girr_tenor_correlation(tenor_i, tenor_j),
    };
    scenario.scale_correlation(base_rho)
}

// ---------------------------------------------------------------------------
// CSR Non-Sec delta
// ---------------------------------------------------------------------------

fn csr_nonsec_delta(sens: &FrtbSensitivities, scenario: CorrelationScenario) -> f64 {
    if sens.csr_nonsec_delta.is_empty() {
        return 0.0;
    }
    csr_bucketed_delta(
        &sens.csr_nonsec_delta,
        csr::csr_nonsec_risk_weight,
        csr::CSR_NONSEC_INTRA_BUCKET_NAME_CORRELATION,
        csr::CSR_NONSEC_INTRA_BUCKET_TENOR_CORRELATION,
        csr::CSR_NONSEC_INTER_BUCKET_CORRELATION,
        scenario,
    )
}

fn csr_sec_ctp_delta(sens: &FrtbSensitivities, scenario: CorrelationScenario) -> f64 {
    if sens.csr_sec_ctp_delta.is_empty() {
        return 0.0;
    }
    // Sec CTP intra-bucket correlation is defined uniformly (MAR21.78),
    // so name and tenor factors are both equal to the prescribed constant
    // and the triple-rho reduces to a single-rho aggregation — but we
    // route through the same helper for consistency.
    csr_bucketed_delta(
        &sens.csr_sec_ctp_delta,
        csr::csr_sec_ctp_risk_weight,
        csr::CSR_SEC_CTP_INTRA_BUCKET_CORRELATION,
        1.0,
        csr::CSR_SEC_CTP_INTER_BUCKET_CORRELATION,
        scenario,
    )
}

fn csr_sec_nonctp_delta(sens: &FrtbSensitivities, scenario: CorrelationScenario) -> f64 {
    if sens.csr_sec_nonctp_delta.is_empty() {
        return 0.0;
    }
    csr_bucketed_delta(
        &sens.csr_sec_nonctp_delta,
        csr::csr_sec_nonctp_risk_weight,
        csr::CSR_SEC_NONCTP_INTRA_BUCKET_CORRELATION,
        1.0,
        csr::CSR_SEC_NONCTP_INTER_BUCKET_CORRELATION,
        scenario,
    )
}

// ---------------------------------------------------------------------------
// Equity delta
// ---------------------------------------------------------------------------

fn equity_delta(sens: &FrtbSensitivities, scenario: CorrelationScenario) -> f64 {
    if sens.equity_delta.is_empty() {
        return 0.0;
    }

    // Group by bucket.
    let mut by_bucket: HashMap<u8, Vec<f64>> = HashMap::default();
    for ((_, bucket), delta) in &sens.equity_delta {
        let rw = equity::equity_risk_weight(*bucket);
        let ws = delta * rw;
        by_bucket.entry(*bucket).or_default().push(ws);
    }

    let intra_rho = scenario.scale_correlation(equity::EQUITY_INTRA_BUCKET_CORRELATION);
    let inter_gamma = scenario.scale_correlation(equity::EQUITY_INTER_BUCKET_CORRELATION);

    let bucket_results = intra_bucket_aggregate_simple(&by_bucket, intra_rho);
    inter_bucket_aggregate(&bucket_results, inter_gamma)
}

// ---------------------------------------------------------------------------
// Commodity delta
// ---------------------------------------------------------------------------

fn commodity_delta(sens: &FrtbSensitivities, scenario: CorrelationScenario) -> f64 {
    if sens.commodity_delta.is_empty() {
        return 0.0;
    }

    let mut by_bucket: HashMap<u8, Vec<f64>> = HashMap::default();
    for ((_, bucket, _), delta) in &sens.commodity_delta {
        let rw = commodity::commodity_risk_weight(*bucket);
        let ws = delta * rw;
        by_bucket.entry(*bucket).or_default().push(ws);
    }

    let intra_rho = scenario.scale_correlation(commodity::COMMODITY_INTRA_BUCKET_CORRELATION);
    let inter_gamma = scenario.scale_correlation(commodity::COMMODITY_INTER_BUCKET_CORRELATION);

    let bucket_results = intra_bucket_aggregate_simple(&by_bucket, intra_rho);
    inter_bucket_aggregate(&bucket_results, inter_gamma)
}

// ---------------------------------------------------------------------------
// FX delta
// ---------------------------------------------------------------------------

fn fx_delta(sens: &FrtbSensitivities, scenario: CorrelationScenario) -> f64 {
    if sens.fx_delta.is_empty() {
        return 0.0;
    }

    // FX has a single bucket with uniform risk weight.
    let weighted: Vec<f64> = sens
        .fx_delta
        .values()
        .map(|d| d * fx::FX_DELTA_RISK_WEIGHT)
        .collect();

    let rho = scenario.scale_correlation(fx::FX_INTER_PAIR_CORRELATION);

    let mut sum = 0.0;
    for (i, ws_i) in weighted.iter().enumerate() {
        for (j, ws_j) in weighted.iter().enumerate() {
            let corr = if i == j { 1.0 } else { rho };
            sum += corr * ws_i * ws_j;
        }
    }
    sum.max(0.0).sqrt()
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// GIRR risk weight lookup by tenor label.
fn girr_risk_weight(tenor: &str) -> f64 {
    girr::GIRR_DELTA_RISK_WEIGHTS
        .iter()
        .find(|(t, _)| *t == tenor)
        .map(|(_, w)| *w)
        .unwrap_or(1.1) // Default to 1.1 for unknown tenors
}

/// Intra-bucket aggregation with uniform correlation.
///
/// Returns `(K_b, S_b_capped)` for each bucket.
fn intra_bucket_aggregate_simple(
    by_bucket: &HashMap<u8, Vec<f64>>,
    intra_rho: f64,
) -> Vec<(f64, f64)> {
    let mut results = Vec::new();
    for entries in by_bucket.values() {
        let mut k_squared = 0.0;
        for (i, ws_i) in entries.iter().enumerate() {
            for (j, ws_j) in entries.iter().enumerate() {
                let rho = if i == j { 1.0 } else { intra_rho };
                k_squared += rho * ws_i * ws_j;
            }
        }
        let k_b = k_squared.max(0.0).sqrt();
        let s_b: f64 = entries.iter().sum();
        let s_b_capped = s_b.max(-k_b).min(k_b);
        results.push((k_b, s_b_capped));
    }
    results
}

/// Inter-bucket aggregation.
///
/// `Delta = sqrt(max(sum_b K_b^2 + sum_{b != c} gamma * S_b * S_c, 0))`
fn inter_bucket_aggregate(bucket_results: &[(f64, f64)], gamma: f64) -> f64 {
    let mut total = 0.0;
    for (i, &(k_i, s_i)) in bucket_results.iter().enumerate() {
        total += k_i * k_i;
        for (j, &(_k_j, s_j)) in bucket_results.iter().enumerate() {
            if i != j {
                total += gamma * s_i * s_j;
            }
        }
    }
    total.max(0.0).sqrt()
}

/// CSR-specific delta aggregation with full intra-bucket `rho = rho_name * rho_tenor`.
///
/// Per MAR21.54 (non-sec) and equivalent sections for sec CTP / sec non-CTP,
/// the intra-bucket correlation between two weighted sensitivities within
/// the same bucket factorises:
///
/// ```text
/// rho_kl = rho_name(name_k, name_l)
///        * rho_tenor(tenor_k, tenor_l)
///        * rho_basis(basis_k, basis_l)
/// ```
///
/// where each component is 1 if the two dimensions match and the prescribed
/// correlation otherwise. Because the sensitivity map carries only
/// `(name, bucket, tenor)` and has no explicit basis dimension, this
/// implementation assumes a single basis per record and computes
/// `rho_name * rho_tenor` only — callers mixing bond vs CDS basis should
/// pre-apply the `rho_basis` factor (e.g. 0.999 for non-sec) to their
/// sensitivities before calling, or the capital number will be slightly
/// overstated (less offset than Basel allows).
fn csr_bucketed_delta(
    sensitivities: &HashMap<(String, u8, String), f64>,
    risk_weight_fn: impl Fn(u8) -> f64,
    name_correlation: f64,
    tenor_correlation: f64,
    inter_bucket_gamma: f64,
    scenario: CorrelationScenario,
) -> f64 {
    // Group weighted sensitivities by bucket, preserving name + tenor.
    type Entry = (String, String, f64); // (name, tenor, weighted_sensitivity)
    let mut by_bucket: HashMap<u8, Vec<Entry>> = HashMap::default();
    for ((name, bucket, tenor), delta) in sensitivities {
        let rw = risk_weight_fn(*bucket);
        let ws = delta * rw;
        by_bucket
            .entry(*bucket)
            .or_default()
            .push((name.clone(), tenor.clone(), ws));
    }

    let scaled_name = scenario.scale_correlation(name_correlation);
    let scaled_tenor = scenario.scale_correlation(tenor_correlation);
    let scaled_inter = scenario.scale_correlation(inter_bucket_gamma);

    // Intra-bucket aggregation with factorised rho.
    let mut bucket_results: Vec<(f64, f64)> = Vec::new();
    for entries in by_bucket.values() {
        let mut k_squared = 0.0;
        for (i, (name_i, tenor_i, ws_i)) in entries.iter().enumerate() {
            for (j, (name_j, tenor_j, ws_j)) in entries.iter().enumerate() {
                let rho = if i == j {
                    1.0
                } else {
                    let rn = if name_i == name_j { 1.0 } else { scaled_name };
                    let rt = if tenor_i == tenor_j {
                        1.0
                    } else {
                        scaled_tenor
                    };
                    rn * rt
                };
                k_squared += rho * ws_i * ws_j;
            }
        }
        let k_b = k_squared.max(0.0).sqrt();
        let s_b: f64 = entries.iter().map(|(_, _, ws)| *ws).sum();
        let s_b_capped = s_b.max(-k_b).min(k_b);
        bucket_results.push((k_b, s_b_capped));
    }

    inter_bucket_aggregate(&bucket_results, scaled_inter)
}
