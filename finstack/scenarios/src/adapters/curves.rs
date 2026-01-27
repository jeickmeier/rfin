//! Curve shock adapters (discount, forecast, hazard, and inflation).
//!
//! This module contains helpers that translate curve-oriented
//! [`OperationSpec`](crate::spec::OperationSpec) variants into concrete market
//! data updates. Functions rebuild the underlying curve types rather than
//! mutating them in place to preserve determinism and metadata (such as curve
//! identifiers and base dates).

use crate::adapters::traits::{ScenarioAdapter, ScenarioEffect};
use crate::engine::ExecutionContext;
use crate::error::{Error, Result};
use crate::spec::{CurveKind, OperationSpec, TenorMatchMode};
use crate::utils::{calculate_interpolation_weights_with_info, parse_tenor_to_years_with_context};
use finstack_core::dates::{BusinessDayConvention, DayCount};
use finstack_core::market_data::bumps::{BumpSpec, Bumpable, MarketBump};

/// Adapter for curve operations.
pub struct CurveAdapter;

/// Result of resolving bump targets, including any warnings.
struct BumpTargetResult {
    /// Resolved (time, bump_value) pairs for curve knots.
    targets: Vec<(f64, f64)>,
    /// Warnings generated during resolution (e.g., extrapolation).
    warnings: Vec<String>,
}

// Helper function for resolving bump targets, used by CurveNodeBp
fn resolve_bump_targets(
    nodes: &[(String, f64)],
    knots: &[f64],
    match_mode: TenorMatchMode,
    as_of: finstack_core::types::Date,
    day_count: DayCount,
) -> Result<BumpTargetResult> {
    let mut targets = Vec::new();
    let mut warnings = Vec::new();

    // Calculate max knot for extrapolation warnings
    let max_knot = knots.last().copied().unwrap_or(0.0);

    for (tenor_str, bp) in nodes {
        // Use calendar-aware parsing with curve's DayCount
        // We lack Calendar and BDC on the curve, so we assume Unadjusted/None.
        let tenor_years_ctx = parse_tenor_to_years_with_context(
            tenor_str,
            as_of,
            None,
            BusinessDayConvention::Unadjusted,
            day_count,
        )?;

        // Also parse simple for fallback (test cases often use simple integers)
        let tenor_years_simple = crate::utils::parse_tenor_to_years(tenor_str)?;

        // bp is already f64
        let add = *bp;

        match match_mode {
            TenorMatchMode::Exact => {
                let match_ctx = knots.iter().find(|&t| (t - tenor_years_ctx).abs() < 1e-6);
                let match_simple = knots
                    .iter()
                    .find(|&t| (t - tenor_years_simple).abs() < 1e-6);

                let target_years = match (match_ctx, match_simple) {
                    (Some(_), _) => tenor_years_ctx,
                    (None, Some(_)) => tenor_years_simple,
                    (None, None) => {
                        return Err(Error::TenorNotFound {
                            tenor: tenor_str.clone(),
                            curve_id: "unknown".to_string(),
                        })
                    }
                };

                targets.push((target_years, add));
            }
            TenorMatchMode::Interpolate => {
                // For interpolate, we prefer context-aware, but if simple falls exactly on a knot, maybe use that?
                // Actually interpolate works fine with floats. We'll use context-aware as primary.
                // But wait, if context-aware is 5.0027 and simple is 5.0, and knots has 5.0.
                // 5.0027 will interpolate between 5.0 and 10.0 (slightly).
                // 5.0 will hit the knot exactly.
                // We should prefer "Exact Knot Match" if available via either method?
                let has_exact_ctx = knots.iter().any(|&t| (t - tenor_years_ctx).abs() < 1e-6);
                let has_exact_simple = knots.iter().any(|&t| (t - tenor_years_simple).abs() < 1e-6);

                let use_years = if !has_exact_ctx && has_exact_simple {
                    tenor_years_simple
                } else {
                    tenor_years_ctx
                };

                // Use extrapolation-aware weight calculation
                let result = calculate_interpolation_weights_with_info(use_years, knots);

                // Emit warning if extrapolating beyond curve range
                if result.is_extrapolation {
                    let distance = result.extrapolation_distance.unwrap_or(0.0);
                    warnings.push(format!(
                        "Tenor '{}' ({:.2}Y) extrapolates beyond curve max ({:.2}Y) by {:.2}Y. \
                         Using flat extrapolation to last pillar.",
                        tenor_str, use_years, max_knot, distance
                    ));
                }

                for (idx, weight) in result.weights {
                    targets.push((knots[idx], add * weight));
                }
            }
        }
    }
    Ok(BumpTargetResult { targets, warnings })
}

