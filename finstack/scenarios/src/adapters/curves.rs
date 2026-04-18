//! Curve shock adapters (discount, forecast, hazard, and inflation).
//!
//! This module contains helpers that translate curve-oriented
//! [`OperationSpec`] variants into concrete market
//! data updates. Functions rebuild the underlying curve types rather than
//! mutating them in place to preserve determinism and metadata (such as curve
//! identifiers and base dates).

use crate::adapters::traits::{ScenarioAdapter, ScenarioEffect};
use crate::engine::ExecutionContext;
use crate::error::{Error, Result};
use crate::spec::{CurveKind, OperationSpec, TenorMatchMode};
use crate::utils::{calculate_interpolation_weights, parse_tenor_to_years_with_context};
use finstack_core::dates::{BusinessDayConvention, DayCount};
use finstack_core::market_data::bumps::{BumpSpec, MarketBump};
use finstack_core::market_data::context::CurveStorage;

/// Adapter for curve operations.
pub struct CurveAdapter;

/// Construct the `MarketDataNotFound` error for a curve that failed to fetch.
fn missing_market_err(curve_id: &str) -> Error {
    Error::MarketDataNotFound {
        id: curve_id.to_string(),
    }
}

/// Build the default effect vector for a curve shock: `UpdateCurve` followed by
/// any warnings accumulated during bump resolution.
fn update_effects<C>(new_curve: C, warnings: Vec<String>) -> Vec<ScenarioEffect>
where
    CurveStorage: From<C>,
{
    let mut effects = vec![ScenarioEffect::UpdateCurve(CurveStorage::from(new_curve))];
    effects.extend(warnings.into_iter().map(ScenarioEffect::Warning));
    effects
}

/// Result of resolving bump targets, including any warnings.
struct BumpTargetResult {
    /// Resolved (time, bump_value) pairs for curve knots (used by BumpRequest::Tenors).
    targets: Vec<(f64, f64)>,
    /// Resolved (knot_index, bump_value) pairs for direct curve modification.
    /// Avoids tolerance-sensitive float matching when applying node bumps.
    indexed_targets: Vec<(usize, f64)>,
    /// Warnings generated during resolution (e.g., extrapolation).
    warnings: Vec<String>,
}

/// Resolve tenor strings to curve knot positions and bump magnitudes.
///
/// Uses calendar-aware parsing by default, with a simple-parsing fallback for
/// test convenience (test cases often use bare integers like `"5Y"` that
/// match knots exactly under simple parsing but land slightly off under
/// calendar-aware parsing). The simple fallback should be revisited once all
/// callers supply a production calendar.
fn resolve_bump_targets(
    nodes: &[(String, f64)],
    knots: &[f64],
    match_mode: TenorMatchMode,
    as_of: finstack_core::dates::Date,
    day_count: DayCount,
) -> Result<BumpTargetResult> {
    let mut targets = Vec::new();
    let mut indexed_targets = Vec::new();
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
                let match_ctx = knots
                    .iter()
                    .enumerate()
                    .find(|(_, t)| (**t - tenor_years_ctx).abs() < 1e-6);
                let match_simple = knots
                    .iter()
                    .enumerate()
                    .find(|(_, t)| (**t - tenor_years_simple).abs() < 1e-6);

                let (idx, target_years) = match (match_ctx, match_simple) {
                    (Some((i, _)), _) => (i, tenor_years_ctx),
                    (None, Some((i, _))) => (i, tenor_years_simple),
                    (None, None) => {
                        return Err(Error::TenorNotFound {
                            tenor: tenor_str.clone(),
                            curve_id: "unknown".to_string(),
                        })
                    }
                };

                targets.push((target_years, add));
                indexed_targets.push((idx, add));
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
                let result = calculate_interpolation_weights(use_years, knots);

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
                    indexed_targets.push((idx, add * weight));
                }
            }
        }
    }
    Ok(BumpTargetResult {
        targets,
        indexed_targets,
        warnings,
    })
}

