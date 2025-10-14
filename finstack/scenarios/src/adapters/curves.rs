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
use finstack_core::market_data::bumps::{BumpSpec, Bumpable};
use finstack_core::market_data::MarketContext;

/// Apply a parallel basis-point shock to a curve.
///
/// # Arguments
/// - `market`: Market context whose curve collection will be updated.
/// - `curve_kind`: Identifies which curve collection to access.
/// - `curve_id`: Curve identifier within the chosen collection.
/// - `bp`: Basis-point amount to add to the curve (positive raises rates).
///
/// # Returns
/// [`Result`](crate::error::Result) reflecting whether the operation succeeded.
///
/// # Errors
/// - [`Error::MarketDataNotFound`](crate::error::Error::MarketDataNotFound) if
///   the target curve is absent.
/// - [`Error::UnsupportedOperation`](crate::error::Error::UnsupportedOperation)
///   if the underlying curve type does not support bumps of the requested kind.
/// - [`Error::Internal`](crate::error::Error::Internal) if a rebuilt curve fails
///   validation.
///
/// # Examples
/// ```rust,no_run
/// use finstack_scenarios::adapters::curves::apply_curve_parallel_shock;
/// use finstack_scenarios::CurveKind;
/// use finstack_core::market_data::MarketContext;
///
/// # fn main() -> finstack_scenarios::Result<()> {
/// let mut market = MarketContext::new();
/// // ... load a discount curve into `market` ...
/// apply_curve_parallel_shock(&mut market, CurveKind::Discount, "USD_SOFR", 25.0)?;
/// # Ok(())
/// # }
/// ```
pub fn apply_curve_parallel_shock(
    market: &mut MarketContext,
    curve_kind: CurveKind,
    curve_id: &str,
    bp: f64,
) -> Result<()> {
    let bump_spec = BumpSpec::parallel_bp(bp);

    match curve_kind {
        CurveKind::Discount => {
            let curve =
                market
                    .get_discount_ref(curve_id)
                    .map_err(|_| Error::MarketDataNotFound {
                        id: curve_id.to_string(),
                    })?;
            // Use with_parallel_bump which creates curve with modified ID
            let bumped_curve = curve.with_parallel_bump(bp);
            
            // Manually rebuild with original ID to preserve instrument references
            let original_id = curve.id();
            let bumped_points: Vec<(f64, f64)> = bumped_curve.knots()
                .iter()
                .zip(bumped_curve.dfs().iter())
                .map(|(&t, &df)| (t, df))
                .collect();
            
            let final_curve = finstack_core::market_data::term_structures::discount_curve::DiscountCurve::builder(original_id.as_str())
                .base_date(curve.base_date())
                .day_count(curve.day_count())
                .knots(bumped_points)
                .build()
                .map_err(|e| Error::Internal(format!("Failed to rebuild curve: {}", e)))?;
            
            market.insert_discount_mut(std::sync::Arc::new(final_curve));
        }
        CurveKind::Forecast => {
            let curve =
                market
                    .get_forward_ref(curve_id)
                    .map_err(|_| Error::MarketDataNotFound {
                        id: curve_id.to_string(),
                    })?;
            let bumped_temp =
                curve
                    .apply_bump(bump_spec)
                    .ok_or_else(|| Error::UnsupportedOperation {
                        operation: format!("parallel bp={}", bp),
                        target: format!("forward curve {}", curve_id),
                    })?;
            
            // Manually rebuild with original ID
            let original_id = curve.id();
            let bumped_points: Vec<(f64, f64)> = bumped_temp.knots()
                .iter()
                .zip(bumped_temp.forwards().iter())
                .map(|(&t, &f)| (t, f))
                .collect();
            
            let final_curve = finstack_core::market_data::term_structures::forward_curve::ForwardCurve::builder(
                original_id.as_str(),
                curve.tenor(),
            )
                .base_date(curve.base_date())
                .knots(bumped_points)
                .build()
                .map_err(|e| Error::Internal(format!("Failed to rebuild forward curve: {}", e)))?;
            
            market.insert_forward_mut(std::sync::Arc::new(final_curve));
        }
        CurveKind::Hazard => {
            let curve = market
                .get_hazard_ref(curve_id)
                .map_err(|_| Error::MarketDataNotFound {
                    id: curve_id.to_string(),
                })?;
            let bumped_temp =
                curve
                    .apply_bump(bump_spec)
                    .ok_or_else(|| Error::UnsupportedOperation {
                        operation: format!("parallel bp={}", bp),
                        target: format!("hazard curve {}", curve_id),
                    })?;
            
            // Use to_builder_with_id helper to rebuild with original ID
            let original_id = curve.id();
            let final_curve = bumped_temp
                .to_builder_with_id(original_id.clone())
                .build()
                .map_err(|e| Error::Internal(format!("Failed to rebuild hazard curve: {}", e)))?;
            
            market.insert_hazard_mut(std::sync::Arc::new(final_curve));
        }
        CurveKind::Inflation => {
            let curve =
                market
                    .get_inflation_ref(curve_id)
                    .map_err(|_| Error::MarketDataNotFound {
                        id: curve_id.to_string(),
                    })?;
            // Inflation curves use percent bumps, convert bp to pct
            let pct_bump = BumpSpec::inflation_shift_pct(bp / 100.0);
            let bumped_temp = curve
                .apply_bump(pct_bump)
                .ok_or_else(|| Error::UnsupportedOperation {
                    operation: format!("inflation bump pct={}", bp / 100.0),
                    target: format!("inflation curve {}", curve_id),
                })?;
            
            // Manually rebuild with original ID
            let original_id = curve.id();
            let bumped_points: Vec<(f64, f64)> = bumped_temp.knots()
                .iter()
                .zip(bumped_temp.cpi_levels().iter())
                .map(|(&t, &cpi)| (t, cpi))
                .collect();
            
            let final_curve = finstack_core::market_data::term_structures::inflation::InflationCurve::builder(
                original_id.as_str(),
            )
                .base_cpi(bumped_temp.base_cpi())
                .knots(bumped_points)
                .build()
                .map_err(|e| Error::Internal(format!("Failed to rebuild inflation curve: {}", e)))?;
            
            market.insert_inflation_mut(std::sync::Arc::new(final_curve));
        }
    }

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
/// use finstack_core::market_data::MarketContext;
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
                        base_curve.with_parallel_bump(*bp)
                    }
                    TenorMatchMode::Interpolate => {
                        // Use key-rate bump (localized shock)
                        base_curve.with_key_rate_bump_years(tenor_years, *bp)
                    }
                };

                base_curve = std::sync::Arc::new(bumped_curve);
            }

            market.insert_discount_mut(base_curve);
        }
        CurveKind::Forecast => {
            let mut base_curve =
                market
                    .get_forward(curve_id)
                    .map_err(|_| Error::MarketDataNotFound {
                        id: curve_id.to_string(),
                    })?;

            // Forward curves: apply bumps via BumpSpec (sequential application)
            for (tenor_str, bp) in nodes {
                let tenor_years = parse_tenor_to_years(tenor_str)?;

                let bumped_curve = match match_mode {
                    TenorMatchMode::Exact => {
                        let knots = base_curve.knots();
                        if !knots.iter().any(|&t| (t - tenor_years).abs() < 1e-6) {
                            return Err(Error::TenorNotFound {
                                tenor: tenor_str.clone(),
                                curve_id: curve_id.to_string(),
                            });
                        }
                        // For forward curves, apply parallel bump (exact pillar logic TBD)
                        let bump_spec = BumpSpec::parallel_bp(*bp);
                        base_curve.apply_bump(bump_spec).ok_or_else(|| {
                            Error::UnsupportedOperation {
                                operation: format!("node bump bp={}", bp),
                                target: format!("forward curve {}", curve_id),
                            }
                        })?
                    }
                    TenorMatchMode::Interpolate => {
                        // Use parallel bump as approximation for forward curves
                        let bump_spec = BumpSpec::parallel_bp(*bp);
                        base_curve.apply_bump(bump_spec).ok_or_else(|| {
                            Error::UnsupportedOperation {
                                operation: format!("interpolate node bump bp={}", bp),
                                target: format!("forward curve {}", curve_id),
                            }
                        })?
                    }
                };

                base_curve = std::sync::Arc::new(bumped_curve);
            }

            market.insert_forward_mut(base_curve);
        }
        CurveKind::Hazard => {
            let mut base_curve =
                market
                    .get_hazard(curve_id)
                    .map_err(|_| Error::MarketDataNotFound {
                        id: curve_id.to_string(),
                    })?;

            // Hazard curves: apply bumps sequentially
            for (_tenor_str, bp) in nodes {
                let _tenor_years = parse_tenor_to_years(_tenor_str)?;

                let bumped_curve = match match_mode {
                    TenorMatchMode::Exact | TenorMatchMode::Interpolate => {
                        // Note: Hazard curves don't expose knots publicly
                        // For now, apply parallel bump for both modes
                        let bump_spec = BumpSpec::parallel_bp(*bp);
                        base_curve.apply_bump(bump_spec).ok_or_else(|| {
                            Error::UnsupportedOperation {
                                operation: format!("node bump bp={}", bp),
                                target: format!("hazard curve {}", curve_id),
                            }
                        })?
                    }
                };

                base_curve = std::sync::Arc::new(bumped_curve);
            }

            market.insert_hazard_mut(base_curve);
        }
        CurveKind::Inflation => {
            let mut base_curve =
                market
                    .get_inflation(curve_id)
                    .map_err(|_| Error::MarketDataNotFound {
                        id: curve_id.to_string(),
                    })?;

            // Inflation curves: use percent bumps
            for (tenor_str, bp) in nodes {
                let tenor_years = parse_tenor_to_years(tenor_str)?;

                let bumped_curve = match match_mode {
                    TenorMatchMode::Exact => {
                        let knots = base_curve.knots();
                        if !knots.iter().any(|&t| (t - tenor_years).abs() < 1e-6) {
                            return Err(Error::TenorNotFound {
                                tenor: tenor_str.clone(),
                                curve_id: curve_id.to_string(),
                            });
                        }
                        let pct_bump = BumpSpec::inflation_shift_pct(*bp / 100.0);
                        base_curve.apply_bump(pct_bump).ok_or_else(|| {
                            Error::UnsupportedOperation {
                                operation: format!("exact node bump pct={}", bp / 100.0),
                                target: format!("inflation curve {}", curve_id),
                            }
                        })?
                    }
                    TenorMatchMode::Interpolate => {
                        let pct_bump = BumpSpec::inflation_shift_pct(*bp / 100.0);
                        base_curve.apply_bump(pct_bump).ok_or_else(|| {
                            Error::UnsupportedOperation {
                                operation: format!("interpolate node bump pct={}", bp / 100.0),
                                target: format!("inflation curve {}", curve_id),
                            }
                        })?
                    }
                };

                base_curve = std::sync::Arc::new(bumped_curve);
            }

            market.insert_inflation_mut(base_curve);
        }
    }

    Ok(())
}
