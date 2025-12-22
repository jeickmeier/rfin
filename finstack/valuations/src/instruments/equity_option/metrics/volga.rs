//! Volga calculator for equity options.
//!
//! Computes volga (∂²V/∂σ²) using finite differences.
//! Volga measures how vega changes with volatility.
//!
//! Volga ≈ (Vega(σ+h) - 2*Vega(σ) + Vega(σ-h)) / h²
//!
//! Or equivalently: (PV(σ+h) - 2*PV(σ) + PV(σ-h)) / h²

use crate::instruments::equity_option::EquityOption;
use crate::metrics::bump_sizes;
use crate::metrics::scale_surface;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Volga calculator for equity options.
pub struct VolgaCalculator;

impl MetricCalculator for VolgaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &EquityOption = context.instrument_as()?;
        let as_of = context.as_of;
        let base_pv = context.base_value.amount();

        // Check if expired
        let t = option.day_count.year_fraction(
            as_of,
            option.expiry,
            finstack_core::dates::DayCountCtx::default(),
        )?;
        if t <= 0.0 {
            return Ok(0.0);
        }

        let vol_bump_pct = bump_sizes::VOLATILITY;

        // Convert the relative surface bump into an absolute Δσ for ∂²V/∂σ².
        let sigma = context
            .curves
            .surface_ref(option.vol_surface_id.as_str())
            .map(|surf| surf.value_clamped(t, option.strike.amount()))
            .unwrap_or(0.0);
        let vol_bump_abs = (sigma * vol_bump_pct).abs();
        if vol_bump_abs < 1e-12 {
            return Ok(0.0);
        }

        // Bump vol up
        let curves_vol_up = scale_surface(
            &context.curves,
            option.vol_surface_id.as_str(),
            1.0 + vol_bump_pct,
        )?;
        let pv_vol_up = option.npv(&curves_vol_up, as_of)?.amount();

        // Bump vol down
        let curves_vol_down = scale_surface(
            &context.curves,
            option.vol_surface_id.as_str(),
            1.0 - vol_bump_pct,
        )?;
        let pv_vol_down = option.npv(&curves_vol_down, as_of)?.amount();

        // Volga (Vomma) = ∂²V/∂σ² ≈ (V(σ+Δσ) - 2V(σ) + V(σ-Δσ)) / (Δσ)²
        //
        // We bump the surface multiplicatively (relative) but divide by the corresponding
        // absolute volatility change Δσ = σ × bump_pct to keep the definition standard.
        let volga = (pv_vol_up - 2.0 * base_pv + pv_vol_down) / (vol_bump_abs * vol_bump_abs);

        Ok(volga)
    }
}