/// Resolve the discount curve ID used by recalibration-based curve bumps.
fn resolve_discount_curve_id(
    market: &finstack_core::market_data::context::MarketContext,
    explicit_discount_curve_id: Option<&str>,
    hint_curve_id: Option<&str>,
) -> Result<(finstack_core::types::CurveId, Option<String>)> {
    if let Some(explicit_id) = explicit_discount_curve_id {
        market
            .get_discount(explicit_id)
            .map_err(|_| missing_market_err(explicit_id))?;
        return Ok((finstack_core::types::CurveId::from(explicit_id), None));
    }

    let state: finstack_core::market_data::context::MarketContextState = market.into();
    let discount_curves: Vec<_> = state
        .curves
        .iter()
        .filter_map(|c| match c {
            finstack_core::market_data::context::CurveState::Discount(dc) => Some(dc),
            _ => None,
        })
        .collect();

    if discount_curves.is_empty() {
        return Err(Error::Validation(
            "No discount curves are available for recalibration-based scenario bump".into(),
        ));
    }

    if let Some(hint) = hint_curve_id {
        let ccy_prefix = hint.get(..3).unwrap_or("");
        if ccy_prefix.len() == 3 && ccy_prefix.chars().all(|c| c.is_ascii_uppercase()) {
            let prefix_matches: Vec<_> = discount_curves
                .iter()
                .filter(|dc| dc.id().as_str().starts_with(ccy_prefix))
                .collect();

            if prefix_matches.len() > 1 {
                return Err(Error::Validation(format!(
                    "Ambiguous discount curve resolution for '{}': multiple '{}' discount curves found",
                    hint, ccy_prefix
                )));
            }

            if let Some(discount_curve) = prefix_matches.first() {
                let discount_curve_id =
                    finstack_core::types::CurveId::from(discount_curve.id().as_str());
                return Ok((
                    discount_curve_id.clone(),
                    Some(format!(
                        "Using heuristic discount curve '{}' for '{}'",
                        discount_curve_id.as_str(),
                        hint
                    )),
                ));
            }
        }
    }

    if discount_curves.len() == 1 {
        let discount_curve_id =
            finstack_core::types::CurveId::from(discount_curves[0].id().as_str());
        return Ok((
            discount_curve_id.clone(),
            Some(format!(
                "Using only available discount curve '{}' as fallback",
                discount_curve_id.as_str()
            )),
        ));
    }

    Err(Error::Validation(format!(
        "Unable to resolve discount curve for '{}' without an explicit discount_curve_id",
        hint_curve_id.unwrap_or("curve bump")
    )))
}

