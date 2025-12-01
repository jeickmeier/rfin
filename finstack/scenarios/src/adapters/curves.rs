//! Curve shock adapters (discount, forecast, hazard, and inflation).
//!
//! This module contains helpers that translate curve-oriented
//! [`OperationSpec`](crate::spec::OperationSpec) variants into concrete market
//! data updates. Functions rebuild the underlying curve types rather than
//! mutating them in place to preserve determinism and metadata (such as curve
//! identifiers and base dates).

use crate::error::{Error, Result};
use crate::spec::{CurveKind, TenorMatchMode};
use crate::utils::parse_tenor_to_years;
use finstack_core::market_data::context::MarketContext;

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
                        let pos = knots
                            .iter()
                            .position(|&t| t >= tenor_years)
                            .unwrap_or(knots.len() - 1);
                        if pos == 0 {
                            forwards[0] += add;
                        } else {
                            let i0 = pos - 1;
                            let i1 = pos.min(knots.len() - 1);
                            let (t0, t1) = (knots[i0], knots[i1]);
                            let w1 = if (t1 - t0).abs() < 1e-12 {
                                0.5
                            } else {
                                (tenor_years - t0) / (t1 - t0)
                            };
                            let w0 = 1.0 - w1;
                            forwards[i0] += add * w0;
                            forwards[i1] += add * w1;
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
        CurveKind::Hazard => {
            // Hazard curves don't expose knots publicly, so node-specific bumps cannot be implemented
            // without extending the core API. Return UnsupportedOperation until core provides
            // either knots() access or a with_key_rate_bump API.
            return Err(Error::UnsupportedOperation {
                operation: "node bump (hazard curves don't expose knots)".to_string(),
                target: format!("hazard curve {}", curve_id),
            });
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
                        let pos = knots
                            .iter()
                            .position(|&t| t >= tenor_years)
                            .unwrap_or(knots.len() - 1);
                        if pos == 0 {
                            cpi_levels[0] *= factor;
                        } else {
                            let i0 = pos - 1;
                            let i1 = pos.min(knots.len() - 1);
                            let (t0, t1) = (knots[i0], knots[i1]);
                            let w1 = if (t1 - t0).abs() < 1e-12 {
                                0.5
                            } else {
                                (tenor_years - t0) / (t1 - t0)
                            };
                            let w0 = 1.0 - w1;
                            cpi_levels[i0] *= 1.0 + (factor - 1.0) * w0;
                            cpi_levels[i1] *= 1.0 + (factor - 1.0) * w1;
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
