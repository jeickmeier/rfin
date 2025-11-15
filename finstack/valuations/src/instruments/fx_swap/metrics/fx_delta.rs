//! FX Delta calculator for FX Swaps.
//!
//! Computes FX delta (FX spot sensitivity) using finite differences.
//! FX delta measures the change in PV for a 1% move in the FX spot rate.
//!
//! Note: This complements fx01 (which is per 1bp). FX delta is per 1% move.

use crate::instruments::fx_swap::FxSwap;
use crate::metrics::bump_sizes;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::money::fx::FxQuery;
use finstack_core::Result;

/// FX Delta calculator for FX Swaps.
pub struct FxDeltaCalculator;

impl MetricCalculator for FxDeltaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let fx_swap: &FxSwap = context.instrument_as()?;
        let as_of = context.as_of;

        // Get current FX spot rate
        let fx_matrix = context.curves.fx.as_ref().ok_or_else(|| {
            finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                id: "fx_matrix".to_string(),
            })
        })?;

        let current_rate = (**fx_matrix)
            .rate(FxQuery::new(
                fx_swap.base_currency,
                fx_swap.quote_currency,
                as_of,
            ))?
            .rate;

        // Bump spot rate by 1%
        let rate_bump = current_rate * bump_sizes::SPOT;
        let bumped_rate = current_rate + rate_bump;

        // Create bumped market context
        // For FX swaps, we need to bump the FX matrix
        // Since FxMatrix is complex, we'll create a new swap with bumped rates
        // Note: FxSwap uses near_rate and far_rate if provided, otherwise computes from spot
        let mut fx_swap_up = fx_swap.clone();
        // Bump near rate if not fixed
        if fx_swap.near_rate.is_none() {
            fx_swap_up.near_rate = Some(bumped_rate);
        }
        // Far rate will be recomputed from bumped spot in npv()

        let pv_up = fx_swap_up.npv(context.curves.as_ref(), as_of)?.amount();

        // Also compute down scenario for symmetric finite difference
        let bumped_rate_down = current_rate - rate_bump;
        let mut fx_swap_down = fx_swap.clone();
        if fx_swap.near_rate.is_none() {
            fx_swap_down.near_rate = Some(bumped_rate_down);
        }
        let pv_down = fx_swap_down.npv(context.curves.as_ref(), as_of)?.amount();

        // FX Delta = (PV_up - PV_down) / (2 * bump_size)
        let fx_delta = (pv_up - pv_down) / (2.0 * bump_sizes::SPOT);

        Ok(fx_delta)
    }
}