impl ScenarioAdapter for CurveAdapter {
    fn try_generate_effects(
        &self,
        op: &OperationSpec,
        ctx: &ExecutionContext,
    ) -> Result<Option<Vec<ScenarioEffect>>> {
        use finstack_valuations::calibration::bumps::{
            bump_discount_curve_synthetic, bump_hazard_shift, bump_hazard_spreads,
            bump_inflation_rates, infer_currency_from_curve_id,
            infer_currency_from_discount_curve_id, observation_lag_from_curve, BumpRequest,
        };

        match op {
            OperationSpec::CurveParallelBp {
                curve_kind,
                curve_id,
                discount_curve_id,
                bp,
            } => {
                let bump_req = BumpRequest::Parallel(*bp);
                let as_of = ctx.as_of; // Assuming Context has as_of

                // For CurveParallelBp, we can use the shared logic which does Solve-to-Par.
                // This replaces the old behavior which might have been simple zero-shifting.
                // Solve-to-Par is generally preferred for scenarios.

                match curve_kind {
                    CurveKind::Discount => {
                        let base_curve = ctx
                            .market
                            .get_discount(curve_id)
                            .map_err(|_| missing_market_err(curve_id))?;

                        let currency = infer_currency_from_discount_curve_id(&base_curve);
                        let new_curve = bump_discount_curve_synthetic(
                            &base_curve,
                            ctx.market,
                            &bump_req,
                            as_of,
                            currency,
                        )
                        .map_err(|e| {
                            Error::Internal(format!("Failed to bump discount curve: {}", e))
                        })?;

                        Ok(Some(vec![ScenarioEffect::UpdateCurve(
                            finstack_core::market_data::context::CurveStorage::from(new_curve),
                        )]))
                    }
                    CurveKind::Forward => {
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
                        let _base_curve = ctx
                            .market
                            .get_forward(curve_id)
                            .map_err(|_| missing_market_err(curve_id))?;

                        let spec = BumpSpec::parallel_bp(*bp);
                        let bump = MarketBump::Curve {
                            id: finstack_core::types::CurveId::from(curve_id.as_str()),
                            spec,
                        };
                        Ok(Some(vec![ScenarioEffect::MarketBump(bump)]))
                    }
                    CurveKind::ParCDS => {
                        let base_curve = ctx
                            .market
                            .get_hazard(curve_id)
                            .map_err(|_| missing_market_err(curve_id))?;
                        let (discount_id, warning) = resolve_discount_curve_id(
                            ctx.market,
                            discount_curve_id.as_deref(),
                            Some(curve_id),
                        )?;
                        let mut fallback_warning: Option<String> = None;
                        let new_curve = match bump_hazard_spreads(
                            &base_curve,
                            ctx.market,
                            &bump_req,
                            Some(&discount_id),
                        ) {
                            Ok(c) => c,
                            Err(recalib_err) => {
                                tracing::warn!(
                                    curve_id = %curve_id,
                                    error = %recalib_err,
                                    "Hazard curve recalibration failed; falling back to direct hazard-rate shift"
                                );
                                let msg = format!(
                                    "Hazard curve '{curve_id}' parallel shock: par-CDS recalibration \
                                     failed ({recalib_err}); applied direct hazard-rate shift instead. \
                                     Risk-neutral default probabilities will be additively shifted by the \
                                     requested bp amount at each pillar rather than re-solved from par \
                                     spreads, which can materially change CS01 for sharply sloped curves."
                                );
                                fallback_warning = Some(msg);
                                bump_hazard_shift(&base_curve, &bump_req).map_err(|e| {
                                    Error::Internal(format!("Failed to bump hazard curve: {}", e))
                                })?
                            }
                        };

                        let mut effects = vec![ScenarioEffect::UpdateCurve(
                            finstack_core::market_data::context::CurveStorage::from(new_curve),
                        )];
                        if let Some(w) = fallback_warning {
                            effects.push(ScenarioEffect::Warning(w));
                        }
                        if let Some(warning) = warning {
                            effects.push(ScenarioEffect::Warning(warning));
                        }
                        Ok(Some(effects))
                    }
                    CurveKind::Inflation => {
                        let base_curve = ctx
                            .market
                            .get_inflation_curve(curve_id)
                            .map_err(|_| missing_market_err(curve_id))?;

                        let (discount_id, warning) = resolve_discount_curve_id(
                            ctx.market,
                            discount_curve_id.as_deref(),
                            Some(curve_id),
                        )?;

                        let currency = infer_currency_from_curve_id(&base_curve);
                        let lag = observation_lag_from_curve(&base_curve);
                        let new_curve = bump_inflation_rates(
                            &base_curve,
                            ctx.market,
                            &bump_req,
                            &discount_id,
                            as_of,
                            currency,
                            &lag,
                        )
                        .map_err(|e| {
                            Error::Internal(format!("Failed to bump inflation curve: {}", e))
                        })?;

                        let mut effects = vec![ScenarioEffect::UpdateCurve(
                            finstack_core::market_data::context::CurveStorage::from(new_curve),
                        )];
                        if let Some(warning) = warning {
                            effects.push(ScenarioEffect::Warning(warning));
                        }
                        Ok(Some(effects))
                    }
                    CurveKind::Commodity => {
                        // Commodity curves stored as DiscountCurve (convenience yields/cost-of-carry).
                        // Use direct additive rate shifts, NOT solve-to-par (which would
                        // incorrectly apply swap-rate calibration to commodity yields).
                        let _base_curve = ctx
                            .market
                            .get_discount(curve_id)
                            .map_err(|_| missing_market_err(curve_id))?;

                        let spec = BumpSpec::parallel_bp(*bp);
                        let bump = MarketBump::Curve {
                            id: finstack_core::types::CurveId::from(curve_id.as_str()),
                            spec,
                        };
                        Ok(Some(vec![ScenarioEffect::MarketBump(bump)]))
                    }
                    CurveKind::VolIndex => {
                        // Volatility index curves store forward volatility *levels* (e.g., VIX).
                        //
                        // Normalize `bp` to absolute index points so parallel and node shocks use
                        // the same convention:
                        // - `bp = 100`  -> `+1.0` vol-index point
                        // - `bp = -25`  -> `-0.25` vol-index points
                        //
                        // This intentionally differs from rate curves where `bp` means 1e-4 in
                        // fractional rate space. Emit a warning so the convention is auditable
                        // in the ApplicationReport.
                        let base_curve = ctx
                            .market
                            .get_vol_index_curve(curve_id)
                            .map_err(|_| missing_market_err(curve_id))?;

                        let pts = *bp / 100.0;
                        let new_curve = base_curve.with_parallel_bump(pts).map_err(|e| {
                            Error::Internal(format!("Failed to bump vol index curve: {}", e))
                        })?;

                        Ok(Some(vec![
                            ScenarioEffect::UpdateCurve(
                                finstack_core::market_data::context::CurveStorage::from(new_curve),
                            ),
                            ScenarioEffect::Warning(format!(
                                "VolIndex '{curve_id}' parallel shock: {bp} bp interpreted as \
                                 {pts:+.4} absolute vol-index points (VolIndex uses bp/100 \
                                 scaling, not the 1 bp = 0.0001 rate convention)"
                            )),
                        ]))
                    }
                }
            }
            OperationSpec::CurveNodeBp {
                curve_kind,
                curve_id,
                discount_curve_id,
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
                        let base_curve = ctx
                            .market
                            .get_discount(curve_id)
                            .map_err(|_| missing_market_err(curve_id))?;

                        let knots: Vec<f64> = base_curve.knots().to_vec();
                        let result = resolve_bump_targets(
                            nodes,
                            &knots,
                            *match_mode,
                            as_of,
                            base_curve.day_count(),
                        )?;
                        let bump_req = BumpRequest::Tenors(result.targets);

                        let currency = infer_currency_from_discount_curve_id(&base_curve);
                        let new_curve = bump_discount_curve_synthetic(
                            &base_curve,
                            ctx.market,
                            &bump_req,
                            as_of,
                            currency,
                        )
                        .map_err(|e| {
                            Error::Internal(format!(
                                "Failed to bump discount curve components: {}",
                                e
                            ))
                        })?;

                        Ok(Some(update_effects(new_curve, result.warnings)))
                    }
                    CurveKind::Forward => {
                        // Forward curve node bumps use direct additive rate shifts.
                        //
                        // METHODOLOGY NOTE:
                        // The bump is applied as: forward_rate += bp * 1e-4
                        // where bp is in basis points. This differs from discount curve
                        // node bumps which use solve-to-par methodology. See the parallel
                        // bump documentation above for rationale.
                        let base_curve = ctx
                            .market
                            .get_forward(curve_id)
                            .map_err(|_| missing_market_err(curve_id))?;

                        let knots = base_curve.knots().to_vec();
                        let mut forwards = base_curve.forwards().to_vec();

                        let result = resolve_bump_targets(
                            nodes,
                            &knots,
                            *match_mode,
                            as_of,
                            base_curve.day_count(),
                        )?;

                        // Use indexed_targets for precise knot matching (avoids
                        // float tolerance issues from round-tripping through time values).
                        for &(idx, bp) in &result.indexed_targets {
                            // bp is in basis points; 1bp = 0.0001 = 1e-4
                            forwards[idx] += bp * 1e-4;
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

                        Ok(Some(update_effects(new_curve, result.warnings)))
                    }
                    CurveKind::ParCDS => {
                        let base_curve = ctx
                            .market
                            .get_hazard(curve_id)
                            .map_err(|_| missing_market_err(curve_id))?;

                        let knots: Vec<f64> = base_curve.knot_points().map(|(t, _)| t).collect();
                        let result = resolve_bump_targets(
                            nodes,
                            &knots,
                            *match_mode,
                            as_of,
                            base_curve.day_count(),
                        )?;
                        let bump_req = BumpRequest::Tenors(result.targets);

                        let (discount_id, warning) = resolve_discount_curve_id(
                            ctx.market,
                            discount_curve_id.as_deref(),
                            Some(curve_id),
                        )?;

                        let mut fallback_warning: Option<String> = None;
                        let new_curve = match bump_hazard_spreads(
                            &base_curve,
                            ctx.market,
                            &bump_req,
                            Some(&discount_id),
                        ) {
                            Ok(c) => c,
                            Err(recalib_err) => {
                                tracing::warn!(
                                    curve_id = %curve_id,
                                    error = %recalib_err,
                                    "Hazard curve recalibration failed; falling back to direct hazard-rate shift"
                                );
                                let msg = format!(
                                    "Hazard curve '{curve_id}' node shock: par-CDS recalibration failed \
                                     ({recalib_err}); applied direct hazard-rate shift instead. \
                                     Risk-neutral default probabilities will be additively shifted by the \
                                     requested bp amount at the targeted pillars rather than re-solved \
                                     from par spreads, which can materially change CS01 for sharply \
                                     sloped curves."
                                );
                                fallback_warning = Some(msg);
                                bump_hazard_shift(&base_curve, &bump_req).map_err(|e| {
                                    Error::Internal(format!(
                                        "Failed to bump hazard curve components: {}",
                                        e
                                    ))
                                })?
                            }
                        };

                        let mut effects = vec![ScenarioEffect::UpdateCurve(
                            finstack_core::market_data::context::CurveStorage::from(new_curve),
                        )];
                        if let Some(w) = fallback_warning {
                            effects.push(ScenarioEffect::Warning(w));
                        }
                        effects.extend(result.warnings.into_iter().map(ScenarioEffect::Warning));
                        if let Some(warning) = warning {
                            effects.push(ScenarioEffect::Warning(warning));
                        }
                        Ok(Some(effects))
                    }
                    CurveKind::Inflation => {
                        let base_curve = ctx
                            .market
                            .get_inflation_curve(curve_id)
                            .map_err(|_| missing_market_err(curve_id))?;

                        let knots: Vec<f64> = base_curve.knots().to_vec();

                        // InflationCurve stores CPI index levels, not rates, so it doesn't
                        // have an inherent day count convention. For tenor-to-years parsing
                        // when matching tenor strings (like "5Y") to curve knots, we use:
                        // 1. The discount curve's day count if one is available (for consistency)
                        // 2. Act365F as fallback (standard for inflation markets)
                        let (discount_id, warning) = resolve_discount_curve_id(
                            ctx.market,
                            discount_curve_id.as_deref(),
                            Some(curve_id),
                        )?;
                        let tenor_day_count = ctx
                            .market
                            .get_discount(discount_id.as_str())
                            .map(|dc| dc.day_count())
                            .unwrap_or(DayCount::Act365F);

                        let result = resolve_bump_targets(
                            nodes,
                            &knots,
                            *match_mode,
                            as_of,
                            tenor_day_count,
                        )?;
                        let bump_req = BumpRequest::Tenors(result.targets);

                        let currency = infer_currency_from_curve_id(&base_curve);
                        let lag = observation_lag_from_curve(&base_curve);
                        let new_curve = bump_inflation_rates(
                            &base_curve,
                            ctx.market,
                            &bump_req,
                            &discount_id,
                            as_of,
                            currency,
                            &lag,
                        )
                        .map_err(|e| {
                            Error::Internal(format!(
                                "Failed to bump inflation curve components: {}",
                                e
                            ))
                        })?;

                        let mut effects = vec![ScenarioEffect::UpdateCurve(
                            finstack_core::market_data::context::CurveStorage::from(new_curve),
                        )];
                        effects.extend(result.warnings.into_iter().map(ScenarioEffect::Warning));
                        if let Some(warning) = warning {
                            effects.push(ScenarioEffect::Warning(warning));
                        }
                        Ok(Some(effects))
                    }
                    CurveKind::Commodity => {
                        // Commodity curves stored as DiscountCurve (convenience yields).
                        // Apply node-specific additive zero-rate shifts rather than
                        // solve-to-par, preserving curve shape at unshocked tenors.
                        let base_curve = ctx
                            .market
                            .get_discount(curve_id)
                            .map_err(|_| missing_market_err(curve_id))?;

                        let knots: Vec<f64> = base_curve.knots().to_vec();
                        let result = resolve_bump_targets(
                            nodes,
                            &knots,
                            *match_mode,
                            as_of,
                            base_curve.day_count(),
                        )?;

                        // Direct zero-rate shifts: convert DF → zero rate, bump, convert back.
                        // This avoids solve-to-par calibration which is inappropriate for
                        // commodity convenience yields.
                        let mut dfs: Vec<f64> = base_curve.dfs().to_vec();
                        for &(idx, bp_shift) in &result.indexed_targets {
                            let t = knots[idx];
                            if t > 1e-12 {
                                if dfs[idx] <= 0.0 {
                                    tracing::warn!(idx, df = dfs[idx], "Non-positive discount factor in commodity curve bump; skipping node");
                                    continue;
                                }
                                let zero = -(dfs[idx].ln()) / t;
                                let shifted = zero + bp_shift * 1e-4;
                                dfs[idx] = (-shifted * t).exp();
                            }
                        }

                        let bumped_points: Vec<(f64, f64)> = knots.into_iter().zip(dfs).collect();
                        let new_curve =
                            finstack_core::market_data::term_structures::DiscountCurve::builder(
                                base_curve.id().as_str(),
                            )
                            .base_date(base_curve.base_date())
                            .day_count(base_curve.day_count())
                            .interp(base_curve.interp_style())
                            .extrapolation(base_curve.extrapolation())
                            .allow_non_monotonic()
                            .knots(bumped_points)
                            .build()
                            .map_err(|e| {
                                Error::Internal(format!("Failed to rebuild commodity curve: {}", e))
                            })?;

                        Ok(Some(update_effects(new_curve, result.warnings)))
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
                        let base_curve = ctx
                            .market
                            .get_vol_index_curve(curve_id)
                            .map_err(|_| missing_market_err(curve_id))?;

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

                        // Use indexed_targets for precise knot matching (avoids
                        // float tolerance issues from round-tripping through time values).
                        let mut total_bp_abs = 0.0_f64;
                        for &(idx, bp) in &result.indexed_targets {
                            // bp / 100 converts to index points: bp=100 → +1.0 index point
                            levels[idx] += bp / 100.0;
                            total_bp_abs += bp.abs();
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

                        let mut effects = vec![ScenarioEffect::UpdateCurve(
                            finstack_core::market_data::context::CurveStorage::from(new_curve),
                        )];
                        if total_bp_abs > 0.0 {
                            effects.push(ScenarioEffect::Warning(format!(
                                "VolIndex '{}' node shocks: bp values rescaled by bp/100 to \
                                 absolute vol-index points (not the 1 bp = 0.0001 rate convention)",
                                curve_id
                            )));
                        }
                        effects.extend(result.warnings.into_iter().map(ScenarioEffect::Warning));
                        Ok(Some(effects))
                    }
                }
            }
            _ => Ok(None),
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use finstack_core::market_data::context::MarketContext;
    use finstack_core::market_data::term_structures::VolatilityIndexCurve;
    use finstack_statements::FinancialModelSpec;
    use time::macros::date;

    #[test]
    fn vol_index_parallel_bp_uses_absolute_index_points() {
        let as_of = date!(2025 - 01 - 01);
        let vol_curve = VolatilityIndexCurve::builder("VIX")
            .base_date(as_of)
            .spot_level(18.5)
            .knots([(0.0, 18.5), (0.25, 20.0), (0.5, 21.5)])
            .build()
            .expect("vol index curve should build");
        let mut market = MarketContext::new().insert(vol_curve);
        let mut model = FinancialModelSpec::new("demo", vec![]);
        let ctx = ExecutionContext {
            market: &mut market,
            model: &mut model,
            instruments: None,
            rate_bindings: None,
            calendar: None,
            as_of,
        };

        let op = OperationSpec::CurveParallelBp {
            curve_kind: CurveKind::VolIndex,
            curve_id: "VIX".to_string(),
            discount_curve_id: None,
            bp: 100.0,
        };

        let effects = CurveAdapter
            .try_generate_effects(&op, &ctx)
            .expect("curve adapter should succeed")
            .expect("vol index curve shock should be handled");

        let updated_curve = match &effects[0] {
            ScenarioEffect::UpdateCurve(storage) => storage
                .vol_index()
                .expect("expected vol index update effect"),
            effect => panic!("expected updated vol index curve, got {effect:?}"),
        };

        assert!((updated_curve.spot_level() - 19.5).abs() < 1.0e-12);
        assert!((updated_curve.forward_level(0.25) - 21.0).abs() < 1.0e-12);
    }
}
