//! Carry PV metric for FX swaps.
//!
//! Computes the present value contribution from holding the foreign leg
//! (converted to the domestic currency) using model-implied spot/forward
//! rates. This approximates the "carry" earned from the interest differential
//! between currencies when maintaining the swap position.

use crate::instruments::fx_swap::FxSwap;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::money::fx::FxQuery;

/// Carry PV calculator for FX swaps.
pub struct CarryPv;

impl MetricCalculator for CarryPv {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let fx_swap: &FxSwap = context.instrument_as()?;
        let curves = context.curves.clone();
        let as_of = context.as_of;

        let domestic_disc = curves.get_discount_ref(fx_swap.domestic_discount_curve_id.as_str())?;
        let foreign_disc = curves.get_discount_ref(fx_swap.foreign_discount_curve_id.as_str())?;

        let df_dom_near = domestic_disc.df_between_dates(as_of, fx_swap.near_date)?;
        let df_dom_far = domestic_disc.df_between_dates(as_of, fx_swap.far_date)?;
        let df_for_near = foreign_disc.df_between_dates(as_of, fx_swap.near_date)?;
        let df_for_far = foreign_disc.df_between_dates(as_of, fx_swap.far_date)?;

        let include_near = fx_swap.near_date >= as_of;
        let include_far = fx_swap.far_date >= as_of;

        let model_spot = if let Some(fx_matrix) = curves.fx() {
            fx_matrix
                .as_ref()
                .rate(FxQuery::new(
                    fx_swap.base_currency,
                    fx_swap.quote_currency,
                    as_of,
                ))?
                .rate
        } else if let Some(rate) = fx_swap.near_rate {
            rate
        } else {
            return Err(finstack_core::InputError::NotFound {
                id: "fx_matrix".to_string(),
            }
            .into());
        };

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
        let model_fwd = model_spot * for_ratio / dom_ratio;
        let base_amount = fx_swap.base_notional.amount();

        // Carry PV corresponds to the converted foreign leg PV using model-implied rates.
        let term1 = if include_near {
            base_amount * model_spot * df_dom_near
        } else {
            0.0
        };
        let term2 = if include_far {
            base_amount * model_fwd * df_dom_far
        } else {
            0.0
        };

        let carry_pv = term1 - term2;
        Ok(carry_pv)
    }
}
