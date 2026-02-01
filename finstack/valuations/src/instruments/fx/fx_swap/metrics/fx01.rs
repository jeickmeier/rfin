//! FX01 for FX Swaps.
//!
//! Computes sensitivity to a 1bp absolute bump in the spot FX rate using
//! central finite difference for O(h²) accuracy.
//!
//! # FX01 vs FX Delta
//!
//! - **FX01**: Sensitivity to a **1bp absolute** move in spot rate (0.0001).
//!   Uses central difference: (PV(S+0.0001) - PV(S-0.0001)) / 2
//!   Useful for small perturbation analysis and hedge ratio calculation.
//!
//! - **FX Delta**: Sensitivity to a **1% relative** move in spot rate.
//!   See [`fx_delta::FxDeltaCalculator`] for details.
//!   Useful for normalized risk comparison across different spot levels.

use crate::instruments::fx::fx_swap::pricing_helper::FxSwapPricingContext;
use crate::instruments::fx::fx_swap::FxSwap;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// FX01 (sensitivity to 1bp absolute shift in spot rate).
///
/// Uses central finite difference for O(h²) accuracy:
/// FX01 = (PV(S + 0.0001) - PV(S - 0.0001)) / 2
pub struct FX01;

/// Standard 1bp absolute bump for FX01 calculation.
const FX01_BUMP: f64 = 0.0001;

impl MetricCalculator for FX01 {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let fx_swap: &FxSwap = context.instrument_as()?;
        let curves = context.curves.clone();
        let as_of = context.as_of;

        // Use shared pricing context for consistent calculations
        let ctx = FxSwapPricingContext::build(fx_swap, &curves, as_of)?;

        // Helper to calculate PV for a given spot rate
        let calculate_pv = |spot: f64| -> Result<f64> {
            // Recompute near/far rates with bumped spot when not fixed
            let near_rate = fx_swap.near_rate.unwrap_or(spot);

            // Recompute forward with bumped spot
            let model_fwd = FxSwapPricingContext::calculate_cip_forward(
                spot,
                ctx.df_dom_near,
                ctx.df_dom_far,
                ctx.df_for_near,
                ctx.df_for_far,
            )?;

            let far_rate = fx_swap.far_rate.unwrap_or(model_fwd);

            Ok(ctx.total_pv_with_spot(spot, near_rate, far_rate))
        };

        // Central finite difference with 1bp absolute bump
        let pv_up = calculate_pv(ctx.model_spot + FX01_BUMP)?;
        let pv_down = calculate_pv(ctx.model_spot - FX01_BUMP)?;

        // FX01 = (PV_up - PV_down) / 2 (per 1bp move)
        let fx01 = (pv_up - pv_down) / 2.0;

        Ok(fx01)
    }
}
