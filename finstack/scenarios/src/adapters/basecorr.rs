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

use crate::error::{Error, Result};
use finstack_core::market_data::context::MarketContext;

/// Apply a parallel point shock to a base correlation surface.
///
/// # Arguments
/// - `market`: Market context containing the surface to update.
/// - `surface_id`: Identifier of the base correlation surface.
/// - `points`: Additive change to apply, expressed in absolute correlation points.
///
/// # Returns
/// `Ok(warnings)` containing any clamping warnings, or an error.
///
/// # Clamping
///
/// Correlation values are clamped to [0, 1]. If any values are clamped,
/// warnings are returned in the result.
///
/// # Errors
/// - [`Error::MarketDataNotFound`](crate::error::Error::MarketDataNotFound) if
///   the surface cannot be located.
/// - [`Error::Internal`](crate::error::Error::Internal) if rebuilding fails.
///
/// # Examples
/// ```rust,no_run
/// use finstack_scenarios::adapters::basecorr::apply_basecorr_parallel_shock;
/// use finstack_core::market_data::context::MarketContext;
///
/// # fn main() -> finstack_scenarios::Result<()> {
/// let mut market = MarketContext::new();
/// // ... load a base correlation surface ...
/// let warnings = apply_basecorr_parallel_shock(&mut market, "CDX_IG", 0.05)?;
/// for w in warnings {
///     eprintln!("Warning: {}", w);
/// }
/// # Ok(())
/// # }
/// ```
pub fn apply_basecorr_parallel_shock(
    market: &mut MarketContext,
    surface_id: &str,
    points: f64,
) -> Result<Vec<String>> {
    let curve =
        market
            .get_base_correlation_ref(surface_id)
            .map_err(|_| Error::MarketDataNotFound {
                id: surface_id.to_string(),
            })?;

    // Apply additive point shock to all detachment points
    let detachment_points = curve.detachment_points().to_vec();
    let correlations = curve.correlations().to_vec();

    let mut warnings = Vec::new();
    let mut clamped_low = 0;
    let mut clamped_high = 0;

    let shocked_points: Vec<(f64, f64)> = detachment_points
        .into_iter()
        .zip(correlations)
        .map(|(d, c)| {
            let raw = c + points;
            let clamped = raw.clamp(0.0, 1.0);

            if raw < 0.0 {
                clamped_low += 1;
            } else if raw > 1.0 {
                clamped_high += 1;
            }

            (d, clamped)
        })
        .collect();

    // Generate warnings for clamping
    if clamped_low > 0 {
        warnings.push(format!(
            "Base correlation clamped to 0 for {} detachment point(s) in surface '{}' \
             (shock {} would have resulted in negative correlation)",
            clamped_low, surface_id, points
        ));
    }
    if clamped_high > 0 {
        warnings.push(format!(
            "Base correlation clamped to 1 for {} detachment point(s) in surface '{}' \
             (shock {} would have resulted in correlation > 1)",
            clamped_high, surface_id, points
        ));
    }

    let bumped = finstack_core::market_data::term_structures::base_correlation::BaseCorrelationCurve::builder(
        curve.id().as_str()
    )
        .knots(shocked_points)
        .build()
        .map_err(|e| {
            Error::Internal(format!("Failed to rebuild base correlation curve: {}", e))
        })?;

    market.insert_base_correlation_mut(std::sync::Arc::new(bumped));
    Ok(warnings)
}

/// Apply a bucket-specific point shock to a base correlation surface.
///
/// Only detachment points that match the optional filters are adjusted; other
/// detachment points are left untouched.
///
/// # Arguments
/// - `market`: Market context containing the surface to update.
/// - `surface_id`: Identifier of the base correlation surface.
/// - `detachment_bps`: Optional detachment points (in basis points) to target.
/// - `_maturities`: Placeholder for maturity filters (currently ignored).
/// - `points`: Additive change to apply to matching buckets.
///
/// # Returns
/// `Ok(warnings)` containing any clamping warnings, or an error.
///
/// # Clamping
///
/// Correlation values are clamped to [0, 1]. If any values are clamped,
/// warnings are returned in the result.
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
/// use finstack_core::market_data::context::MarketContext;
///
/// # fn main() -> finstack_scenarios::Result<()> {
/// let mut market = MarketContext::new();
/// // ... load a base correlation surface ...
/// let warnings = apply_basecorr_bucket_shock(
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
) -> Result<Vec<String>> {
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

    let mut warnings = Vec::new();
    let mut clamped_low = 0;
    let mut clamped_high = 0;

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
            let raw = correlation + points;
            let new_corr = raw.clamp(0.0, 1.0);

            if raw < 0.0 {
                clamped_low += 1;
            } else if raw > 1.0 {
                clamped_high += 1;
            }

            new_points.push((detachment, new_corr));
        } else {
            // Keep unchanged
            new_points.push((detachment, correlation));
        }
    }

    // Generate warnings for clamping
    if clamped_low > 0 {
        warnings.push(format!(
            "Base correlation clamped to 0 for {} detachment point(s) in surface '{}' \
             (bucket shock {} would have resulted in negative correlation)",
            clamped_low, surface_id, points
        ));
    }
    if clamped_high > 0 {
        warnings.push(format!(
            "Base correlation clamped to 1 for {} detachment point(s) in surface '{}' \
             (bucket shock {} would have resulted in correlation > 1)",
            clamped_high, surface_id, points
        ));
    }

    // Rebuild curve
    let bumped = finstack_core::market_data::term_structures::base_correlation::BaseCorrelationCurve::builder(
        curve.id().as_str()
    )
        .knots(new_points)
        .build()
        .map_err(|e| {
            Error::Internal(format!("Failed to rebuild base correlation curve: {}", e))
        })?;

    market.insert_base_correlation_mut(std::sync::Arc::new(bumped));
    Ok(warnings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::market_data::term_structures::base_correlation::BaseCorrelationCurve;

    fn setup_market() -> MarketContext {
        let curve = BaseCorrelationCurve::builder("TEST_BC")
            .knots(vec![
                (0.03, 0.2),  // 3% detachment -> 20% correlation
                (0.07, 0.35), // 7% detachment -> 35% correlation
                (0.10, 0.45), // 10% detachment -> 45% correlation
                (0.15, 0.60), // 15% detachment -> 60% correlation
            ])
            .build()
            .expect("valid base corr curve");

        MarketContext::new().insert_base_correlation(curve)
    }

    #[test]
    fn test_parallel_shock_no_clamping() {
        let mut market = setup_market();
        let warnings = apply_basecorr_parallel_shock(&mut market, "TEST_BC", 0.05)
            .expect("shock should succeed");

        assert!(warnings.is_empty(), "No clamping should occur");
    }

    #[test]
    fn test_parallel_shock_with_clamping_high() {
        let mut market = setup_market();
        // Shock +0.6 will push 60% correlation to 120%, which must be clamped
        let warnings = apply_basecorr_parallel_shock(&mut market, "TEST_BC", 0.6)
            .expect("shock should succeed");

        assert!(!warnings.is_empty(), "Should have clamping warnings");
        assert!(warnings[0].contains("clamped to 1"));
    }

    #[test]
    fn test_parallel_shock_with_clamping_low() {
        let mut market = setup_market();
        // Shock -0.5 will push 20% correlation to -30%, which must be clamped
        let warnings = apply_basecorr_parallel_shock(&mut market, "TEST_BC", -0.5)
            .expect("shock should succeed");

        assert!(!warnings.is_empty(), "Should have clamping warnings");
        assert!(warnings[0].contains("clamped to 0"));
    }
}
