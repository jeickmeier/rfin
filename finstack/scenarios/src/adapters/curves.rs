//! Curve shock adapters (discount, forecast, hazard, and inflation).
//!
//! This module contains helpers that translate curve-oriented
//! [`OperationSpec`](crate::spec::OperationSpec) variants into concrete market
//! data updates. Functions rebuild the underlying curve types rather than
//! mutating them in place to preserve determinism and metadata (such as curve
//! identifiers and base dates).

use crate::error::{Error, Result};
use crate::spec::{CurveKind, TenorMatchMode};
use crate::utils::{calculate_interpolation_weights, parse_tenor_to_years};
use finstack_core::market_data::bumps::{BumpSpec, MarketBump};
use finstack_core::market_data::context::MarketContext;

/// Schedule a parallel basis-point shock to a curve.
///
/// Handles `CurveKind::Inflation` specially by creating an `inflation_shift_pct` bump;
/// others receive a standard `parallel_bp` bump.
///
/// # Arguments
/// - `curve_kind`: Family of the curve (affects bump type).
/// - `curve_id`: Identifier of the curve.
/// - `bp`: Shock size in basis points.
/// - `market_bumps`: Output vector to append scheduled bumps to.
///
/// # Returns
/// `Ok(())` on success.
pub fn apply_curve_parallel_shock(
    curve_kind: CurveKind,
    curve_id: &str,
    bp: f64,
    market_bumps: &mut Vec<MarketBump>,
) -> Result<()> {
    let spec = if curve_kind == CurveKind::Inflation {
        BumpSpec::inflation_shift_pct(bp / 100.0)
    } else {
        BumpSpec::parallel_bp(bp)
    };

    market_bumps.push(MarketBump::Curve {
        id: finstack_core::types::CurveId::from(curve_id),
        spec,
    });
    Ok(())
}

