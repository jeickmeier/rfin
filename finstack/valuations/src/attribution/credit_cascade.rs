//! PR-8a: per-issuer credit-factor cascade for waterfall and parallel.
//!
//! Builds an ordered cascade `(generic / level_0 / ... / level_{L-1} / adder)`
//! of incremental synthetic spread shifts (in bp) that, applied per-issuer to
//! the instrument's hazard curves, decomposes the credit P&L into hierarchy
//! components.
//!
//! Single-instrument scope mirrors the linear PR-7 wire: the instrument's
//! issuer is read from `attributes().get_meta("credit::issuer_id")`, and the
//! per-issuer ΔS_i is synthesized by feeding `S_t0=0, S_t1=ΔS_i` to
//! `decompose_levels`, with `Δgeneric=0`.  Because `Σ_step (β·ΔF) + Δadder ≡
//! ΔS_i` by linearity of `decompose_period`, the cascade telescopes back to a
//! parallel ΔS_i shift on every credit curve.
//!
//! To preserve `credit_curves_pnl` *byte-identically* against the no-model
//! single-step in the presence of non-parallel hazard curve moves, the final
//! `Adder` step swaps the running hazard curves for the T1 hazard curves
//! wholesale (instead of merely bumping by `Δadder_i` bp). The step is still
//! labelled "Adder" — the bp-bump portion exactly equals `Δadder_i` for
//! parallel moves; for non-parallel moves the residual tenor structure is
//! absorbed into Adder, matching the PR-plan's intent that all credit
//! roll-down / curve-shape effects flow into the per-issuer adder.

use std::collections::BTreeMap;
use std::sync::Arc;

use finstack_core::dates::Date;
use finstack_core::factor_model::credit_hierarchy::{
    dimension_key, CreditFactorModel, HierarchyDimension, IssuerTags,
};
use finstack_core::factor_model::matching::ISSUER_ID_META_KEY;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::diff::{measure_hazard_curve_shift, TenorSamplingMethod};
use finstack_core::types::{CurveId, IssuerId};
use finstack_core::Result;

use crate::calibration::bumps::{bump_hazard_shift, BumpRequest};
use crate::factor_model::{decompose_levels, decompose_period};
use crate::instruments::common_impl::traits::Instrument;

/// What kind of cascade step a single bump represents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CreditStepKind {
    /// Generic / PC factor: bp = β_PC × ΔF_PC.
    Generic,
    /// Hierarchy level k: bp = β_level_k × ΔF_level_k(g_i^k).
    Level(usize),
    /// Per-issuer adder. The bump value is `Δadder_i` bp; the implementation
    /// also snaps the running hazard curves to T1 at this step so that
    /// `credit_curves_pnl` matches the no-model single-step value byte-identically.
    Adder,
}

/// One step in the credit cascade.
#[derive(Debug, Clone)]
pub(crate) struct CreditCascadeStep {
    /// Step kind.
    pub kind: CreditStepKind,
    /// Human-readable label, e.g. `"credit::generic"`, `"credit::rating"`,
    /// `"credit::adder"`.
    pub label: String,
    /// Per-issuer synthetic spread shift in basis points to apply at this step.
    /// For the `Adder` step this is the Δadder bp; the running market is also
    /// snapped to T1 hazard at the end of that step.
    pub delta_bp: f64,
}

/// Planned cascade for a single instrument's credit P&L.
#[derive(Debug, Clone)]
pub(crate) struct CreditCascade {
    /// Resolved issuer id (from instrument attributes).
    pub issuer_id: IssuerId,
    /// Hazard curve ids the instrument depends on.
    pub hazard_curve_ids: Vec<CurveId>,
    /// Ordered cascade steps: generic, then one per hierarchy level, then adder.
    pub steps: Vec<CreditCascadeStep>,
    /// Level names (one per hierarchy dimension), used to build
    /// `LevelPnl.level_name` and as parallel-factor labels.
    pub level_names: Vec<String>,
}

