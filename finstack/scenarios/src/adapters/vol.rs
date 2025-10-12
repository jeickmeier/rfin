//! Volatility surface shock adapter.

use crate::error::{Error, Result};
use crate::utils::parse_tenor_to_years;
use finstack_core::market_data::MarketContext;

/// Apply parallel percent shock to a volatility surface.
pub fn apply_vol_parallel_shock(
    market: &mut MarketContext,
    surface_id: &str,
    pct: f64,
) -> Result<()> {
    let surface = market
        .surface_ref(surface_id)
        .map_err(|_| Error::MarketDataNotFound {
            id: surface_id.to_string(),
        })?;

    // Clone and apply multiplicative shock to all vols
    let factor = 1.0 + (pct / 100.0);
    let bumped = surface.clone_with_shock(factor);

    market.insert_surface_mut(std::sync::Arc::new(bumped));
    Ok(())
}

/// Apply bucketed percent shock to a volatility surface.
///
/// Only shocks buckets that match the tenor and/or strike filters.
/// Unmatched buckets remain unchanged.
pub fn apply_vol_bucket_shock(
    market: &mut MarketContext,
    surface_id: &str,
    tenors: Option<&[String]>,
    strikes: Option<&[f64]>,
    pct: f64,
) -> Result<()> {
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
    let mut builder = finstack_core::market_data::surfaces::vol_surface::VolSurface::builder(
        surface.id().as_str(),
    )
    .expiries(&expiries)
    .strikes(&strikes_vec);

    for &expiry in expiries.iter().take(n_expiries) {
        let mut row = Vec::with_capacity(n_strikes);
        for &strike in strikes_vec.iter().take(n_strikes) {
            let val = surface.value(expiry, strike);

            // Check if this bucket matches filters
            let tenor_match = tenor_years.as_ref().map_or(true, |tenors| {
                tenors.iter().any(|&t| (t - expiry).abs() < 0.01) // 0.01 year tolerance
            });

            let strike_match = strikes.map_or(true, |strike_filters| {
                strike_filters.iter().any(|&s| (s - strike).abs() < 0.01) // 0.01 strike tolerance
            });

            // Apply shock only if both filters match
            if tenor_match && strike_match {
                row.push(val * factor);
            } else {
                row.push(val);
            }
        }
        builder = builder.row(&row);
    }

    let bumped = builder
        .build()
        .map_err(|e| Error::Internal(format!("Failed to rebuild vol surface: {}", e)))?;

    market.insert_surface_mut(std::sync::Arc::new(bumped));
    Ok(())
}

// Extension trait for VolSurface cloning with shock
trait VolSurfaceShock {
    fn clone_with_shock(&self, factor: f64) -> Self;
}

impl VolSurfaceShock for finstack_core::market_data::surfaces::vol_surface::VolSurface {
    fn clone_with_shock(&self, factor: f64) -> Self {
        // Access internal data and apply shock
        let expiries = self.expiries().to_vec();
        let strikes = self.strikes().to_vec();
        let (n_expiries, n_strikes) = self.grid_shape();

        // Rebuild surface with shocked vols
        let mut builder = Self::builder(self.id().as_str())
            .expiries(&expiries)
            .strikes(&strikes);

        // Apply shock row by row
        for &expiry in expiries.iter().take(n_expiries) {
            let mut row = Vec::with_capacity(n_strikes);
            for &strike in strikes.iter().take(n_strikes) {
                let val = self.value(expiry, strike);
                row.push(val * factor);
            }
            builder = builder.row(&row);
        }

        builder.build().expect("Failed to rebuild vol surface")
    }
}
