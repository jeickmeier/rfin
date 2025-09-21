//! FX01 for FX Swaps.
//!
//! Computes sensitivity to a 1bp bump in the spot FX rate by revaluing with
//! a bumped spot while respecting instrument overrides for near/far rates.

use crate::instruments::fx_swap::FxSwap;
use crate::instruments::common::traits::Priceable;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::money::fx::{FxConversionPolicy, FxQuery};
use finstack_core::F;

/// FX01 (sensitivity to 1bp shift in spot rate).
pub struct FX01;

impl MetricCalculator for FX01 {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let fx_swap: &FxSwap = context.instrument_as()?;
        let curves = context.curves.clone();
        let as_of = context.as_of;

        let original_pv = fx_swap.value(&curves, as_of)?;

        let domestic_disc = curves
            .get::<finstack_core::market_data::term_structures::discount_curve::DiscountCurve>(
                fx_swap.domestic_disc_id,
            )?;
        let foreign_disc = curves
            .get::<finstack_core::market_data::term_structures::discount_curve::DiscountCurve>(
                fx_swap.foreign_disc_id,
            )?;

        let df_dom_near = domestic_disc.df_on_date_curve(fx_swap.near_date);
        let df_dom_far = domestic_disc.df_on_date_curve(fx_swap.far_date);
        let df_for_near = foreign_disc.df_on_date_curve(fx_swap.near_date);
        let df_for_far = foreign_disc.df_on_date_curve(fx_swap.far_date);

        let fx_matrix = curves.fx.as_ref().ok_or_else(|| {
            finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                id: "fx_matrix".to_string(),
            })
        })?;

        // Original spot
        let original_spot = (**fx_matrix)
            .rate(FxQuery {
                from: fx_swap.base_currency,
                to: fx_swap.quote_currency,
                on: as_of,
                policy: FxConversionPolicy::CashflowDate,
                closure_check: None,
                want_meta: false,
            })?
            .rate;

        // 1bp bump
        let bump = 0.0001;
        let bumped_spot = original_spot + bump;

        // Recompute near/far rates with bumped spot when not fixed
        let near_rate = fx_swap.near_rate.unwrap_or(bumped_spot);
        let far_rate = fx_swap
            .far_rate
            .unwrap_or(bumped_spot * df_for_far / df_dom_far);

        let base_amt = fx_swap.base_notional.amount();
        let pv_for_leg = base_amt * df_for_near - base_amt * df_for_far;
        let pv_dom_leg = -base_amt * near_rate * df_dom_near + base_amt * far_rate * df_dom_far;

        let bumped_pv = pv_for_leg * bumped_spot + pv_dom_leg;
        Ok(bumped_pv - original_pv.amount())
    }
}


