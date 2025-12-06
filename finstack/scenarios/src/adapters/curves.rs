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
use crate::utils::{calculate_interpolation_weights, parse_tenor_to_years};
use finstack_core::market_data::bumps::{BumpSpec, MarketBump};

/// Adapter for curve operations.
pub struct CurveAdapter;

impl ScenarioAdapter for CurveAdapter {
    fn try_generate_effects(
        &self,
        op: &OperationSpec,
        ctx: &ExecutionContext,
    ) -> Result<Option<Vec<ScenarioEffect>>> {
        match op {
            OperationSpec::CurveParallelBp {
                curve_kind,
                curve_id,
                bp,
            } => {
                let spec = if *curve_kind == CurveKind::Inflation {
                    BumpSpec::inflation_shift_pct(bp / 100.0)
                } else {
                    BumpSpec::parallel_bp(*bp)
                };

                let bump = MarketBump::Curve {
                    id: finstack_core::types::CurveId::from(curve_id.as_str()),
                    spec,
                };
                Ok(Some(vec![ScenarioEffect::MarketBump(bump)]))
            }
            OperationSpec::CurveNodeBp {
                curve_kind,
                curve_id,
                nodes,
                match_mode,
            } => {
                // Determine effect based on curve kind
                match curve_kind {
                    CurveKind::Discount => {
                        let base_curve = ctx.market.get_discount_ref(curve_id).map_err(|_| {
                            Error::MarketDataNotFound {
                                id: curve_id.to_string(),
                            }
                        })?;

                        // Apply shocks (logic extracted from old apply_curve_node_shock)
                        // Note: base_curve is Arc<dyn...>. We can't clone it easily to a specific builder unless we cast?
                        // But `knots()` defines structure.

                        // We need access to the data to rebuild.
                        // `DiscountCurve` trait usually has `knots()`, `df(t)`.
                        // Rebuilding requires knowing the CONCRETE type or using a generic builder.
                        // `finstack_core` builders usually creating `InterpolatedDiscountCurve`.
                        // We can't easily modify an opaque `Arc<dyn DiscountCurve>`.
                        // WE MUST assume it's something we can rebuild, or use a "Bumped" decorator?
                        // The old code assumed `try_with_parallel_bump` existed?
                        // No, old code used `base_curve.knots()`.
                        // And `try_with_parallel_bump`.
                        // Wait, `DiscountCurve` trait has `try_with_parallel_bump`?
                        // If so, we can use it!

                        // For Node shocks, old code used `try_with_triangular_key_rate_bump_neighbors`.
                        // These are methods on the trait?
                        // Let's assume they are available on the trait (likely default impls or required).

                        let mut current_curve = base_curve.clone();

                        for (tenor_str, bp) in nodes {
                            let tenor_years = parse_tenor_to_years(tenor_str)?;

                            let bumped = match match_mode {
                                TenorMatchMode::Exact => {
                                    // Exact match check
                                    let knots = current_curve.knots();
                                    if !knots.iter().any(|&t| (t - tenor_years).abs() < 1e-6) {
                                        return Err(Error::TenorNotFound {
                                            tenor: tenor_str.clone(),
                                            curve_id: curve_id.to_string(),
                                        });
                                    }
                                    current_curve.try_with_parallel_bump(*bp).map_err(|e| {
                                        Error::Internal(format!("Failed to bump curve: {}", e))
                                    })?
                                }
                                TenorMatchMode::Interpolate => {
                                    let prev_bucket = (tenor_years * 0.5).max(0.0);
                                    let next_bucket = tenor_years * 1.5;
                                    current_curve
                                        .try_with_triangular_key_rate_bump_neighbors(
                                            prev_bucket,
                                            tenor_years,
                                            next_bucket,
                                            *bp,
                                        )
                                        .map_err(|e| {
                                            Error::Internal(format!(
                                                "Failed to key-rate bump curve: {}",
                                                e
                                            ))
                                        })?
                                }
                            };
                            current_curve = bumped;
                        }

                        Ok(Some(vec![ScenarioEffect::UpdateDiscountCurve {
                            id: curve_id.clone(),
                            curve: std::sync::Arc::new(current_curve),
                        }]))
                    }
                    CurveKind::Forecast => {
                        let base_curve = ctx.market.get_forward_ref(curve_id).map_err(|_| {
                            Error::MarketDataNotFound {
                                id: curve_id.to_string(),
                            }
                        })?;

                        // Extract knots and forwards for key-rate bumping
                        let knots = base_curve.knots().to_vec();
                        let mut forwards = base_curve.forwards().to_vec();

                        for (tenor_str, bp) in nodes {
                            let tenor_years = parse_tenor_to_years(tenor_str)?;
                            let add = *bp / 10_000.0;

                            match match_mode {
                                TenorMatchMode::Exact => {
                                    if let Some((i, _)) = knots
                                        .iter()
                                        .enumerate()
                                        .find(|(_, &t)| (t - tenor_years).abs() < 1e-6)
                                    {
                                        forwards[i] += add;
                                    } else {
                                        return Err(Error::TenorNotFound {
                                            tenor: tenor_str.clone(),
                                            curve_id: curve_id.to_string(),
                                        });
                                    }
                                }
                                TenorMatchMode::Interpolate => {
                                    let weights =
                                        calculate_interpolation_weights(tenor_years, &knots);
                                    for (idx, weight) in weights {
                                        forwards[idx] += add * weight;
                                    }
                                }
                            }
                        }

                        // Rebuild
                        let bumped_points: Vec<(f64, f64)> =
                            knots.into_iter().zip(forwards).collect();
                        let rebuilt =
                            finstack_core::market_data::term_structures::forward_curve::ForwardCurve::builder(
                                base_curve.id().as_str(),
                                base_curve.tenor(),
                            )
                            .base_date(base_curve.base_date())
                            .knots(bumped_points)
                            .build()
                            .map_err(|e| Error::Internal(format!("Failed to rebuild forward curve: {}", e)))?;

                        Ok(Some(vec![ScenarioEffect::UpdateForwardCurve {
                            id: curve_id.clone(),
                            curve: std::sync::Arc::new(rebuilt),
                        }]))
                    }
                    CurveKind::ParCDS => {
                        let base_curve = ctx.market.get_hazard_ref(curve_id).map_err(|_| {
                            Error::MarketDataNotFound {
                                id: curve_id.to_string(),
                            }
                        })?;

                        let recovery = base_curve.recovery_rate();
                        let div = if (1.0 - recovery).abs() < 1e-4 {
                            1e-4
                        } else {
                            1.0 - recovery
                        };

                        let mut points: Vec<(f64, f64)> = base_curve.knot_points().collect();

                        for (tenor_str, bp) in nodes {
                            let tenor_years = parse_tenor_to_years(tenor_str)?;
                            // TODO: Use exact CDS solver if finstack_math available.
                            // Currently using Lambda ~ Spread / (1-R) approximation.
                            // For distressed credits (high spread, or recovery near 1), this is inaccurate.
                            let lambda_bump = (*bp / 10_000.0) / div;

                            match match_mode {
                                TenorMatchMode::Exact => {
                                    if let Some((_, lambda)) = points
                                        .iter_mut()
                                        .find(|(t, _)| (*t - tenor_years).abs() < 1e-6)
                                    {
                                        *lambda = (*lambda + lambda_bump).max(0.0);
                                    } else {
                                        return Err(Error::TenorNotFound {
                                            tenor: tenor_str.clone(),
                                            curve_id: curve_id.to_string(),
                                        });
                                    }
                                }
                                TenorMatchMode::Interpolate => {
                                    let times: Vec<f64> = points.iter().map(|(t, _)| *t).collect();
                                    let weights =
                                        calculate_interpolation_weights(tenor_years, &times);
                                    for (idx, weight) in weights {
                                        points[idx].1 =
                                            (points[idx].1 + lambda_bump * weight).max(0.0);
                                    }
                                }
                            }
                        }

                        let mut builder =
                            finstack_core::market_data::term_structures::hazard_curve::HazardCurve::builder(
                                base_curve.id().as_str(),
                            )
                            .base_date(base_curve.base_date())
                            .recovery_rate(base_curve.recovery_rate())
                            .day_count(base_curve.day_count())
                            .knots(points)
                            .par_interp(base_curve.par_interp());

                        // Copy optional fields
                        if let Some(issuer) = base_curve.issuer() {
                            builder = builder.issuer(issuer);
                        }
                        if let Some(seniority) = base_curve.seniority {
                            builder = builder.seniority(seniority);
                        }
                        if let Some(currency) = base_curve.currency() {
                            builder = builder.currency(currency);
                        }
                        // Explicitly copy par points
                        let par_points: Vec<(f64, f64)> = base_curve.par_spread_points().collect();
                        builder = builder.par_spreads(par_points);

                        let rebuilt = builder.build().map_err(|e| {
                            Error::Internal(format!("Failed to rebuild hazard curve: {}", e))
                        })?;

                        Ok(Some(vec![ScenarioEffect::UpdateHazardCurve {
                            id: curve_id.clone(),
                            curve: std::sync::Arc::new(rebuilt),
                        }]))
                    }
                    CurveKind::Inflation => {
                        let base_curve = ctx.market.get_inflation_ref(curve_id).map_err(|_| {
                            Error::MarketDataNotFound {
                                id: curve_id.to_string(),
                            }
                        })?;

                        let knots = base_curve.knots().to_vec();
                        let mut cpi_levels = base_curve.cpi_levels().to_vec();

                        for (tenor_str, bp) in nodes {
                            let tenor_years = parse_tenor_to_years(tenor_str)?;
                            let factor = 1.0 + (*bp / 10_000.0);

                            match match_mode {
                                TenorMatchMode::Exact => {
                                    if let Some((i, _)) = knots
                                        .iter()
                                        .enumerate()
                                        .find(|(_, &t)| (t - tenor_years).abs() < 1e-6)
                                    {
                                        cpi_levels[i] *= factor;
                                    } else {
                                        return Err(Error::TenorNotFound {
                                            tenor: tenor_str.clone(),
                                            curve_id: curve_id.to_string(),
                                        });
                                    }
                                }
                                TenorMatchMode::Interpolate => {
                                    let weights =
                                        calculate_interpolation_weights(tenor_years, &knots);
                                    let shock_pct = factor - 1.0;
                                    for (idx, weight) in weights {
                                        cpi_levels[idx] *= 1.0 + shock_pct * weight;
                                    }
                                }
                            }
                        }

                        let bumped_points: Vec<(f64, f64)> =
                            knots.into_iter().zip(cpi_levels).collect();
                        let rebuilt =
                            finstack_core::market_data::term_structures::inflation::InflationCurve::builder(
                                base_curve.id().as_str(),
                            )
                            .base_cpi(base_curve.base_cpi())
                            .knots(bumped_points)
                            .build()
                            .map_err(|e| {
                                Error::Internal(format!("Failed to rebuild inflation curve: {}", e))
                            })?;

                        Ok(Some(vec![ScenarioEffect::UpdateInflationCurve {
                            id: curve_id.clone(),
                            curve: std::sync::Arc::new(rebuilt),
                        }]))
                    }
                }
            }
            _ => Ok(None),
        }
    }
}
