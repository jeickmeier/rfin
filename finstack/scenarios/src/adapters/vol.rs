//! Volatility surface shock adapter.
//!
//! Supports both parallel and bucketed volatility adjustments that power the
//! `OperationSpec::VolSurfaceParallelPct` and `OperationSpec::VolSurfaceBucketPct`
//! variants. The helpers rebuild the vol surface from the shocked grid so that
//! the resulting object remains Serde-friendly and deterministic.
//!
//! # Tolerances
//!
//! Bucket matching uses relative tolerances to handle various strike scales:
//! - Expiry: 2% relative tolerance (e.g., 0.5Y matches 0.49-0.51Y)
//! - Strike: 0.5% relative tolerance (e.g., 100 matches 99.5-100.5)
//!
//! # Arbitrage Detection
//!
//! After applying shocks, basic arbitrage validation is performed:
//! - Calendar spread: Total variance must be non-decreasing in expiry
//! - Positive vol: All volatilities must be positive

use crate::error::{Error, Result};
use crate::utils::parse_tenor_to_years;
use finstack_core::market_data::MarketContext;

/// Relative tolerance for expiry matching (2%)
const EXPIRY_REL_TOL: f64 = 0.02;

/// Relative tolerance for strike matching (0.5%)
const STRIKE_REL_TOL: f64 = 0.005;

/// Check if two expiries match within relative tolerance.
#[inline]
fn matches_expiry(target: f64, actual: f64) -> bool {
    let base = target.abs().max(0.01); // Avoid division by zero for very short expiries
    (target - actual).abs() < base * EXPIRY_REL_TOL
}

/// Check if two strikes match within relative tolerance.
#[inline]
fn matches_strike(target: f64, actual: f64) -> bool {
    let base = actual.abs().max(1e-6); // Avoid division by zero
    (target - actual).abs() / base < STRIKE_REL_TOL
}

/// Arbitrage violation types detected in volatility surfaces.
#[derive(Debug, Clone)]
pub enum ArbitrageViolation {
    /// Calendar spread arbitrage: total variance decreases with expiry at given strike
    CalendarSpread {
        /// Strike level where violation was detected
        strike: f64,
        /// Expiry time in years where violation was detected
        expiry: f64,
        /// Total variance at previous expiry
        prev_variance: f64,
        /// Total variance at current expiry (lower than prev, indicating arbitrage)
        curr_variance: f64,
    },
    /// Negative or zero volatility detected
    NonPositiveVol {
        /// Expiry time in years where violation was detected
        expiry: f64,
        /// Strike level where violation was detected
        strike: f64,
        /// The non-positive volatility value
        vol: f64,
    },
}

impl std::fmt::Display for ArbitrageViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArbitrageViolation::CalendarSpread {
                strike,
                expiry,
                prev_variance,
                curr_variance,
            } => write!(
                f,
                "Calendar spread arbitrage at strike={:.2}, expiry={:.4}Y: \
                 total variance decreased from {:.6} to {:.6}",
                strike, expiry, prev_variance, curr_variance
            ),
            ArbitrageViolation::NonPositiveVol {
                expiry,
                strike,
                vol,
            } => write!(
                f,
                "Non-positive vol at expiry={:.4}Y, strike={:.2}: vol={:.6}",
                expiry, strike, vol
            ),
        }
    }
}

/// Check a vol surface grid for arbitrage violations.
///
/// # Arguments
/// - `expiries`: Expiry times in years
/// - `strikes`: Strike levels
/// - `vols`: 2D grid of volatilities [expiry_idx][strike_idx]
///
/// # Returns
/// Vector of detected arbitrage violations (empty if none).
fn check_arbitrage(
    expiries: &[f64],
    strikes: &[f64],
    vols: &[Vec<f64>],
) -> Vec<ArbitrageViolation> {
    let mut violations = Vec::new();

    // Check calendar spread arbitrage for each strike
    for (strike_idx, &strike) in strikes.iter().enumerate() {
        let mut prev_var = 0.0;

        for (exp_idx, &expiry) in expiries.iter().enumerate() {
            if exp_idx >= vols.len() || strike_idx >= vols[exp_idx].len() {
                continue;
            }

            let vol = vols[exp_idx][strike_idx];

            // Check for non-positive vol
            if vol <= 0.0 {
                violations.push(ArbitrageViolation::NonPositiveVol {
                    expiry,
                    strike,
                    vol,
                });
                continue;
            }

            // Check total variance is non-decreasing
            let total_var = vol * vol * expiry;
            if total_var < prev_var - 1e-8 {
                violations.push(ArbitrageViolation::CalendarSpread {
                    strike,
                    expiry,
                    prev_variance: prev_var,
                    curr_variance: total_var,
                });
            }
            prev_var = total_var;
        }
    }

    violations
}

