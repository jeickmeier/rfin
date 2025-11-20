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

        let df_as_of_dom = domestic_disc.df_on_date_curve(as_of);
        let df_as_of_for = foreign_disc.df_on_date_curve(as_of);

        let calc_df = |curve: &finstack_core::market_data::term_structures::DiscountCurve, date: finstack_core::dates::Date, df_as_of: f64| -> f64 {
            if df_as_of != 0.0 {
                curve.df_on_date_curve(date) / df_as_of
            } else {
                1.0
            }
        };

        let df_dom_near = calc_df(domestic_disc, fx_swap.near_date, df_as_of_dom);
        let df_dom_far = calc_df(domestic_disc, fx_swap.far_date, df_as_of_dom);
        let df_for_far = calc_df(foreign_disc, fx_swap.far_date, df_as_of_for);

        let include_near = fx_swap.near_date >= as_of;
        let include_far = fx_swap.far_date >= as_of;

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

        let model_fwd = if df_dom_far.abs() > 1e-12 {
            model_spot * df_for_far / df_dom_far
        } else {
            model_spot
        };
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
