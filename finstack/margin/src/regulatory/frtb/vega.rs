//! FRTB vega risk charge computation.
//!
//! Vega sensitivities are volatility-weighted, then aggregated using
//! the same two-level (intra-bucket, inter-bucket) formula as delta,
//! but with vega-specific risk weights and correlations.

use super::params::{commodity, csr, equity, fx, girr};
use super::types::{CorrelationScenario, FrtbRiskClass, FrtbSensitivities};
use finstack_core::HashMap;

/// Compute the vega risk charge for a single risk class under one
/// correlation scenario.
pub fn vega_charge(
    risk_class: FrtbRiskClass,
    sensitivities: &FrtbSensitivities,
    scenario: CorrelationScenario,
) -> f64 {
    match risk_class {
        FrtbRiskClass::Girr => girr_vega(sensitivities, scenario),
        FrtbRiskClass::CsrNonSec => csr_nonsec_vega(sensitivities, scenario),
        FrtbRiskClass::CsrSecCtp => csr_sec_ctp_vega(sensitivities, scenario),
        FrtbRiskClass::CsrSecNonCtp => csr_sec_nonctp_vega(sensitivities, scenario),
        FrtbRiskClass::Equity => equity_vega(sensitivities, scenario),
        FrtbRiskClass::Commodity => commodity_vega(sensitivities, scenario),
        FrtbRiskClass::Fx => fx_vega(sensitivities, scenario),
    }
}

// ---------------------------------------------------------------------------
// GIRR vega
// ---------------------------------------------------------------------------

fn girr_vega(sens: &FrtbSensitivities, scenario: CorrelationScenario) -> f64 {
    if sens.girr_vega.is_empty() {
        return 0.0;
    }

    // Group by currency bucket.
    let mut by_currency: HashMap<_, Vec<f64>> = HashMap::default();
    for ((ccy, _, _), vega) in &sens.girr_vega {
        let ws = vega * girr::GIRR_VEGA_RISK_WEIGHT;
        by_currency.entry(*ccy).or_default().push(ws);
    }

    // Intra-bucket: use a uniform vega correlation.
    let intra_rho = scenario.scale_correlation(0.96);
    let inter_gamma = scenario.scale_correlation(girr::GIRR_INTER_BUCKET_CORRELATION);

    let bucket_results = intra_bucket_aggregate(&by_currency, intra_rho);
    inter_bucket_agg(&bucket_results, inter_gamma)
}

// ---------------------------------------------------------------------------
// CSR vega (non-sec, sec CTP, sec non-CTP)
// ---------------------------------------------------------------------------

fn csr_nonsec_vega(sens: &FrtbSensitivities, scenario: CorrelationScenario) -> f64 {
    generic_bucketed_vega(
        &sens.csr_nonsec_vega,
        csr::CSR_NONSEC_VEGA_RISK_WEIGHT,
        csr::CSR_NONSEC_INTRA_BUCKET_NAME_CORRELATION,
        csr::CSR_NONSEC_INTER_BUCKET_CORRELATION,
        scenario,
    )
}

fn csr_sec_ctp_vega(sens: &FrtbSensitivities, scenario: CorrelationScenario) -> f64 {
    generic_bucketed_vega(
        &sens.csr_sec_ctp_vega,
        csr::CSR_NONSEC_VEGA_RISK_WEIGHT, // Same vega weight for sec CTP.
        csr::CSR_SEC_CTP_INTRA_BUCKET_CORRELATION,
        csr::CSR_SEC_CTP_INTER_BUCKET_CORRELATION,
        scenario,
    )
}

fn csr_sec_nonctp_vega(sens: &FrtbSensitivities, scenario: CorrelationScenario) -> f64 {
    generic_bucketed_vega(
        &sens.csr_sec_nonctp_vega,
        csr::CSR_NONSEC_VEGA_RISK_WEIGHT,
        csr::CSR_SEC_NONCTP_INTRA_BUCKET_CORRELATION,
        csr::CSR_SEC_NONCTP_INTER_BUCKET_CORRELATION,
        scenario,
    )
}

// ---------------------------------------------------------------------------
// Equity vega
// ---------------------------------------------------------------------------

