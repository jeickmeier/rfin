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
    let curve = market.get_base_correlation_ref(surface_id).map_err(|_| {
        Error::MarketDataNotFound {
            id: surface_id.to_string(),
        }
    })?;

    // Convert points to percent for BumpSpec (e.g., 0.05 points = 5%)
    let pct = points * 100.0;
    let bump_spec = BumpSpec::correlation_shift_pct(pct);

    let bumped = curve.apply_bump(bump_spec).ok_or_else(|| {
        Error::UnsupportedOperation {
            operation: format!("correlation shift points={}", points),
            target: format!("base correlation {}", surface_id),
        }
    })?;

    market.insert_base_correlation_mut(std::sync::Arc::new(bumped));
    Ok(())
}

/// Apply bucket-specific point shock to base correlation surface.
///
/// Phase A: applies parallel shock (bucket filtering not yet implemented).
pub fn apply_basecorr_bucket_shock(
    market: &mut MarketContext,
    surface_id: &str,
    _detachment_bps: Option<&[i32]>,
    _maturities: Option<&[String]>,
    points: f64,
) -> Result<()> {
    // Phase A: apply parallel shock (simplified)
    // TODO: implement bucket-specific filtering by detachment/maturity
    apply_basecorr_parallel_shock(market, surface_id, points)
}

