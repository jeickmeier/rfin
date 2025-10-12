//! Base correlation shock adapter.

use crate::error::{Error, Result};
use finstack_core::market_data::bumps::{BumpSpec, Bumpable};
use finstack_core::market_data::MarketContext;

/// Apply parallel point shock to a base correlation surface.
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

    // Convert points to percent for BumpSpec (e.g., 0.05 points = 5%)
    let pct = points * 100.0;
    let bump_spec = BumpSpec::correlation_shift_pct(pct);

    let bumped = curve
        .apply_bump(bump_spec)
        .ok_or_else(|| Error::UnsupportedOperation {
            operation: format!("correlation shift points={}", points),
            target: format!("base correlation {}", surface_id),
        })?;

    market.insert_base_correlation_mut(std::sync::Arc::new(bumped));
    Ok(())
}

/// Apply bucket-specific point shock to base correlation surface.
///
/// Only shocks buckets that match the detachment point filters.
/// Unmatched buckets remain unchanged.
///
/// Note: The `maturities` filter is applied via curve ID matching.
/// For simplicity, this implementation filters only by detachment points within a curve.
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
        let matches = target_detachments.as_ref().map_or(true, |targets| {
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
