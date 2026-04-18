//! FRTB vega risk charge computation.
//!
//! Vega sensitivities are volatility-weighted, then aggregated using
//! the same two-level (intra-bucket, inter-bucket) formula as delta,
//! but with vega-specific risk weights and correlations.

use super::aggregation::{inter_bucket, intra_bucket_uniform_map};
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

    // Group by currency bucket, carrying option maturity and underlying
    // tenor so intra-bucket correlation can reflect both dimensions per
    // MAR21.89.
    // Entry: (ws, option_maturity_years, underlying_tenor_years)
    type VegaEntry = (f64, f64, f64);
    let mut by_currency: HashMap<_, Vec<VegaEntry>> = HashMap::default();
    for ((ccy, opt_mat, und_tenor), vega) in &sens.girr_vega {
        let ws = vega * girr::GIRR_VEGA_RISK_WEIGHT;
        // Default to 5Y if the label is unrecognised — matches the GIRR
        // delta fallback and is dominated by the exp-decay elsewhere.
        let t_opt = girr::tenor_to_years(opt_mat).unwrap_or(5.0);
        let t_und = girr::tenor_to_years(und_tenor).unwrap_or(5.0);
        by_currency
            .entry(*ccy)
            .or_default()
            .push((ws, t_opt, t_und));
    }

    let inter_gamma = scenario.scale_correlation(girr::GIRR_INTER_BUCKET_CORRELATION);

    // Intra-bucket aggregation with MAR21.89 correlation
    //   rho = min(rho_opt_mat * rho_under_mat, 1)
    //   rho_opt_mat   = exp(-alpha * |T_k - T_l| / min(T_k, T_l)), alpha=0.01
    //   rho_under_mat = exp(-alpha * |U_k - U_l| / min(U_k, U_l)), alpha=0.03
    // (option-maturity alpha uses the standard Basel value; underlying-
    // tenor alpha reuses the GIRR delta tenor formula.)
    let mut bucket_results: Vec<(f64, f64)> = Vec::new();
    for entries in by_currency.values() {
        let mut k_squared = 0.0;
        for (i, &(ws_i, t_opt_i, t_und_i)) in entries.iter().enumerate() {
            for (j, &(ws_j, t_opt_j, t_und_j)) in entries.iter().enumerate() {
                let base_rho = if i == j {
                    1.0
                } else {
                    let rho_opt = exp_decay_rho(t_opt_i, t_opt_j, 0.01);
                    let rho_und = girr::girr_tenor_correlation(t_und_i, t_und_j);
                    (rho_opt * rho_und).min(1.0)
                };
                let rho = scenario.scale_correlation(base_rho);
                k_squared += rho * ws_i * ws_j;
            }
        }
        let k_b = k_squared.max(0.0).sqrt();
        let s_b: f64 = entries.iter().map(|(ws, _, _)| ws).sum();
        bucket_results.push((k_b, s_b));
    }

    inter_bucket(&bucket_results, inter_gamma)
}

/// Exponential-decay correlation between two tenors / maturities.
///
/// `rho = exp(-alpha * |T_i - T_j| / min(T_i, T_j))` — the canonical
/// Basel tenor correlation form used across GIRR. `min(T_i, T_j)` is
/// floored at a small positive value to avoid division by zero for
/// zero-tenor cases.
fn exp_decay_rho(t_i: f64, t_j: f64, alpha: f64) -> f64 {
    let min_t = t_i.min(t_j).max(1.0 / 365.0);
    (-alpha * (t_i - t_j).abs() / min_t).exp()
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
        csr::CSR_SEC_CTP_VEGA_RISK_WEIGHT,
        csr::CSR_SEC_CTP_INTRA_BUCKET_CORRELATION,
        csr::CSR_SEC_CTP_INTER_BUCKET_CORRELATION,
        scenario,
    )
}

fn csr_sec_nonctp_vega(sens: &FrtbSensitivities, scenario: CorrelationScenario) -> f64 {
    generic_bucketed_vega(
        &sens.csr_sec_nonctp_vega,
        csr::CSR_SEC_NONCTP_VEGA_RISK_WEIGHT,
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

    let bucket_results = intra_bucket_uniform_map(&by_bucket, intra_rho);
    inter_bucket(&bucket_results, inter_gamma)
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

    let bucket_results = intra_bucket_uniform_map(&by_bucket, intra_rho);
    inter_bucket(&bucket_results, inter_gamma)
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

    let bucket_results = intra_bucket_uniform_map(&by_bucket, scaled_intra);
    inter_bucket(&bucket_results, scaled_inter)
}
