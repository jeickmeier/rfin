//! Base correlation shock adapter.
//!
//! Contains helpers for both parallel and bucketed base correlation shocks,
//! used by the base-correlation `OperationSpec` variants.
//!
//! # Clamping Behavior
//!
//! Correlation values are clamped to the valid range [0, 1] after applying shocks.
//! When clamping occurs, warnings are returned to alert the caller that the
//! requested shock could not be fully applied.

use crate::adapters::traits::{ScenarioAdapter, ScenarioEffect};
use crate::engine::ExecutionContext;
use crate::error::Result;
use crate::spec::OperationSpec;
use finstack_core::market_data::bumps::{BumpMode, BumpSpec, BumpType, BumpUnits, MarketBump};
use finstack_core::market_data::term_structures::BASE_CORR_DETACHMENT_MATCH_TOLERANCE;

/// Adapter for base correlation operations.
pub struct BaseCorrAdapter;

fn detachment_matches(requested: f64, detachment: f64) -> bool {
    (requested - detachment).abs() <= BASE_CORR_DETACHMENT_MATCH_TOLERANCE
}

fn bump_warnings(
    ctx: &ExecutionContext,
    surface_id: &str,
    detachments: Option<&[f64]>,
    points: f64,
) -> Vec<String> {
    let Ok(curve) = ctx.market.get_base_correlation(surface_id) else {
        return Vec::new();
    };

    let matching: Vec<(f64, f64)> = curve
        .detachment_points()
        .iter()
        .copied()
        .zip(curve.correlations().iter().copied())
        .filter(|(detachment, _)| {
            detachments
                .map(|requested| {
                    requested
                        .iter()
                        .any(|target| detachment_matches(*target, *detachment))
                })
                .unwrap_or(true)
        })
        .collect();

    let mut warnings = Vec::new();
    if detachments.is_some_and(|requested| !requested.is_empty()) && matching.is_empty() {
        warnings.push(format!(
            "BaseCorrBucketPts on {surface_id} matched no detachment buckets"
        ));
    }

    warnings.extend(matching.into_iter().filter_map(|(detachment, corr)| {
        let bumped = corr + points;
        if (0.0..=1.0).contains(&bumped) {
            None
        } else {
            let clamped = bumped.clamp(0.0, 1.0);
            Some(format!(
                "BaseCorrBucketPts on {surface_id} detachment {detachment:.10} clamped \
                 correlation from requested {bumped:.10} to {clamped:.10}"
            ))
        }
    }));

    warnings
}

impl ScenarioAdapter for BaseCorrAdapter {
    fn try_generate_effects(
        &self,
        op: &OperationSpec,
        ctx: &ExecutionContext,
    ) -> Result<Option<Vec<ScenarioEffect>>> {
        match op {
            OperationSpec::BaseCorrParallelPts { surface_id, points } => {
                let bump = MarketBump::Curve {
                    id: finstack_core::types::CurveId::from(surface_id.as_str()),
                    spec: BumpSpec {
                        mode: BumpMode::Additive,
                        units: BumpUnits::Fraction,
                        value: *points,
                        bump_type: BumpType::Parallel,
                    },
                };
                let mut effects = vec![ScenarioEffect::MarketBump(bump)];
                effects.extend(
                    bump_warnings(ctx, surface_id, None, *points)
                        .into_iter()
                        .map(ScenarioEffect::Warning),
                );
                Ok(Some(effects))
            }
            OperationSpec::BaseCorrBucketPts {
                surface_id,
                detachment_bps,
                maturities,
                points,
            } => {
                let dets = detachment_bps
                    .as_ref()
                    .map(|v| crate::utils::bps_to_fractions(v));

                if let Some(mats) = maturities {
                    if !mats.is_empty() {
                        return Err(crate::error::Error::Validation(
                            "BaseCorrBucketPts maturity filtering is not supported: \
                             BaseCorrelationCurve is 1D (detachment only, no term structure). \
                             Remove the maturities field or use detachment_bps alone."
                                .to_string(),
                        ));
                    }
                }

                let bump = MarketBump::BaseCorrBucketPts {
                    surface_id: finstack_core::types::CurveId::from(surface_id.as_str()),
                    detachments: dets.clone(),
                    points: *points,
                };

                let mut effects = vec![ScenarioEffect::MarketBump(bump)];
                effects.extend(
                    bump_warnings(ctx, surface_id, dets.as_deref(), *points)
                        .into_iter()
                        .map(ScenarioEffect::Warning),
                );

                Ok(Some(effects))
            }
            _ => Ok(None),
        }
    }
}
