//! Vanna calculator for equity options.
//!
//! Computes vanna (∂²V/∂S∂σ) using finite differences.
//! Vanna measures how delta changes with volatility.
//!
//! Vanna ≈ (Delta(σ+h) - Delta(σ-h)) / (2h)
//!
//! Where Delta(σ) is computed by bumping both spot and vol.

use crate::instruments::equity_option::EquityOption;
use crate::metrics::scale_surface;
use crate::metrics::{bump_scalar_price, bump_sizes};
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Vanna calculator for equity options.
pub struct VannaCalculator;

impl MetricCalculator for VannaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &EquityOption = context.instrument_as()?;
        let as_of = context.as_of;

        // Check if expired
        let t = option.day_count.year_fraction(
            as_of,
            option.expiry,
            finstack_core::dates::DayCountCtx::default(),
        )?;
        if t <= 0.0 {
            return Ok(0.0);
        }

        // Get current spot
        let spot_scalar = context.curves.price(&option.spot_id)?;
        let current_spot = match spot_scalar {
            finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
            finstack_core::market_data::scalars::MarketScalar::Price(m) => m.amount(),
        };

        let spot_bump = current_spot * bump_sizes::SPOT;
        let vol_bump_pct = bump_sizes::VOLATILITY;

        // Determine the effective (ATM-ish) volatility so we can convert the relative bump
        // into an absolute Δσ and compute ∂Δ/∂σ (not ∂Δ/∂scale).
        let sigma = context
            .curves
            .surface_ref(option.vol_surface_id.as_str())
            .map(|surf| surf.value_clamped(t, option.strike.amount()))
            .unwrap_or(0.0);
        let vol_bump_abs = (sigma * vol_bump_pct).abs();
        if vol_bump_abs < 1e-12 {
            return Ok(0.0);
        }

        // Compute delta at vol_up: bump both spot and vol, compute delta
        let curves_vol_up = scale_surface(
            &context.curves,
            option.vol_surface_id.as_str(),
            1.0 + vol_bump_pct,
        )?;

        // Delta at vol_up: (PV(S+h, σ+h) - PV(S-h, σ+h)) / (2h_S)
        let curves_up_vol_up =
            bump_scalar_price(&curves_vol_up, &option.spot_id, bump_sizes::SPOT)?;
        let pv_up_vol_up = option.npv(&curves_up_vol_up, as_of)?.amount();
        let curves_down_vol_up =
            bump_scalar_price(&curves_vol_up, &option.spot_id, -bump_sizes::SPOT)?;
        let pv_down_vol_up = option.npv(&curves_down_vol_up, as_of)?.amount();
        let delta_vol_up = (pv_up_vol_up - pv_down_vol_up) / (2.0 * spot_bump);

        // Compute delta at vol_down
        let curves_vol_down = scale_surface(
            &context.curves,
            option.vol_surface_id.as_str(),
            1.0 - vol_bump_pct,
        )?;

        // Delta at vol_down: (PV(S+h, σ-h) - PV(S-h, σ-h)) / (2h_S)
        let curves_up_vol_down =
            bump_scalar_price(&curves_vol_down, &option.spot_id, bump_sizes::SPOT)?;
        let pv_up_vol_down = option.npv(&curves_up_vol_down, as_of)?.amount();
        let curves_down_vol_down =
            bump_scalar_price(&curves_vol_down, &option.spot_id, -bump_sizes::SPOT)?;
        let pv_down_vol_down = option.npv(&curves_down_vol_down, as_of)?.amount();
        let delta_vol_down = (pv_up_vol_down - pv_down_vol_down) / (2.0 * spot_bump);

        // Vanna = ∂Δ/∂σ ≈ (Δ(σ+Δσ) - Δ(σ-Δσ)) / (2Δσ)
        //
        // We bump the surface multiplicatively (relative) but divide by the corresponding
        // absolute volatility change Δσ = σ × bump_pct to keep the definition standard.
        let vanna = (delta_vol_up - delta_vol_down) / (2.0 * vol_bump_abs);

        Ok(vanna)
    }
}