/// Plan a credit cascade for one instrument.
///
/// Returns `Ok(None)` when no cascade can be planned: instrument has no
/// `credit::issuer_id` attribute, the issuer is not in the model's
/// `issuer_betas`, the instrument has no hazard curve dependencies, or none of
/// the hazard curves can be measured (missing curves on either side).
pub(crate) fn plan_credit_cascade(
    model: &CreditFactorModel,
    instrument: &Arc<dyn Instrument>,
    market_t0: &MarketContext,
    market_t1: &MarketContext,
    as_of_t0: Date,
    as_of_t1: Date,
) -> Result<Option<CreditCascade>> {
    // Resolve issuer id from instrument attributes.
    let issuer_id_str = match instrument.attributes().get_meta(ISSUER_ID_META_KEY) {
        Some(s) => s.to_string(),
        None => return Ok(None),
    };
    let issuer_id = IssuerId::new(issuer_id_str);

    // Find issuer in model.
    let issuer_row = match model.issuer_betas.iter().find(|r| r.issuer_id == issuer_id) {
        Some(row) => row,
        None => return Ok(None),
    };
    let tags = issuer_row.tags.clone();

    // Resolve hazard curves from the instrument's market dependencies.
    let market_deps = instrument.market_dependencies()?;
    let credit_curves: Vec<CurveId> = market_deps.curve_dependencies().credit_curves.to_vec();
    if credit_curves.is_empty() {
        return Ok(None);
    }

    // Measure average ΔS_i across the issuer's hazard curves (in bp).
    let mut total_shift_bp = 0.0;
    let mut count = 0usize;
    for curve_id in &credit_curves {
        if let Ok(shift) = measure_hazard_curve_shift(
            curve_id.as_str(),
            market_t0,
            market_t1,
            TenorSamplingMethod::Standard,
        ) {
            total_shift_bp += shift;
            count += 1;
        }
    }
    if count == 0 {
        return Ok(None);
    }
    let ds_i = total_shift_bp / count as f64;

    // Synthesize a single-issuer period decomposition: feed S_t0=0, S_t1=ΔS_i,
    // generic=0 to mirror the PR-7 linear wire. Δgeneric will be 0; the level-0
    // bucket carries ΔS_i; remaining levels and the adder are zero (modulo
    // calibrated betas).
    let mut s_t0: BTreeMap<IssuerId, f64> = BTreeMap::new();
    let mut s_t1: BTreeMap<IssuerId, f64> = BTreeMap::new();
    s_t0.insert(issuer_id.clone(), 0.0);
    s_t1.insert(issuer_id.clone(), ds_i);

    let mut runtime_tags: BTreeMap<IssuerId, IssuerTags> = BTreeMap::new();
    runtime_tags.insert(issuer_id.clone(), tags);

    let from = decompose_levels(model, &s_t0, 0.0, as_of_t0, Some(&runtime_tags))
        .map_err(|e| finstack_core::Error::Validation(format!("decompose_levels(t0): {e}")))?;
    let to = decompose_levels(model, &s_t1, 0.0, as_of_t1, Some(&runtime_tags))
        .map_err(|e| finstack_core::Error::Validation(format!("decompose_levels(t1): {e}")))?;
    let period = decompose_period(&from, &to)
        .map_err(|e| finstack_core::Error::Validation(format!("decompose_period: {e}")))?;

    // Build cascade steps.
    let beta_pc = issuer_row.betas.pc;
    let generic_bp = beta_pc * period.d_generic;

    let mut level_names: Vec<String> = Vec::with_capacity(model.hierarchy.levels.len());
    let mut steps: Vec<CreditCascadeStep> = Vec::with_capacity(model.hierarchy.levels.len() + 2);

    steps.push(CreditCascadeStep {
        kind: CreditStepKind::Generic,
        label: "credit::generic".to_string(),
        delta_bp: generic_bp,
    });

    for (k, dim) in model.hierarchy.levels.iter().enumerate() {
        let level_name = match dim {
            HierarchyDimension::Custom(s) => s.clone(),
            _ => dimension_key(dim),
        };
        let bucket = model
            .hierarchy
            .bucket_path(&issuer_row.tags, k)
            .unwrap_or_default();
        let d_level = period.by_level[k]
            .deltas
            .get(&bucket)
            .copied()
            .unwrap_or(0.0);
        let beta_k = issuer_row.betas.levels.get(k).copied().unwrap_or(0.0);
        let level_bp = beta_k * d_level;
        steps.push(CreditCascadeStep {
            kind: CreditStepKind::Level(k),
            label: format!("credit::{}", level_name),
            delta_bp: level_bp,
        });
        level_names.push(level_name);
    }

    let adder_bp = period.d_adder.get(&issuer_id).copied().unwrap_or(0.0);
    steps.push(CreditCascadeStep {
        kind: CreditStepKind::Adder,
        label: "credit::adder".to_string(),
        delta_bp: adder_bp,
    });

    Ok(Some(CreditCascade {
        issuer_id,
        hazard_curve_ids: credit_curves,
        steps,
        level_names,
    }))
}

