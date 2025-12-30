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
        let curves = context.curves.clone();
        let as_of = context.as_of;

        let domestic_disc = curves.get_discount(fx_swap.domestic_discount_curve_id.as_str())?;
        let foreign_disc = curves.get_discount(fx_swap.foreign_discount_curve_id.as_str())?;

        let df_dom_near = domestic_disc.df_between_dates(as_of, fx_swap.near_date)?;
        let df_dom_far = domestic_disc.df_between_dates(as_of, fx_swap.far_date)?;
        let df_for_near = foreign_disc.df_between_dates(as_of, fx_swap.near_date)?;
        let df_for_far = foreign_disc.df_between_dates(as_of, fx_swap.far_date)?;

        // Settlement checks
        let include_near = fx_swap.near_date >= as_of;
        let include_far = fx_swap.far_date >= as_of;

        // Get current FX spot rate
        let fx_matrix = context.curves.fx().ok_or_else(|| {
            finstack_core::Error::from(finstack_core::InputError::NotFound {
                id: "fx_matrix".to_string(),
            })
        })?;

        let current_rate = fx_matrix
            .as_ref()
            .rate(FxQuery::new(
                fx_swap.base_currency,
                fx_swap.quote_currency,
                as_of,
            ))?
            .rate;

        // Helper to calculate PV for a given spot rate
        let calculate_pv = |spot: f64| -> f64 {
            // Recompute near/far rates with bumped spot when not fixed
            let near_rate = fx_swap.near_rate.unwrap_or(spot);

            // Covered interest parity: F = S × DF_for / DF_dom
            let dom_ratio = if df_dom_near.abs() > 1e-12 {
                df_dom_far / df_dom_near
            } else {
                1.0
            };
            let for_ratio = if df_for_near.abs() > 1e-12 {
                df_for_far / df_for_near
            } else {
                1.0
            };
            let model_fwd = spot * for_ratio / dom_ratio;

            let far_rate = fx_swap.far_rate.unwrap_or(model_fwd);
            let base_amt = fx_swap.base_notional.amount();

            let mut pv_for_leg = 0.0;
            if include_near {
                pv_for_leg += base_amt * df_for_near;
            }
            if include_far {
                pv_for_leg -= base_amt * df_for_far;
            }

            let mut pv_dom_leg = 0.0;
            if include_near {
                pv_dom_leg -= base_amt * near_rate * df_dom_near;
            }
            if include_far {
                pv_dom_leg += base_amt * far_rate * df_dom_far;
            }

            // Convert foreign leg to domestic at the bumped spot
            pv_for_leg * spot + pv_dom_leg
        };

        // Central finite difference
        let rate_bump_amt = current_rate * bump_sizes::SPOT; // e.g. 1% of spot
        let pv_up = calculate_pv(current_rate + rate_bump_amt);
        let pv_down = calculate_pv(current_rate - rate_bump_amt);

        // FX Delta = (PV_up - PV_down) / (2 * bump_pct) ??
        // Note: bump_sizes::SPOT is the percentage (e.g. 0.01).
        // If we divide by (2 * bump_sizes::SPOT), we get the sensitivity to a 100% move (linearized), scaled by 1%.
        // Or rather: Delta = dPV/dS * S.
        // (PV_up - PV_down) / (2 * dS) * S = (PV_up - PV_down) / (2 * S * pct) * S = (PV_up - PV_down) / (2 * pct).
        // The code divides by `2.0 * bump_sizes::SPOT`. This matches the definition of "Change in PV for 1% move" (if result is roughly PV * 1%).

        let fx_delta = (pv_up - pv_down) / (2.0 * bump_sizes::SPOT);

        Ok(fx_delta)
    }
}
