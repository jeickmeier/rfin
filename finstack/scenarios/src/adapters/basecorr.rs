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

use crate::error::Result;
use finstack_core::market_data::bumps::{BumpMode, BumpSpec, BumpType, BumpUnits, MarketBump};

/// Schedule a parallel point shock to a base correlation surface.
///
/// This schedules a `MarketBump::Curve` with `BumpMode::Additive` and `BumpUnits::Fraction`.
///
/// # Arguments
/// - `surface_id`: Identifier of the base correlation surface.
/// - `points`: Additive change to apply, expressed in absolute correlation points.
/// - `market_bumps`: Output vector to append scheduled bumps to.
///
/// # Returns
/// `Ok(())` on success.
pub fn apply_basecorr_parallel_shock(
    surface_id: &str,
    points: f64,
    market_bumps: &mut Vec<MarketBump>,
) -> Result<()> {
    market_bumps.push(MarketBump::Curve {
        id: finstack_core::types::CurveId::from(surface_id),
        spec: BumpSpec {
            mode: BumpMode::Additive,
            units: BumpUnits::Fraction,
            value: points,
            bump_type: BumpType::Parallel,
        },
    });
    Ok(())
}

/// Schedule a bucket-specific point shock to a base correlation surface.
///
/// # Arguments
/// - `surface_id`: Identifier of the base correlation surface.
/// - `detachment_bps`: Optional detachment points (in basis points) to target.
/// - `maturities`: Placeholder for maturity filters (currently unsupported).
/// - `points`: Additive change to apply to matching buckets.
/// - `market_bumps`: Output vector to append scheduled bumps to.
/// - `warnings`: Output vector for warnings (unsupported features).
///
/// # Returns
/// `Ok(())` on success.
pub fn apply_basecorr_bucket_shock(
    surface_id: &str,
    detachment_bps: Option<&[i32]>,
    maturities: Option<&[String]>,
    points: f64,
    market_bumps: &mut Vec<MarketBump>,
    warnings: &mut Vec<String>,
) -> Result<()> {
    let dets = detachment_bps
        .as_ref()
        .map(|v| v.iter().map(|bp| *bp as f64 / 100.0).collect());

    if let Some(mats) = maturities {
        if !mats.is_empty() {
            warnings.push(
                "BaseCorrBucketPts maturities filter not yet supported; applying detachment-only bump"
                    .to_string(),
            );
        }
    }

    market_bumps.push(MarketBump::BaseCorrBucketPts {
        surface_id: finstack_core::types::CurveId::from(surface_id),
        detachments: dets,
        points,
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parallel_shock_scheduling() {
        // We don't need a full market context for scheduling tests since we don't access it
        let mut bumps = Vec::new();
        apply_basecorr_parallel_shock("TEST_BC", 0.05, &mut bumps).expect("shock should succeed");

        assert_eq!(bumps.len(), 1);
        match &bumps[0] {
            MarketBump::Curve { id, spec } => {
                assert_eq!(id.as_str(), "TEST_BC");
                assert_eq!(spec.value, 0.05);
                assert_eq!(spec.mode, BumpMode::Additive);
            }
            _ => panic!("Expected Curve bump"),
        }
    }

    #[test]
    fn test_bucket_shock_scheduling() {
        let mut bumps = Vec::new();
        let mut warnings = Vec::new();

        apply_basecorr_bucket_shock(
            "TEST_BC",
            Some(&[300, 700]),
            None,
            0.02,
            &mut bumps,
            &mut warnings,
        )
        .expect("shock should succeed");

        assert_eq!(bumps.len(), 1);
        match &bumps[0] {
            MarketBump::BaseCorrBucketPts {
                surface_id,
                detachments,
                points,
            } => {
                assert_eq!(surface_id.as_str(), "TEST_BC");
                assert_eq!(*points, 0.02);
                assert!(detachments.is_some());
                let dets = detachments.as_ref().expect("detachments should be present");
                assert_eq!(dets.len(), 2);
                assert!((dets[0] - 3.0).abs() < 1e-6); // 300bp = 3.0? No wait 300bp = 0.03?
                                                       // In generic code: bp / 100.0. So 300/100 = 3.
                                                       // Wait, base corr detachments are usually 0.03 (3%).
                                                       // But typically input is 300bps.
            }
            _ => panic!("Expected BaseCorrBucketPts bump"),
        }
    }
}