/// Apply a parallel percentage shock to a volatility surface.
///
/// # Arguments
/// - `market`: Market context where the target surface is stored.
/// - `surface_id`: Identifier of the volatility surface.
/// - `pct`: Percentage change to apply.
///
/// # Returns
/// `Ok(warnings)` containing any arbitrage warnings, or an error.
///
/// # Errors
/// - [`Error::MarketDataNotFound`](crate::error::Error::MarketDataNotFound) if
///   the surface cannot be found.
///
/// # Examples
/// ```rust,no_run
/// use finstack_scenarios::adapters::vol::apply_vol_parallel_shock;
/// use finstack_core::market_data::MarketContext;
///
/// # fn main() -> finstack_scenarios::Result<()> {
/// let mut market = MarketContext::new();
/// // ... insert a surface ...
/// let warnings = apply_vol_parallel_shock(&mut market, "SPX_VOL", 12.5)?;
/// for w in warnings {
///     eprintln!("Warning: {}", w);
/// }
/// # Ok(())
/// # }
/// ```
pub fn apply_vol_parallel_shock(
    market: &mut MarketContext,
    surface_id: &str,
    pct: f64,
) -> Result<Vec<String>> {
    let surface = market
        .surface_ref(surface_id)
        .map_err(|_| Error::MarketDataNotFound {
            id: surface_id.to_string(),
        })?;

    // Apply multiplicative shock to all vols and rebuild surface
    let factor = 1.0 + (pct / 100.0);
    let expiries = surface.expiries().to_vec();
    let strikes = surface.strikes().to_vec();
    let (n_expiries, n_strikes) = surface.grid_shape();

    let mut vols: Vec<Vec<f64>> = Vec::with_capacity(n_expiries);
    let mut builder = finstack_core::market_data::surfaces::vol_surface::VolSurface::builder(
        surface.id().as_str(),
    )
    .expiries(&expiries)
    .strikes(&strikes);

    for &expiry in expiries.iter().take(n_expiries) {
        let mut row = Vec::with_capacity(n_strikes);
        for &strike in strikes.iter().take(n_strikes) {
            let val = surface.value(expiry, strike);
            let shocked = (val * factor).max(1e-6); // Ensure positive
            row.push(shocked);
        }
        vols.push(row.clone());
        builder = builder.row(&row);
    }

    // Check for arbitrage violations
    let violations = check_arbitrage(&expiries, &strikes, &vols);
    let warnings: Vec<String> = violations.iter().map(|v| v.to_string()).collect();

    let bumped = builder
        .build()
        .map_err(|e| Error::Internal(format!("Failed to rebuild vol surface: {}", e)))?;

    market.insert_surface_mut(std::sync::Arc::new(bumped));
    Ok(warnings)
}

