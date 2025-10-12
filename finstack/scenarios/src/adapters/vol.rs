//! Volatility surface shock adapter.

use crate::error::{Error, Result};
use finstack_core::market_data::MarketContext;

/// Apply parallel percent shock to a volatility surface.
pub fn apply_vol_parallel_shock(
    market: &mut MarketContext,
    surface_id: &str,
    pct: f64,
) -> Result<()> {
    let surface = market.surface_ref(surface_id).map_err(|_| {
        Error::MarketDataNotFound {
            id: surface_id.to_string(),
        }
    })?;

    // Clone and apply multiplicative shock to all vols
    let factor = 1.0 + (pct / 100.0);
    let bumped = surface.clone_with_shock(factor);

    market.insert_surface_mut(std::sync::Arc::new(bumped));
    Ok(())
}

/// Apply bucketed percent shock to a volatility surface.
///
/// Phase A: applies parallel shock (bucket filtering not yet implemented).
pub fn apply_vol_bucket_shock(
    market: &mut MarketContext,
    surface_id: &str,
    _tenors: Option<&[String]>,
    _strikes: Option<&[f64]>,
    pct: f64,
) -> Result<()> {
    // Phase A: apply parallel shock (simplified)
    // TODO: implement bucket-specific filtering by tenor/strike
    apply_vol_parallel_shock(market, surface_id, pct)
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