fn equity_vega(sens: &FrtbSensitivities, scenario: CorrelationScenario) -> f64 {
    if sens.equity_vega.is_empty() {
        return 0.0;
    }

    let mut by_bucket: HashMap<u8, Vec<f64>> = HashMap::default();
    for ((_, bucket, _), vega) in &sens.equity_vega {
        let ws = vega * equity::EQUITY_VEGA_RISK_WEIGHT;
        by_bucket.entry(*bucket).or_default().push(ws);
    }

    let intra_rho = scenario.scale_correlation(equity::EQUITY_INTRA_BUCKET_CORRELATION);
    let inter_gamma = scenario.scale_correlation(equity::EQUITY_INTER_BUCKET_CORRELATION);

    let bucket_results = intra_bucket_aggregate_u8(&by_bucket, intra_rho);
    inter_bucket_agg(&bucket_results, inter_gamma)
}

// ---------------------------------------------------------------------------
// Commodity vega
// ---------------------------------------------------------------------------

fn commodity_vega(sens: &FrtbSensitivities, scenario: CorrelationScenario) -> f64 {
    if sens.commodity_vega.is_empty() {
        return 0.0;
    }

    let mut by_bucket: HashMap<u8, Vec<f64>> = HashMap::default();
    for ((_, bucket, _), vega) in &sens.commodity_vega {
        let ws = vega * commodity::COMMODITY_VEGA_RISK_WEIGHT;
        by_bucket.entry(*bucket).or_default().push(ws);
    }

    let intra_rho = scenario.scale_correlation(commodity::COMMODITY_INTRA_BUCKET_CORRELATION);
    let inter_gamma = scenario.scale_correlation(commodity::COMMODITY_INTER_BUCKET_CORRELATION);

    let bucket_results = intra_bucket_aggregate_u8(&by_bucket, intra_rho);
    inter_bucket_agg(&bucket_results, inter_gamma)
}

// ---------------------------------------------------------------------------
// FX vega
// ---------------------------------------------------------------------------

fn fx_vega(sens: &FrtbSensitivities, scenario: CorrelationScenario) -> f64 {
    if sens.fx_vega.is_empty() {
        return 0.0;
    }

    let weighted: Vec<f64> = sens
        .fx_vega
        .values()
        .map(|v| v * fx::FX_VEGA_RISK_WEIGHT)
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

/// Generic bucketed vega aggregation for (name, bucket, tenor) keys.
fn generic_bucketed_vega(
    sensitivities: &HashMap<(String, u8, String), f64>,
    vega_rw: f64,
    intra_rho: f64,
    inter_gamma: f64,
    scenario: CorrelationScenario,
) -> f64 {
    if sensitivities.is_empty() {
        return 0.0;
    }

    let mut by_bucket: HashMap<u8, Vec<f64>> = HashMap::default();
    for ((_, bucket, _), vega) in sensitivities {
        let ws = vega * vega_rw;
        by_bucket.entry(*bucket).or_default().push(ws);
    }

    let scaled_intra = scenario.scale_correlation(intra_rho);
    let scaled_inter = scenario.scale_correlation(inter_gamma);

    let bucket_results = intra_bucket_aggregate_u8(&by_bucket, scaled_intra);
    inter_bucket_agg(&bucket_results, scaled_inter)
}

/// Intra-bucket aggregation for currency-keyed data.
fn intra_bucket_aggregate<K: Eq + std::hash::Hash>(
    by_bucket: &HashMap<K, Vec<f64>>,
    intra_rho: f64,
) -> Vec<(f64, f64)> {
    let mut results = Vec::new();
    for entries in by_bucket.values() {
        let (k_b, s_b) = aggregate_within_bucket(entries, intra_rho);
        results.push((k_b, s_b));
    }
    results
}

/// Intra-bucket aggregation for u8-keyed data.
fn intra_bucket_aggregate_u8(
    by_bucket: &HashMap<u8, Vec<f64>>,
    intra_rho: f64,
) -> Vec<(f64, f64)> {
    let mut results = Vec::new();
    for entries in by_bucket.values() {
        let (k_b, s_b) = aggregate_within_bucket(entries, intra_rho);
        results.push((k_b, s_b));
    }
    results
}

fn aggregate_within_bucket(entries: &[f64], intra_rho: f64) -> (f64, f64) {
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
    (k_b, s_b_capped)
}

/// Inter-bucket aggregation.
fn inter_bucket_agg(bucket_results: &[(f64, f64)], gamma: f64) -> f64 {
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
