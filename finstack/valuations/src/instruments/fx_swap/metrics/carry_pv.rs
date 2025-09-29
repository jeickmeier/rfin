//! Carry PV metric for FX swaps.
//!
//! Computes the present value contribution from holding the foreign leg
//! (converted to the domestic currency) using model-implied spot/forward
//! rates. This approximates the "carry" earned from the interest differential
//! between currencies when maintaining the swap position.

use crate::instruments::fx_swap::FxSwap;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::money::fx::FxQuery;
use finstack_core::F;

/// Carry PV calculator for FX swaps.
pub struct CarryPv;

impl MetricCalculator for CarryPv {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let fx_swap: &FxSwap = context.instrument_as()?;
        let curves = context.curves.clone();
        let as_of = context.as_of;

        let domestic_disc = curves.get_discount_ref(fx_swap.domestic_disc_id.as_str())?;
        let foreign_disc = curves.get_discount_ref(fx_swap.foreign_disc_id.as_str())?;

        let df_dom_near = domestic_disc.df_on_date_curve(fx_swap.near_date);
        let df_dom_far = domestic_disc.df_on_date_curve(fx_swap.far_date);
        let df_for_far = foreign_disc.df_on_date_curve(fx_swap.far_date);

        let model_spot = if let Some(fx_matrix) = curves.fx.as_ref() {
            (**fx_matrix)
                .rate(FxQuery::new(
                    fx_swap.base_currency,
                    fx_swap.quote_currency,
                    as_of,
                ))?
                .rate
        } else if let Some(rate) = fx_swap.near_rate {
            rate
        } else {
            return Err(finstack_core::error::InputError::NotFound {
                id: "fx_matrix".to_string(),
            }
            .into());
        };

        let model_fwd = model_spot * df_for_far / df_dom_far;
        let base_amount = fx_swap.base_notional.amount();

        // Carry PV corresponds to the converted foreign leg PV using model-implied rates.
        let carry_pv =
            base_amount * model_spot * df_dom_near - base_amount * model_fwd * df_dom_far;
        Ok(carry_pv)
    }
}