/// Apply a bucketed percentage shock to a volatility surface.
///
/// Only buckets matching the provided tenor and strike filters are shocked; all
/// others remain unchanged. Filters are optional, allowing callers to target
/// just the tenor dimension, just the strike dimension, or both together.
///
/// Uses relative tolerances for matching:
/// - Expiry: 2% relative tolerance
/// - Strike: 0.5% relative tolerance
///
/// # Arguments
/// - `market`: Market context being mutated.
/// - `surface_id`: Identifier of the volatility surface.
/// - `tenors`: Optional slice of tenor strings to match (e.g., `["1M", "3M"]`).
/// - `strikes`: Optional slice of strikes to match (absolute, not relative).
/// - `pct`: Percentage change to apply to matching buckets.
///
/// # Returns
/// `Ok(warnings)` containing any arbitrage warnings, or an error.
///
/// # Errors
/// - [`Error::MarketDataNotFound`](crate::error::Error::MarketDataNotFound)
///   when the surface cannot be located.
/// - [`Error::InvalidTenor`](crate::error::Error::InvalidTenor) if tenor
///   strings cannot be parsed.
/// - [`Error::Internal`](crate::error::Error::Internal) if rebuilding the
///   updated surface fails validation.
///
/// # Examples
/// ```rust,no_run
/// use finstack_scenarios::adapters::vol::apply_vol_bucket_shock;
/// use finstack_core::market_data::MarketContext;
///
/// # fn main() -> finstack_scenarios::Result<()> {
/// let mut market = MarketContext::new();
/// // ... insert a surface ...
/// let warnings = apply_vol_bucket_shock(
///     &mut market,
///     "SPX_VOL",
///     Some(&["1M".into(), "3M".into()]),
///     Some(&[90.0, 100.0]),
///     15.0,
/// )?;
/// # Ok(())
/// # }
/// ```
pub fn apply_vol_bucket_shock(
    market: &mut MarketContext,
    surface_id: &str,
    tenors: Option<&[String]>,
    strikes: Option<&[f64]>,
    pct: f64,
) -> Result<Vec<String>> {
    let surface = market
        .surface_ref(surface_id)
        .map_err(|_| Error::MarketDataNotFound {
            id: surface_id.to_string(),
        })?;

    // If no filters specified, apply parallel shock
    if tenors.is_none() && strikes.is_none() {
        return apply_vol_parallel_shock(market, surface_id, pct);
    }

    // Parse tenor filters to years
    let tenor_years: Option<Vec<f64>> = if let Some(tenor_strs) = tenors {
        let years: Result<Vec<f64>> = tenor_strs.iter().map(|s| parse_tenor_to_years(s)).collect();
        Some(years?)
    } else {
        None
    };

    // Get surface data
    let expiries = surface.expiries().to_vec();
    let strikes_vec = surface.strikes().to_vec();
    let (n_expiries, n_strikes) = surface.grid_shape();

    // Apply shock selectively
    let factor = 1.0 + (pct / 100.0);
    let mut vols: Vec<Vec<f64>> = Vec::with_capacity(n_expiries);
    let mut builder = finstack_core::market_data::surfaces::vol_surface::VolSurface::builder(
        surface.id().as_str(),
    )
    .expiries(&expiries)
    .strikes(&strikes_vec);

    for &expiry in expiries.iter().take(n_expiries) {
        let mut row = Vec::with_capacity(n_strikes);
        for &strike in strikes_vec.iter().take(n_strikes) {
            let val = surface.value(expiry, strike);

            // Check if this bucket matches filters using relative tolerances
            let tenor_match = tenor_years
                .as_ref()
                .is_none_or(|tenors| tenors.iter().any(|&t| matches_expiry(t, expiry)));

            let strike_match = strikes.is_none_or(|strike_filters| {
                strike_filters.iter().any(|&s| matches_strike(s, strike))
            });

            // Apply shock only if both filters match
            let shocked = if tenor_match && strike_match {
                (val * factor).max(1e-6) // Ensure positive
            } else {
                val
            };
            row.push(shocked);
        }
        vols.push(row.clone());
        builder = builder.row(&row);
    }

    // Check for arbitrage violations
    let violations = check_arbitrage(&expiries, &strikes_vec, &vols);
    let warnings: Vec<String> = violations.iter().map(|v| v.to_string()).collect();

    let bumped = builder
        .build()
        .map_err(|e| Error::Internal(format!("Failed to rebuild vol surface: {}", e)))?;

    market.insert_surface_mut(std::sync::Arc::new(bumped));
    Ok(warnings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matches_expiry() {
        // 1Y should match within 2%
        assert!(matches_expiry(1.0, 1.0));
        assert!(matches_expiry(1.0, 1.01)); // 1% diff
        assert!(matches_expiry(1.0, 0.99)); // 1% diff
        assert!(!matches_expiry(1.0, 1.05)); // 5% diff
    }

    #[test]
    fn test_matches_strike() {
        // 100 should match within 0.5%
        assert!(matches_strike(100.0, 100.0));
        assert!(matches_strike(100.0, 100.4)); // 0.4% diff
        assert!(!matches_strike(100.0, 101.0)); // 1% diff
    }

    #[test]
    fn test_arbitrage_detection_calendar_spread() {
        // Create a surface with calendar spread arbitrage
        // Vol decreases with expiry (arbitrage)
        let expiries = vec![0.25, 0.5, 1.0];
        let strikes = vec![100.0];
        let vols = vec![
            vec![0.3],  // 0.25Y: var = 0.3^2 * 0.25 = 0.0225
            vec![0.2],  // 0.5Y: var = 0.2^2 * 0.5 = 0.02 < 0.0225 (arbitrage!)
            vec![0.15], // 1Y: var = 0.15^2 * 1.0 = 0.0225
        ];

        let violations = check_arbitrage(&expiries, &strikes, &vols);
        assert!(!violations.is_empty());
        assert!(matches!(
            &violations[0],
            ArbitrageViolation::CalendarSpread { .. }
        ));
    }

    #[test]
    fn test_arbitrage_detection_non_positive() {
        let expiries = vec![0.5];
        let strikes = vec![100.0, 110.0];
        let vols = vec![vec![0.2, -0.1]]; // Negative vol

        let violations = check_arbitrage(&expiries, &strikes, &vols);
        assert!(!violations.is_empty());
        assert!(matches!(
            &violations[0],
            ArbitrageViolation::NonPositiveVol { .. }
        ));
    }

    #[test]
    fn test_no_arbitrage_clean_surface() {
        // Create a clean surface with no arbitrage
        let expiries = vec![0.25, 0.5, 1.0];
        let strikes = vec![90.0, 100.0, 110.0];
        let vols = vec![
            vec![0.25, 0.20, 0.22], // 0.25Y
            vec![0.24, 0.19, 0.21], // 0.5Y (variance increases)
            vec![0.22, 0.18, 0.20], // 1Y (variance increases)
        ];

        let violations = check_arbitrage(&expiries, &strikes, &vols);
        assert!(violations.is_empty());
    }
}
