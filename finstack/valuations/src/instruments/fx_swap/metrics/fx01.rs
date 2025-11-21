//! FX01 for FX Swaps.
//!
//! Computes sensitivity to a 1bp bump in the spot FX rate by revaluing with
//! a bumped spot while respecting instrument overrides for near/far rates.

use crate::instruments::common::traits::Instrument;
use crate::instruments::fx_swap::FxSwap;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::money::fx::FxQuery;

/// FX01 (sensitivity to 1bp shift in spot rate).
pub struct FX01;

impl MetricCalculator for FX01 {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let fx_swap: &FxSwap = context.instrument_as()?;
        let curves = context.curves.clone();
        let as_of = context.as_of;

        let original_pv = fx_swap.value(&curves, as_of)?;

        let domestic_disc = curves.get_discount_ref(fx_swap.domestic_discount_curve_id.as_str())?;
        let foreign_disc = curves.get_discount_ref(fx_swap.foreign_discount_curve_id.as_str())?;

        let df_as_of_dom = domestic_disc.df_on_date_curve(as_of);
        let df_as_of_for = foreign_disc.df_on_date_curve(as_of);

        let normalize = |df: f64, df_base: f64| -> f64 {
            if df_base != 0.0 {
                df / df_base
            } else {
                1.0
            }
        };

        let df_dom_near = normalize(
            domestic_disc.df_on_date_curve(fx_swap.near_date),
            df_as_of_dom,
        );
        let df_dom_far = normalize(
            domestic_disc.df_on_date_curve(fx_swap.far_date),
            df_as_of_dom,
        );
        let df_for_near = normalize(
            foreign_disc.df_on_date_curve(fx_swap.near_date),
            df_as_of_for,
        );
        let df_for_far = normalize(
            foreign_disc.df_on_date_curve(fx_swap.far_date),
            df_as_of_for,
        );

        // Settlement checks
        let include_near = fx_swap.near_date >= as_of;
        let include_far = fx_swap.far_date >= as_of;

        let fx_matrix = curves.fx.as_ref().ok_or_else(|| {
            finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                id: "fx_matrix".to_string(),
            })
        })?;

        // Original spot
        let original_spot = (**fx_matrix)
            .rate(FxQuery::new(
                fx_swap.base_currency,
                fx_swap.quote_currency,
                as_of,
            ))?
            .rate;

        // 1bp bump
        let bump = 0.0001;
        let bumped_spot = original_spot + bump;

        // Recompute near/far rates with bumped spot when not fixed
        let near_rate = fx_swap.near_rate.unwrap_or(bumped_spot);
        let far_rate = fx_swap.far_rate.unwrap_or(if df_dom_far.abs() > 1e-12 {
            bumped_spot * df_for_far / df_dom_far
        } else {
            bumped_spot
        });

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

        let bumped_pv = pv_for_leg * bumped_spot + pv_dom_leg;
        Ok(bumped_pv - original_pv.amount())
    }
}
