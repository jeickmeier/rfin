//! Volga calculator for equity options.
//!
//! Computes volga (∂²V/∂σ²) using finite differences.
//! Volga measures how vega changes with volatility.
//!
//! Volga ≈ (Vega(σ+h) - 2*Vega(σ) + Vega(σ-h)) / h²
//!
//! Or equivalently: (PV(σ+h) - 2*PV(σ) + PV(σ-h)) / h²

use crate::instruments::common::traits::Instrument;
use crate::instruments::equity_option::EquityOption;
use crate::metrics::bump_sizes;
use crate::metrics::bump_surface_vol_absolute;
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

        let vol_bump_abs = bump_sizes::VOLATILITY;

        // Bump vol up
        let curves_vol_up = bump_surface_vol_absolute(
            &context.curves,
            option.vol_surface_id.as_str(),
            vol_bump_abs,
        )?;
        let pv_vol_up = option.value(&curves_vol_up, as_of)?.amount();

        // Bump vol down
        let curves_vol_down = bump_surface_vol_absolute(
            &context.curves,
            option.vol_surface_id.as_str(),
            -vol_bump_abs,
        )?;
        let pv_vol_down = option.value(&curves_vol_down, as_of)?.amount();

        // Volga (Vomma) = ∂²V/∂σ² ≈ (V(σ+Δσ) - 2V(σ) + V(σ-Δσ)) / (Δσ)²
        //
        let volga = (pv_vol_up - 2.0 * base_pv + pv_vol_down) / (vol_bump_abs * vol_bump_abs);

        Ok(volga)
    }
}
