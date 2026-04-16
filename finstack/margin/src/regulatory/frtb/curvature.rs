//! FRTB curvature risk charge computation.
//!
//! For each risk factor k:
//!   CVR_k = max(CVR_up_k, CVR_down_k)
//!
//! Where CVR_up and CVR_down are provided as inputs (pre-computed from
//! the revaluation under curvature shocks minus the delta-hedged component).
//!
//! Intra-bucket and inter-bucket aggregation follow the same two-level
//! structure as delta/vega, but applied to curvature CVR values.

use super::params::{commodity, csr, equity, fx, girr};
use super::types::{CorrelationScenario, FrtbRiskClass, FrtbSensitivities};
use finstack_core::HashMap;

/// Compute the curvature risk charge for a single risk class.
pub fn curvature_charge(
    risk_class: FrtbRiskClass,
    sensitivities: &FrtbSensitivities,
    scenario: CorrelationScenario,
) -> f64 {
    match risk_class {
        FrtbRiskClass::Girr => girr_curvature(sensitivities, scenario),
        FrtbRiskClass::CsrNonSec => csr_nonsec_curvature(sensitivities, scenario),
        FrtbRiskClass::CsrSecCtp => csr_sec_ctp_curvature(sensitivities, scenario),
        FrtbRiskClass::CsrSecNonCtp => csr_sec_nonctp_curvature(sensitivities, scenario),
        FrtbRiskClass::Equity => equity_curvature(sensitivities, scenario),
        FrtbRiskClass::Commodity => commodity_curvature(sensitivities, scenario),
        FrtbRiskClass::Fx => fx_curvature(sensitivities, scenario),
    }
}

// ---------------------------------------------------------------------------
// GIRR curvature
// ---------------------------------------------------------------------------

fn girr_curvature(sens: &FrtbSensitivities, scenario: CorrelationScenario) -> f64 {
    if sens.girr_curvature.is_empty() {
        return 0.0;
    }

    // GIRR curvature: one risk factor per currency.
    // Intra-bucket is trivial (one factor per bucket = currency).
    let bucket_cvrs: Vec<f64> = sens
        .girr_curvature
        .values()
        .map(|(up, down)| {
            let cvr = f64::max(*up, *down);
            f64::max(cvr, 0.0)
        })
        .collect();

    let rho = scenario
        .scale_correlation(girr::GIRR_INTER_BUCKET_CORRELATION)
        .powi(2);
    curvature_inter_bucket(&bucket_cvrs, rho)
}

// ---------------------------------------------------------------------------
// CSR curvature
// ---------------------------------------------------------------------------

fn csr_nonsec_curvature(sens: &FrtbSensitivities, scenario: CorrelationScenario) -> f64 {
    generic_curvature_bucketed(
        &sens.csr_nonsec_curvature,
        csr::CSR_NONSEC_INTRA_BUCKET_NAME_CORRELATION,
        csr::CSR_NONSEC_INTER_BUCKET_CORRELATION,
        scenario,
    )
}

fn csr_sec_ctp_curvature(sens: &FrtbSensitivities, scenario: CorrelationScenario) -> f64 {
    generic_curvature_bucketed(
        &sens.csr_sec_ctp_curvature,
        csr::CSR_SEC_CTP_INTRA_BUCKET_CORRELATION,
        csr::CSR_SEC_CTP_INTER_BUCKET_CORRELATION,
        scenario,
    )
}

fn csr_sec_nonctp_curvature(sens: &FrtbSensitivities, scenario: CorrelationScenario) -> f64 {
    generic_curvature_bucketed(
        &sens.csr_sec_nonctp_curvature,
        csr::CSR_SEC_NONCTP_INTRA_BUCKET_CORRELATION,
        csr::CSR_SEC_NONCTP_INTER_BUCKET_CORRELATION,
        scenario,
    )
}

// ---------------------------------------------------------------------------
// Equity curvature
// ---------------------------------------------------------------------------

fn equity_curvature(sens: &FrtbSensitivities, scenario: CorrelationScenario) -> f64 {
    generic_curvature_bucketed(
        &sens.equity_curvature,
        equity::EQUITY_INTRA_BUCKET_CORRELATION,
        equity::EQUITY_INTER_BUCKET_CORRELATION,
        scenario,
    )
}

// ---------------------------------------------------------------------------
// Commodity curvature
// ---------------------------------------------------------------------------

fn commodity_curvature(sens: &FrtbSensitivities, scenario: CorrelationScenario) -> f64 {
    generic_curvature_bucketed(
        &sens.commodity_curvature,
        commodity::COMMODITY_INTRA_BUCKET_CORRELATION,
        commodity::COMMODITY_INTER_BUCKET_CORRELATION,
        scenario,
    )
}

// ---------------------------------------------------------------------------
// FX curvature
// ---------------------------------------------------------------------------

fn fx_curvature(sens: &FrtbSensitivities, scenario: CorrelationScenario) -> f64 {
    if sens.fx_curvature.is_empty() {
        return 0.0;
    }

    // FX: single bucket, aggregate all pairs.
    let cvrs: Vec<f64> = sens
        .fx_curvature
        .values()
        .map(|(up, down)| f64::max(f64::max(*up, *down), 0.0))
        .collect();

    let rho = scenario
        .scale_correlation(fx::FX_INTER_PAIR_CORRELATION)
        .powi(2);
    curvature_inter_bucket(&cvrs, rho)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Generic curvature aggregation for (name, bucket) keyed data.
fn generic_curvature_bucketed(
    curvatures: &HashMap<(String, u8), (f64, f64)>,
    intra_rho: f64,
    inter_gamma: f64,
    scenario: CorrelationScenario,
) -> f64 {
    if curvatures.is_empty() {
        return 0.0;
    }

    // Group by bucket.
    let mut by_bucket: HashMap<u8, Vec<f64>> = HashMap::default();
    for ((_, bucket), (up, down)) in curvatures {
        let cvr = f64::max(f64::max(*up, *down), 0.0);
        by_bucket.entry(*bucket).or_default().push(cvr);
    }

    // For curvature, correlations are squared.
    let intra_rho_sq = scenario.scale_correlation(intra_rho).powi(2);
    let inter_gamma_sq = scenario.scale_correlation(inter_gamma).powi(2);

    // Intra-bucket aggregation.
    let mut bucket_charges: Vec<f64> = Vec::new();
    for entries in by_bucket.values() {
        let mut k_squared = 0.0;
        for (i, cvr_i) in entries.iter().enumerate() {
            for (j, cvr_j) in entries.iter().enumerate() {
                let rho = if i == j { 1.0 } else { intra_rho_sq };
                k_squared += rho * cvr_i * cvr_j;
            }
        }
        bucket_charges.push(f64::max(k_squared, 0.0).sqrt());
    }

    curvature_inter_bucket(&bucket_charges, inter_gamma_sq)
}

/// Inter-bucket curvature aggregation.
fn curvature_inter_bucket(bucket_charges: &[f64], rho_sq: f64) -> f64 {
    let mut total = 0.0;
    for (i, &c_i) in bucket_charges.iter().enumerate() {
        for (j, &c_j) in bucket_charges.iter().enumerate() {
            let corr = if i == j { 1.0 } else { rho_sq };
            total += corr * c_i * c_j;
        }
    }
    f64::max(total, 0.0).sqrt()
}