/// Apply an additive parallel bp-shift to every hazard curve in `curve_ids`
/// from `base_market`, returning a new MarketContext with the shifted curves.
/// Non-hazard families on `base_market` are preserved.
pub(crate) fn shift_hazard_curves(
    base_market: &MarketContext,
    curve_ids: &[CurveId],
    delta_bp: f64,
) -> Result<MarketContext> {
    // For each curve, take its current state from base_market, bump in-place,
    // and re-insert into a clone of base_market.
    let mut new_market = base_market.clone();
    if delta_bp == 0.0 {
        return Ok(new_market);
    }
    for curve_id in curve_ids {
        let cur = match base_market.get_hazard(curve_id.as_str()) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let bumped = bump_hazard_shift(cur.as_ref(), &BumpRequest::Parallel(delta_bp))?;
        new_market = new_market.insert(bumped);
    }
    Ok(new_market)
}

/// Replace the running market's hazard curves (for `curve_ids`) with the T1
/// hazard curves from `market_t1`. Used at the Adder step so end-state matches
/// the no-model single-Credit-step result.
pub(crate) fn snap_hazard_to_t1(
    base_market: &MarketContext,
    market_t1: &MarketContext,
    curve_ids: &[CurveId],
) -> MarketContext {
    let mut new_market = base_market.clone();
    for curve_id in curve_ids {
        if let Ok(curve_t1) = market_t1.get_hazard(curve_id.as_str()) {
            new_market = new_market.insert((*curve_t1).clone());
        }
    }
    new_market
}

/// Build a `CreditFactorAttribution` from per-step P&L amounts captured during
/// the cascade. `step_pnls` must align with `cascade.steps`.
pub(crate) fn build_credit_factor_attribution(
    model: &CreditFactorModel,
    cascade: &CreditCascade,
    options: &super::credit_factor::CreditFactorDetailOptions,
    step_pnls: &[finstack_core::money::Money],
) -> super::types::CreditFactorAttribution {
    use super::credit_factor::credit_factor_model_id;
    use super::types::{CreditFactorAttribution, LevelPnl};

    debug_assert_eq!(step_pnls.len(), cascade.steps.len());
    let ccy = step_pnls
        .first()
        .map(|m| m.currency())
        .unwrap_or(finstack_core::currency::Currency::USD);

    // Resolve issuer's bucket path for per-bucket detail (single-instrument
    // scope: each level has at most one populated bucket).
    let issuer_row = model
        .issuer_betas
        .iter()
        .find(|r| r.issuer_id == cascade.issuer_id);

    let mut generic_pnl = finstack_core::money::Money::new(0.0, ccy);
    let mut adder_pnl = finstack_core::money::Money::new(0.0, ccy);
    let mut level_pnls: BTreeMap<usize, finstack_core::money::Money> = BTreeMap::new();

    for (step, pnl) in cascade.steps.iter().zip(step_pnls.iter()) {
        match step.kind {
            CreditStepKind::Generic => generic_pnl = *pnl,
            CreditStepKind::Adder => adder_pnl = *pnl,
            CreditStepKind::Level(k) => {
                level_pnls.insert(k, *pnl);
            }
        }
    }

    let mut levels: Vec<LevelPnl> = Vec::with_capacity(cascade.level_names.len());
    for (k, level_name) in cascade.level_names.iter().enumerate() {
        let total = level_pnls
            .get(&k)
            .copied()
            .unwrap_or_else(|| finstack_core::money::Money::new(0.0, ccy));
        let mut by_bucket: BTreeMap<String, finstack_core::money::Money> = BTreeMap::new();
        if options.include_per_bucket_breakdown {
            if let Some(row) = issuer_row {
                if let Some(bucket) = model.hierarchy.bucket_path(&row.tags, k) {
                    by_bucket.insert(bucket, total);
                }
            }
        }
        levels.push(LevelPnl {
            level_name: level_name.clone(),
            total,
            by_bucket,
        });
    }

    let adder_pnl_by_issuer = if options.include_per_issuer_adder {
        let mut m = BTreeMap::new();
        m.insert(cascade.issuer_id.clone(), adder_pnl);
        Some(m)
    } else {
        None
    };

    CreditFactorAttribution {
        model_id: credit_factor_model_id(model),
        generic_pnl,
        levels,
        adder_pnl_total: adder_pnl,
        adder_pnl_by_issuer,
    }
}
