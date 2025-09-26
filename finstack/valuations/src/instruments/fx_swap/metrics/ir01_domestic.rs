//! Domestic IR01 for FX Swaps.
//!
//! Computes sensitivity to a 1bp parallel bump in the domestic (quote) discount curve
//! by revaluing with bumped domestic discount factors.

use crate::instruments::common::traits::Instrument;
use crate::instruments::fx_swap::FxSwap;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::money::fx::FxQuery;
use finstack_core::F;

/// Domestic IR01 (sensitivity to 1bp parallel shift in domestic curve).
pub struct DomesticIR01;

impl MetricCalculator for DomesticIR01 {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let fx_swap: &FxSwap = context.instrument_as()?;
        let curves = context.curves.clone();
        let as_of = context.as_of;
        let original_pv = fx_swap.value(&curves, as_of)?;

        let domestic_disc = curves.get_discount(fx_swap.domestic_disc_id)?;
        let foreign_disc = curves.get_discount(fx_swap.foreign_disc_id)?;

        // Bump domestic curve by 1bp: df_bumped(t) = df(t) * exp(-bp * t)
        let bump = 0.0001;
        let df_dom_near_b = {
            let base = domestic_disc.base_date();
            let t = domestic_disc
                .day_count()
                .year_fraction(
                    base,
                    fx_swap.near_date,
                    finstack_core::dates::DayCountCtx::default(),
                )
                .unwrap_or(0.0);
            domestic_disc.df_on_date_curve(fx_swap.near_date) * (-bump * t).exp()
        };
        let df_dom_far_b = {
            let base = domestic_disc.base_date();
            let t = domestic_disc
                .day_count()
                .year_fraction(
                    base,
                    fx_swap.far_date,
                    finstack_core::dates::DayCountCtx::default(),
                )
                .unwrap_or(0.0);
            domestic_disc.df_on_date_curve(fx_swap.far_date) * (-bump * t).exp()
        };

        let df_for_near = foreign_disc.df_on_date_curve(fx_swap.near_date);
        let df_for_far = foreign_disc.df_on_date_curve(fx_swap.far_date);

        // Resolve near rate at as_of
        let fx_matrix = curves.fx.as_ref().ok_or_else(|| {
            finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                id: "fx_matrix".to_string(),
            })
        })?;
        let near_rate = match fx_swap.near_rate {
            Some(rate) => rate,
            None => {
                (**fx_matrix)
                    .rate(FxQuery::new(
                        fx_swap.base_currency,
                        fx_swap.quote_currency,
                        as_of,
                    ))?
                    .rate
            }
        };

        // Far rate uses bumped domestic df in parity
        let far_rate = match fx_swap.far_rate {
            Some(rate) => rate,
            None => near_rate * df_for_far / df_dom_far_b,
        };

        let base_amt = fx_swap.base_notional.amount();
        let pv_for_leg = base_amt * df_for_near - base_amt * df_for_far; // unchanged in base
        let pv_dom_leg = -base_amt * near_rate * df_dom_near_b + base_amt * far_rate * df_dom_far_b;

        // Convert base leg to quote at spot
        let spot = (**fx_matrix)
            .rate(FxQuery::new(
                fx_swap.base_currency,
                fx_swap.quote_currency,
                as_of,
            ))?
            .rate;

        let bumped_pv = pv_for_leg * spot + pv_dom_leg;
        Ok(bumped_pv - original_pv.amount())
    }
}
