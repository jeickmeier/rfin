//! Base correlation shock adapter.
//!
//! Contains helpers for both parallel and bucketed base correlation shocks,
//! used by the base-correlation `OperationSpec` variants.

use crate::error::{Error, Result};
use finstack_core::market_data::MarketContext;

/// Apply a parallel point shock to a base correlation surface.
///
/// # Arguments
/// - `market`: Market context containing the surface to update.
/// - `surface_id`: Identifier of the base correlation surface.
/// - `points`: Additive change to apply, expressed in absolute correlation points.
///
/// # Returns
/// [`Result`](crate::error::Result) with `Ok(())` on success.
///
/// # Errors
/// - [`Error::MarketDataNotFound`](crate::error::Error::MarketDataNotFound) if
///   the surface cannot be located.
/// - [`Error::UnsupportedOperation`](crate::error::Error::UnsupportedOperation)
///   if the underlying surface cannot be bumped.
///
/// # Examples
/// ```rust,no_run
/// use finstack_scenarios::adapters::basecorr::apply_basecorr_parallel_shock;
/// use finstack_core::market_data::MarketContext;
///
/// # fn main() -> finstack_scenarios::Result<()> {
/// let mut market = MarketContext::new();
/// // ... load a base correlation surface ...
/// apply_basecorr_parallel_shock(&mut market, "CDX_IG", 0.05)?;
/// # Ok(())
/// # }
/// ```
pub fn apply_basecorr_parallel_shock(
    market: &mut MarketContext,
    surface_id: &str,
    points: f64,
) -> Result<()> {
    let curve =
        market
            .get_base_correlation_ref(surface_id)
            .map_err(|_| Error::MarketDataNotFound {
                id: surface_id.to_string(),
            })?;

    // Apply additive point shock to all detachment points
    let detachment_points = curve.detachment_points().to_vec();
    let correlations = curve.correlations().to_vec();

    let shocked_points: Vec<(f64, f64)> = detachment_points
        .into_iter()
        .zip(correlations)
        .map(|(d, c)| (d, (c + points).clamp(0.0, 1.0)))
        .collect();

    let bumped = finstack_core::market_data::term_structures::base_correlation::BaseCorrelationCurve::builder(
        curve.id().as_str()
    )
        .points(shocked_points)
        .build()
        .map_err(|e| {
            Error::Internal(format!("Failed to rebuild base correlation curve: {}", e))
        })?;

    market.insert_base_correlation_mut(std::sync::Arc::new(bumped));
    Ok(())
}

/// Apply a bucket-specific point shock to a base correlation surface.
///
/// Only detachment points that match the optional filters are adjusted; other
/// detachment points are left untouched. The `maturities` parameter is currently
/// unused, but kept for future compatibility with more granular filtering.
///
/// # Arguments
/// - `market`: Market context containing the surface to update.
/// - `surface_id`: Identifier of the base correlation surface.
/// - `detachment_bps`: Optional detachment points (in basis points) to target.
/// - `_maturities`: Placeholder for maturity filters (currently ignored).
/// - `points`: Additive change to apply to matching buckets.
///
/// # Returns
/// [`Result`](crate::error::Result) with `Ok(())` on success.
///
/// # Errors
/// - [`Error::MarketDataNotFound`](crate::error::Error::MarketDataNotFound) if
///   the surface cannot be found.
/// - [`Error::Internal`](crate::error::Error::Internal) if rebuilding the
///   surface fails.
///
/// # Examples
/// ```rust,no_run
/// use finstack_scenarios::adapters::basecorr::apply_basecorr_bucket_shock;
/// use finstack_core::market_data::MarketContext;
///
/// # fn main() -> finstack_scenarios::Result<()> {
/// let mut market = MarketContext::new();
/// // ... load a base correlation surface ...
/// apply_basecorr_bucket_shock(
///     &mut market,
///     "CDX_IG",
///     Some(&[300, 700]),
///     None,
///     0.02,
/// )?;
/// # Ok(())
/// # }
/// ```
pub fn apply_basecorr_bucket_shock(
    market: &mut MarketContext,
    surface_id: &str,
    detachment_bps: Option<&[i32]>,
    _maturities: Option<&[String]>,
    points: f64,
) -> Result<()> {
    // If no filters specified, apply parallel shock
    if detachment_bps.is_none() {
        return apply_basecorr_parallel_shock(market, surface_id, points);
    }

    let curve =
        market
            .get_base_correlation_ref(surface_id)
            .map_err(|_| Error::MarketDataNotFound {
                id: surface_id.to_string(),
            })?;

    // Convert detachment_bps to percentages (e.g., 300 bp = 3%)
    let target_detachments: Option<Vec<f64>> =
        detachment_bps.map(|bps| bps.iter().map(|&bp| bp as f64 / 100.0).collect());

    // Get curve data
    let detachment_points = curve.detachment_points().to_vec();
    let correlations = curve.correlations().to_vec();

    // Apply shock selectively
    let mut new_points: Vec<(f64, f64)> = Vec::with_capacity(detachment_points.len());

    for (i, &detachment) in detachment_points.iter().enumerate() {
        let correlation = correlations[i];

        // Check if this detachment point matches filter
        let matches = target_detachments.as_ref().is_none_or(|targets| {
            targets.iter().any(|&t| (t - detachment).abs() < 0.01) // 0.01% tolerance
        });

        if matches {
            // Apply shock (additive)
            let new_corr = (correlation + points).clamp(0.0, 1.0);
            new_points.push((detachment, new_corr));
        } else {
            // Keep unchanged
            new_points.push((detachment, correlation));
        }
    }

    // Rebuild curve
    let bumped = finstack_core::market_data::term_structures::base_correlation::BaseCorrelationCurve::builder(
        curve.id().as_str()
    )
        .points(new_points)
        .build()
        .map_err(|e| {
            Error::Internal(format!("Failed to rebuild base correlation curve: {}", e))
        })?;

    market.insert_base_correlation_mut(std::sync::Arc::new(bumped));
    Ok(())
}