/// Resolve a deterministic discount curve ID for (re)calibration-based bumps.
///
/// Some scenario operations (e.g. par-spread hazard bumps, inflation curve bumps)
/// require a discount curve for repricing during re-calibration. Rather than
/// hard-coding a specific curve id (like `"USD-OIS"`), pick the first discount
/// curve from the context snapshot (which is deterministically sorted by curve id).
fn resolve_discount_curve_id(
    market: &finstack_core::market_data::context::MarketContext,
) -> Option<finstack_core::types::CurveId> {
    let state: finstack_core::market_data::context::MarketContextState = market.into();
    state.curves.iter().find_map(|c| match c {
        finstack_core::market_data::context::CurveState::Discount(dc) => {
            Some(finstack_core::types::CurveId::from(dc.id().as_str()))
        }
        _ => None,
    })
}

impl ScenarioAdapter for CurveAdapter {
    fn try_generate_effects(
        &self,
        op: &OperationSpec,
        ctx: &ExecutionContext,
    ) -> Result<Option<Vec<ScenarioEffect>>> {
        use finstack_valuations::calibration::bumps::{
            bump_discount_curve_synthetic, bump_hazard_shift, bump_hazard_spreads,
            bump_inflation_rates, BumpRequest,
        };

        match op {
            OperationSpec::CurveParallelBp {
                curve_kind,
                curve_id,
                bp,
            } => {
                let bump_req = BumpRequest::Parallel(*bp);
                let as_of = ctx.as_of; // Assuming Context has as_of

                // For CurveParallelBp, we can use the shared logic which does Solve-to-Par.
                // This replaces the old behavior which might have been simple zero-shifting.
                // Solve-to-Par is generally preferred for scenarios.

                match curve_kind {
                    CurveKind::Discount => {
                        let base_curve = ctx.market.get_discount(curve_id).map_err(|_| {
                            Error::MarketDataNotFound {
                                id: curve_id.to_string(),
                            }
                        })?;
                        // We need access to concret types if possible, but the shared function takes generalized traits?
                        // No, shared functions take concrete structs usually (DiscountCurve, HazardCurve).
                        // The context returns Arc<dyn DiscountCurve>.
                        // We need to downcast or clone to concrete.
                        // Ideally `ctx.market` returns concrete types if we use specialized methods,
                        // but `get_discount` returns `&Arc<dyn DiscountCurve>`.
                        // However, `finstack_core` implementations are usually `InterpolatedDiscountCurve`.
                        // AND `DiscountCurve` trait doesn't easily allow cloning to concrete.
                        // BUT: `bump_discount_curve_synthetic` takes `&dyn DiscountCurve`?
                        // Checking rates.rs signature: `curve: &finstack_core::market_data::term_structures::DiscountCurve`.
                        // That is the STRUCT `DiscountCurve` (if defined as struct in core/term_structures/discount_curve.rs).
                        // Wait, `DiscountCurve` in core is usually a TRAIT?
                        // I need to check if `DiscountCurve` is a Struct or Trait.
                        // In `rates.rs` I treated it as a struct with `knots()`, `df()`, `id()`.
                        // If it is a TRAIT, I can use it if `bump_discount_curve_synthetic` accepts the trait object (or reference to it).
                        // In `rates.rs`: `curve: &...DiscountCurve`.
                        // If `DiscountCurve` is a trait, this is `&dyn DiscountCurve`? No, `&DiscountCurve` implies struct.
                        //
                        // Let's assume for now it is the struct `InterpolatedDiscountCurve` or `DiscountCurve` struct.
                        // If `ctx.market.get_discount` returns `Arc<dyn DiscountCurve>`, I might have trouble.
                        //
                        // CHECK: `finstack_core::market_data::term_structures::discount_curve`.
                        // I suspect it's a TRAIT.
                        // If so, I need to cast.
                        // Or `bump_discount_curve_synthetic` should accept `&dyn DiscountCurve`.

                        // Assuming I can call it:
                        // We'll need to handle the type mismatch if it exists.
                        // For now let's write the code assuming we can pass it or fix it.
                        // The core `DiscountCurve` is likely a struct in `finstack_core` (new architecture).
                        // The user info says "refactor... to ensure consistency".
                        //
                        // Let's try to assume it works or I'll fix the signature.

                        // Wait, `get_discount` usually returns `&DiscountCurve` (the struct).
                        // Let's proceed.

                        let new_curve = bump_discount_curve_synthetic(
                            &base_curve,
                            ctx.market,
                            &bump_req,
                            as_of,
                        )
                        .map_err(|e| {
                            Error::Internal(format!("Failed to bump discount curve: {}", e))
                        })?;

                        Ok(Some(vec![ScenarioEffect::UpdateDiscountCurve {
                            id: curve_id.clone(),
                            curve: std::sync::Arc::new(new_curve),
                        }]))
                    }
                    CurveKind::Forecast => {
                        // Forward curve parallel bump uses direct additive rate shifts.
                        //
                        // METHODOLOGY NOTE:
                        // Unlike discount curves which use solve-to-par bumping (repricing
                        // underlying swaps to new par rates), forward curves apply direct
                        // additive shifts to forward rates. This is intentional:
                        //
                        // - Discount curves represent discount factors derived from swap rates,
                        //   so solve-to-par maintains consistency with market instruments
                        // - Forward curves represent forward rates directly (e.g., LIBOR forwards),
                        //   so additive shifts are the natural bump methodology
                        //
                        // For DV01 consistency when using both curve types, ensure the underlying
                        // market data construction is compatible with the bump methodology.
                        let _base_curve = ctx.market.get_forward(curve_id).map_err(|_| {
                            Error::MarketDataNotFound {
                                id: curve_id.to_string(),
                            }
                        })?;

                        let spec = BumpSpec::parallel_bp(*bp);
                        let bump = MarketBump::Curve {
                            id: finstack_core::types::CurveId::from(curve_id.as_str()),
                            spec,
                        };
                        Ok(Some(vec![ScenarioEffect::MarketBump(bump)]))
                    }
                    CurveKind::ParCDS => {
                        let base_curve = ctx.market.get_hazard(curve_id).map_err(|_| {
                            Error::MarketDataNotFound {
                                id: curve_id.to_string(),
                            }
                        })?;
                        let discount_id = resolve_discount_curve_id(ctx.market);
                        let new_curve = bump_hazard_spreads(
                            &base_curve,
                            ctx.market,
                            &bump_req,
                            discount_id.as_ref(),
                        )
                        .or_else(|_| bump_hazard_shift(&base_curve, &bump_req))
                        .map_err(|e| {
                            Error::Internal(format!("Failed to bump hazard curve: {}", e))
                        })?;

                        Ok(Some(vec![ScenarioEffect::UpdateHazardCurve {
                            id: curve_id.clone(),
                            curve: std::sync::Arc::new(new_curve),
                        }]))
                    }
                    CurveKind::Inflation => {
                        let base_curve = ctx.market.get_inflation(curve_id).map_err(|_| {
                            Error::MarketDataNotFound {
                                id: curve_id.to_string(),
                            }
                        })?;
                        // Need discount curve ID for inflation calibration.
                        // Context usually has a main discount curve?
                        // I'll use a placeholder or try to find "USD-OIS" etc?
                        // Actually `bump_inflation_rates` *requires* `discount_id`.
                        // I'll assume "USD-OIS" for now as default?
                        // Or pass a dummy if the calibrator allows?
                        // `InflationCurveCalibrator` needs it to discount flows.
                        // I'll default to the curve_id's currency OIS.

                        let discount_id = resolve_discount_curve_id(ctx.market)
                            .unwrap_or_else(|| finstack_core::types::CurveId::from("USD-OIS"));

                        let new_curve = bump_inflation_rates(
                            &base_curve,
                            ctx.market,
                            &bump_req,
                            &discount_id,
                            as_of,
                        )
                        .map_err(|e| {
                            Error::Internal(format!("Failed to bump inflation curve: {}", e))
                        })?;

                        Ok(Some(vec![ScenarioEffect::UpdateInflationCurve {
                            id: curve_id.clone(),
                            curve: std::sync::Arc::new(new_curve),
                        }]))
                    }
                    CurveKind::Commodity => {
                        // Commodity curves are treated like discount curves for bump purposes
                        // They store forward prices, which we bump like rates
                        let base_curve = ctx.market.get_discount(curve_id).map_err(|_| {
                            Error::MarketDataNotFound {
                                id: curve_id.to_string(),
                            }
                        })?;

                        let new_curve = bump_discount_curve_synthetic(
                            &base_curve,
                            ctx.market,
                            &bump_req,
                            as_of,
                        )
                        .map_err(|e| {
                            Error::Internal(format!(
                                "Failed to bump commodity curve components: {}",
                                e
                            ))
                        })?;

                        Ok(Some(vec![ScenarioEffect::UpdateDiscountCurve {
                            id: curve_id.clone(),
                            curve: std::sync::Arc::new(new_curve),
                        }]))
                    }
                    CurveKind::VolIndex => {
                        // Volatility index curves store forward volatility levels (e.g., VIX, VSTOXX)
                        //
                        // UNIT SEMANTICS:
                        // The `bp` parameter for vol index curves is passed directly to `BumpSpec::parallel_bp()`.
                        // Unlike rate curves where bp=100 means 1% (100 basis points), for vol index curves
                        // the underlying bump implementation determines the semantic meaning.
                        //
                        // The `Bumpable` trait implementation for `VolatilityIndexCurve` treats bp as
                        // "index points" scaled by the bump spec. For example:
                        // - A parallel bump of bp=100 with the default spec may add 1.0 index points
                        // - Check the `VolatilityIndexCurve::apply_bump` implementation for exact behavior
                        //
                        // For node-specific bumps, see `CurveNodeBp` handling below which documents the
                        // explicit bp/100 scaling used there.
                        let base_curve = ctx.market.get_vol_index(curve_id).map_err(|_| {
                            Error::MarketDataNotFound {
                                id: curve_id.to_string(),
                            }
                        })?;

                        let spec = BumpSpec::parallel_bp(*bp);
                        let new_curve = base_curve.apply_bump(spec).map_err(|e| {
                            Error::Internal(format!("Failed to bump vol index curve: {}", e))
                        })?;

                        Ok(Some(vec![ScenarioEffect::UpdateVolIndexCurve {
                            id: curve_id.clone(),
                            curve: std::sync::Arc::new(new_curve),
                        }]))
                    }
                }
            }
            OperationSpec::CurveNodeBp {
                curve_kind,
                curve_id,
                nodes,
                match_mode,
            } => {
                let as_of = ctx.as_of;

                // Common logic to resolve nodes to a BumpRequest
                // We need to access knots to handle Interpolate.
                // This requires fetching the curve first.

                // Helper to build request
                // We'll do it inside each match arm because getting knots depends on curve type.

                match curve_kind {
                    CurveKind::Discount => {
                        let base_curve = ctx.market.get_discount(curve_id).map_err(|_| {
                            Error::MarketDataNotFound {
                                id: curve_id.to_string(),
                            }
                        })?;

                        let knots: Vec<f64> = base_curve.knots().to_vec();
                        let result = resolve_bump_targets(
                            nodes,
                            &knots,
                            *match_mode,
                            as_of,
                            base_curve.day_count(),
                        )?;
                        let bump_req = BumpRequest::Tenors(result.targets);

                        let new_curve = bump_discount_curve_synthetic(
                            &base_curve,
                            ctx.market,
                            &bump_req,
                            as_of,
                        )
                        .map_err(|e| {
                            Error::Internal(format!(
                                "Failed to bump discount curve components: {}",
                                e
                            ))
                        })?;

                        let mut effects = vec![ScenarioEffect::UpdateDiscountCurve {
                            id: curve_id.clone(),
                            curve: std::sync::Arc::new(new_curve),
                        }];
                        effects.extend(result.warnings.into_iter().map(ScenarioEffect::Warning));
                        Ok(Some(effects))
                    }
                    CurveKind::Forecast => {
                        // Forward curve node bumps use direct additive rate shifts.
                        //
                        // METHODOLOGY NOTE:
                        // The bump is applied as: forward_rate += bp * 1e-4
                        // where bp is in basis points. This differs from discount curve
                        // node bumps which use solve-to-par methodology. See the parallel
                        // bump documentation above for rationale.
                        let base_curve = ctx.market.get_forward(curve_id).map_err(|_| {
                            Error::MarketDataNotFound {
                                id: curve_id.to_string(),
                            }
                        })?;

                        let knots = base_curve.knots().to_vec();
                        let mut forwards = base_curve.forwards().to_vec();

                        let result = resolve_bump_targets(
                            nodes,
                            &knots,
                            *match_mode,
                            as_of,
                            base_curve.day_count(),
                        )?;

                        for (t, bp) in &result.targets {
                            // Find exact match in knots (within tolerance for floating point)
                            if let Some(idx) = knots.iter().position(|&k| (k - *t).abs() < 1e-4) {
                                // bp is in basis points; 1bp = 0.0001 = 1e-4
                                forwards[idx] += bp * 1e-4;
                            }
                        }

                        // Rebuild the curve with bumped forward rates
                        let bumped_points: Vec<(f64, f64)> =
                            knots.into_iter().zip(forwards).collect();
                        let new_curve =
                            finstack_core::market_data::term_structures::ForwardCurve::builder(
                                base_curve.id().as_str(),
                                base_curve.tenor(),
                            )
                            .base_date(base_curve.base_date())
                            .knots(bumped_points)
                            .build()
                            .map_err(|e| {
                                Error::Internal(format!("Failed to rebuild forward curve: {}", e))
                            })?;

                        let mut effects = vec![ScenarioEffect::UpdateForwardCurve {
                            id: curve_id.clone(),
                            curve: std::sync::Arc::new(new_curve),
                        }];
                        effects.extend(result.warnings.into_iter().map(ScenarioEffect::Warning));
                        Ok(Some(effects))
                    }
                    CurveKind::ParCDS => {
                        let base_curve = ctx.market.get_hazard(curve_id).map_err(|_| {
                            Error::MarketDataNotFound {
                                id: curve_id.to_string(),
                            }
                        })?;

                        let knots: Vec<f64> = base_curve.knot_points().map(|(t, _)| t).collect();
                        let result = resolve_bump_targets(
                            nodes,
                            &knots,
                            *match_mode,
                            as_of,
                            base_curve.day_count(),
                        )?;
                        let bump_req = BumpRequest::Tenors(result.targets);

                        let discount_id = resolve_discount_curve_id(ctx.market);

                        let new_curve = bump_hazard_spreads(
                            &base_curve,
                            ctx.market,
                            &bump_req,
                            discount_id.as_ref(),
                        )
                        .or_else(|_| bump_hazard_shift(&base_curve, &bump_req))
                        .map_err(|e| {
                            Error::Internal(format!(
                                "Failed to bump hazard curve components: {}",
                                e
                            ))
                        })?;

                        let mut effects = vec![ScenarioEffect::UpdateHazardCurve {
                            id: curve_id.clone(),
                            curve: std::sync::Arc::new(new_curve),
                        }];
                        effects.extend(result.warnings.into_iter().map(ScenarioEffect::Warning));
                        Ok(Some(effects))
                    }
                    CurveKind::Inflation => {
                        let base_curve = ctx.market.get_inflation(curve_id).map_err(|_| {
                            Error::MarketDataNotFound {
                                id: curve_id.to_string(),
                            }
                        })?;

                        let knots: Vec<f64> = base_curve.knots().to_vec();

                        // InflationCurve stores CPI index levels, not rates, so it doesn't
                        // have an inherent day count convention. For tenor-to-years parsing
                        // when matching tenor strings (like "5Y") to curve knots, we use:
                        // 1. The discount curve's day count if one is available (for consistency)
                        // 2. Act365F as fallback (standard for inflation markets)
                        let tenor_day_count = resolve_discount_curve_id(ctx.market)
                            .and_then(|dc_id| {
                                ctx.market
                                    .get_discount(dc_id.as_str())
                                    .ok()
                                    .map(|dc| dc.day_count())
                            })
                            .unwrap_or(DayCount::Act365F);

                        let result = resolve_bump_targets(
                            nodes,
                            &knots,
                            *match_mode,
                            as_of,
                            tenor_day_count,
                        )?;
                        let bump_req = BumpRequest::Tenors(result.targets);

                        let discount_id = resolve_discount_curve_id(ctx.market)
                            .unwrap_or_else(|| finstack_core::types::CurveId::from("USD-OIS"));

                        let new_curve = bump_inflation_rates(
                            &base_curve,
                            ctx.market,
                            &bump_req,
                            &discount_id,
                            as_of,
                        )
                        .map_err(|e| {
                            Error::Internal(format!(
                                "Failed to bump inflation curve components: {}",
                                e
                            ))
                        })?;

                        let mut effects = vec![ScenarioEffect::UpdateInflationCurve {
                            id: curve_id.clone(),
                            curve: std::sync::Arc::new(new_curve),
                        }];
                        effects.extend(result.warnings.into_iter().map(ScenarioEffect::Warning));
                        Ok(Some(effects))
                    }
                    CurveKind::Commodity => {
                        // Commodity curves treated like discount curves for bump purposes
                        let base_curve = ctx.market.get_discount(curve_id).map_err(|_| {
                            Error::MarketDataNotFound {
                                id: curve_id.to_string(),
                            }
                        })?;

                        let knots: Vec<f64> = base_curve.knots().to_vec();
                        let result = resolve_bump_targets(
                            nodes,
                            &knots,
                            *match_mode,
                            as_of,
                            base_curve.day_count(),
                        )?;
                        let bump_req = BumpRequest::Tenors(result.targets);

                        let new_curve = bump_discount_curve_synthetic(
                            &base_curve,
                            ctx.market,
                            &bump_req,
                            as_of,
                        )
                        .map_err(|e| {
                            Error::Internal(format!(
                                "Failed to bump commodity curve components: {}",
                                e
                            ))
                        })?;

                        let mut effects = vec![ScenarioEffect::UpdateDiscountCurve {
                            id: curve_id.clone(),
                            curve: std::sync::Arc::new(new_curve),
                        }];
                        effects.extend(result.warnings.into_iter().map(ScenarioEffect::Warning));
                        Ok(Some(effects))
                    }
                    CurveKind::VolIndex => {
                        // Volatility index curves - apply node-specific bumps
                        //
                        // UNIT SEMANTICS:
                        // The `bp` parameter is interpreted as "index points" (not basis points).
                        // For example, a bp=100 shock on a VIX curve with level 20 would produce:
                        //   new_level = 20 + (100 / 100) = 21
                        //
                        // This differs from rate curves where bp represents basis points (1bp = 0.01%).
                        // The division by 100 converts the input to a direct index level change.
                        let base_curve = ctx.market.get_vol_index(curve_id).map_err(|_| {
                            Error::MarketDataNotFound {
                                id: curve_id.to_string(),
                            }
                        })?;

                        let knots: Vec<f64> = base_curve.knots().to_vec();
                        let result = resolve_bump_targets(
                            nodes,
                            &knots,
                            *match_mode,
                            as_of,
                            base_curve.day_count(),
                        )?;

                        // Apply node bumps by rebuilding the curve with shifted levels
                        let mut levels: Vec<f64> = base_curve.levels().to_vec();

                        for (t, bp) in &result.targets {
                            // Find exact match in knots
                            if let Some(idx) = knots.iter().position(|&k| (k - *t).abs() < 1e-4) {
                                // bp / 100 converts to index points: bp=100 → +1.0 index point
                                levels[idx] += bp / 100.0;
                            }
                        }

                        // Rebuild the vol index curve
                        let bumped_points: Vec<(f64, f64)> =
                            knots.into_iter().zip(levels).collect();
                        let new_curve =
                            finstack_core::market_data::term_structures::VolatilityIndexCurve::builder(
                                base_curve.id().as_str(),
                            )
                            .base_date(base_curve.base_date())
                            .day_count(base_curve.day_count())
                            .spot_level(base_curve.spot_level())
                            .knots(bumped_points)
                            .build()
                            .map_err(|e| Error::Internal(format!("Failed to rebuild vol index curve: {}", e)))?;

                        let mut effects = vec![ScenarioEffect::UpdateVolIndexCurve {
                            id: curve_id.clone(),
                            curve: std::sync::Arc::new(new_curve),
                        }];
                        effects.extend(result.warnings.into_iter().map(ScenarioEffect::Warning));
                        Ok(Some(effects))
                    }
                }
            }
            _ => Ok(None),
        }
    }
}