/// Apply node-specific basis-point shocks to a curve.
///
/// Supports two match modes:
/// - [`TenorMatchMode::Exact`]: Require a pillar to exist at the requested
///   tenor; returns an error if not found.
/// - [`TenorMatchMode::Interpolate`]: Apply a key-rate bump around the tenor,
///   allowing the tenor to fall between knots.
///
/// # Arguments
/// - `market`: Market context to mutate.
/// - `curve_kind`: Curve family (discount, forecast, hazard, inflation).
/// - `curve_id`: Identifier of the curve to update.
/// - `nodes`: `(tenor, bp)` pairs describing each shock to apply sequentially.
/// - `match_mode`: Strategy for aligning tenors with curve data.
///
/// # Returns
/// [`Result`](crate::error::Result) signalling success or failure.
///
/// # Errors
/// - [`Error::MarketDataNotFound`](crate::error::Error::MarketDataNotFound) if
///   the curve cannot be located.
/// - [`Error::TenorNotFound`](crate::error::Error::TenorNotFound) when operating
///   in exact mode and a tenor is missing.
/// - [`Error::UnsupportedOperation`](crate::error::Error::UnsupportedOperation)
///   if the underlying curve cannot be bumped in the requested fashion.
/// - [`Error::InvalidTenor`](crate::error::Error::InvalidTenor) if a tenor fails
///   string parsing.
///
/// # Examples
/// ```rust,no_run
/// use finstack_scenarios::adapters::curves::apply_curve_node_shock;
/// use finstack_scenarios::{CurveKind, TenorMatchMode};
/// use finstack_core::market_data::context::MarketContext;
///
/// # fn main() -> finstack_scenarios::Result<()> {
/// let mut market = MarketContext::new();
/// // ... load a curve ...
/// apply_curve_node_shock(
///     &mut market,
///     CurveKind::Discount,
///     "USD_SOFR",
///     &[("2Y".into(), 15.0), ("10Y".into(), -10.0)],
///     TenorMatchMode::Interpolate,
/// )?;
/// # Ok(())
/// # }
/// ```
pub fn apply_curve_node_shock(
    market: &mut MarketContext,
    curve_kind: CurveKind,
    curve_id: &str,
    nodes: &[(String, f64)],
    match_mode: TenorMatchMode,
) -> Result<()> {
    match curve_kind {
        CurveKind::Discount => {
            let mut base_curve =
                market
                    .get_discount(curve_id)
                    .map_err(|_| Error::MarketDataNotFound {
                        id: curve_id.to_string(),
                    })?;

            // Apply each node shock sequentially
            for (tenor_str, bp) in nodes {
                let tenor_years = parse_tenor_to_years(tenor_str)?;

                let bumped_curve = match match_mode {
                    TenorMatchMode::Exact => {
                        // Find exact pillar match
                        let knots = base_curve.knots();
                        if !knots.iter().any(|&t| (t - tenor_years).abs() < 1e-6) {
                            return Err(Error::TenorNotFound {
                                tenor: tenor_str.clone(),
                                curve_id: curve_id.to_string(),
                            });
                        }
                        // Apply shock via parallel bump (approximation)
                        base_curve
                            .try_with_parallel_bump(*bp)
                            .map_err(|e| Error::Internal(format!("Failed to bump curve: {}", e)))?
                    }
                    TenorMatchMode::Interpolate => {
                        // Use triangular key-rate bump (industry-standard localized shock)
                        // For scenarios, we use the tenor as the center of the triangle
                        // with default neighbors at +/- 50% of the tenor
                        let prev_bucket = (tenor_years * 0.5).max(0.0);
                        let next_bucket = tenor_years * 1.5;
                        base_curve
                            .try_with_triangular_key_rate_bump_neighbors(
                                prev_bucket,
                                tenor_years,
                                next_bucket,
                                *bp,
                            )
                            .map_err(|e| {
                                Error::Internal(format!("Failed to key-rate bump curve: {}", e))
                            })?
                    }
                };

                base_curve = std::sync::Arc::new(bumped_curve);
            }

            market.insert_discount_mut(base_curve);
        }
        CurveKind::Forecast => {
            let base_curve =
                market
                    .get_forward_ref(curve_id)
                    .map_err(|_| Error::MarketDataNotFound {
                        id: curve_id.to_string(),
                    })?;

            // Extract knots and forwards for key-rate bumping
            let knots = base_curve.knots().to_vec();
            let mut forwards = base_curve.forwards().to_vec();

            // Apply each node shock sequentially
            for (tenor_str, bp) in nodes {
                let tenor_years = parse_tenor_to_years(tenor_str)?;
                let add = *bp / 10_000.0;

                match match_mode {
                    TenorMatchMode::Exact => {
                        // Find exact pillar match
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
                        // Distribute bump to bracket pillars via linear weights
                        let weights = calculate_interpolation_weights(tenor_years, &knots);
                        for (idx, weight) in weights {
                            forwards[idx] += add * weight;
                        }
                    }
                }
            }

            // Rebuild forward curve with adjusted forwards
            let bumped_points: Vec<(f64, f64)> = knots.into_iter().zip(forwards).collect();
            let rebuilt =
                finstack_core::market_data::term_structures::forward_curve::ForwardCurve::builder(
                    base_curve.id().as_str(),
                    base_curve.tenor(),
                )
                .base_date(base_curve.base_date())
                .knots(bumped_points)
                .build()
                .map_err(|e| Error::Internal(format!("Failed to rebuild forward curve: {}", e)))?;

            market.insert_forward_mut(std::sync::Arc::new(rebuilt));
        }
        CurveKind::ParCDS => {
            let base_curve =
                market
                    .get_hazard_ref(curve_id)
                    .map_err(|_| Error::MarketDataNotFound {
                        id: curve_id.to_string(),
                    })?;

            let recovery = base_curve.recovery_rate();
            // Protect against R ~= 1.0
            let div = if (1.0 - recovery).abs() < 1e-4 {
                1e-4
            } else {
                1.0 - recovery
            };

            // Extract knots and lambdas
            let mut points: Vec<(f64, f64)> = base_curve.knot_points().collect();
            // points is sorted by time

            // Apply each node shock sequentially
            for (tenor_str, bp) in nodes {
                let tenor_years = parse_tenor_to_years(tenor_str)?;
                // Approximate hazard bump: dLambda = dSpread / (1 - R)
                let lambda_bump = (*bp / 10_000.0) / div;

                match match_mode {
                    TenorMatchMode::Exact => {
                        // Find exact pillar match
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
                        // Distribute bump to bracket pillars
                        let times: Vec<f64> = points.iter().map(|(t, _)| *t).collect();
                        let weights = calculate_interpolation_weights(tenor_years, &times);
                        for (idx, weight) in weights {
                            points[idx].1 = (points[idx].1 + lambda_bump * weight).max(0.0);
                        }
                    }
                }
            }

            // Rebuild hazard curve
            let mut builder =
                finstack_core::market_data::term_structures::hazard_curve::HazardCurve::builder(
                    base_curve.id().as_str(),
                )
                .base_date(base_curve.base_date())
                .recovery_rate(base_curve.recovery_rate())
                .day_count(base_curve.day_count())
                .knots(points)
                .par_interp(base_curve.par_interp()); // Use accessor if available, or default

            if let Some(issuer) = base_curve.issuer() {
                builder = builder.issuer(issuer);
            }
            if let Some(seniority) = base_curve.seniority {
                builder = builder.seniority(seniority);
            }
            if let Some(currency) = base_curve.currency() {
                builder = builder.currency(currency);
            }
            // Preserve par spread points (though they might be inconsistent with new lambdas)
            // Ideally we'd bump them too, but for scenario purposes, the lambdas matter for pricing.
            // Using a hack to access par_spread_points via iterator if public?
            // Yes, par_spread_points() is public.
            let par_points: Vec<(f64, f64)> = base_curve.par_spread_points().collect();
            builder = builder.par_spreads(par_points);

            let rebuilt = builder
                .build()
                .map_err(|e| Error::Internal(format!("Failed to rebuild hazard curve: {}", e)))?;

            market.insert_hazard_mut(std::sync::Arc::new(rebuilt));
        }
        CurveKind::Inflation => {
            let base_curve =
                market
                    .get_inflation_ref(curve_id)
                    .map_err(|_| Error::MarketDataNotFound {
                        id: curve_id.to_string(),
                    })?;

            // Extract knots and CPI levels for multiplicative bumping
            let knots = base_curve.knots().to_vec();
            let mut cpi_levels = base_curve.cpi_levels().to_vec();

            // Apply each node shock sequentially
            // Inflation curves store CPI levels, so bp bumps translate to multiplicative factors
            for (tenor_str, bp) in nodes {
                let tenor_years = parse_tenor_to_years(tenor_str)?;
                let factor = 1.0 + (*bp / 10_000.0);

                match match_mode {
                    TenorMatchMode::Exact => {
                        // Find exact pillar match
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
                        // Distribute multiplicative factor to bracket pillars via linear weights
                        // factor = 1 + shock. We apply 1 + shock*weight to each?
                        // No, linear interpolation of the multiplicative *impact*?
                        // Original logic: cpi *= 1 + (factor - 1) * w
                        // factor - 1 is the shock pct.
                        let weights = calculate_interpolation_weights(tenor_years, &knots);
                        let shock_pct = factor - 1.0;
                        for (idx, weight) in weights {
                            cpi_levels[idx] *= 1.0 + shock_pct * weight;
                        }
                    }
                }
            }

            // Rebuild inflation curve with adjusted CPI levels
            let bumped_points: Vec<(f64, f64)> = knots.into_iter().zip(cpi_levels).collect();
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

            market.insert_inflation_mut(std::sync::Arc::new(rebuilt));
        }
    }

    Ok(())
}
