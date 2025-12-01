//! Forward points metric for FX Swaps.
//!
//! Computes forward points as `far_rate - near_rate`, where the near rate is
//! either provided on the instrument or sourced from the FX matrix, and the far
//! rate is either provided or derived from covered interest parity using the
//! discount curves.

use crate::instruments::fx_swap::FxSwap;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::money::fx::FxQuery;

/// Forward points (far rate - near rate).
pub struct ForwardPoints;

impl MetricCalculator for ForwardPoints {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let fx_swap: &FxSwap = context.instrument_as()?;
        let curves = context.curves.clone();
        let as_of = context.as_of;

        let domestic_disc = curves.get_discount_ref(fx_swap.domestic_discount_curve_id.as_str())?;
        let foreign_disc = curves.get_discount_ref(fx_swap.foreign_discount_curve_id.as_str())?;

        // Use curve-consistent discount factors on dates (relative to as_of)
        let df_as_of_dom = domestic_disc.df_on_date_curve(as_of);
        let df_as_of_for = foreign_disc.df_on_date_curve(as_of);
        let df_dom_near = domestic_disc.df_on_date_curve(fx_swap.near_date) / df_as_of_dom;
        let df_dom_far = domestic_disc.df_on_date_curve(fx_swap.far_date) / df_as_of_dom;
        let df_for_near = foreign_disc.df_on_date_curve(fx_swap.near_date) / df_as_of_for;
        let df_for_far = foreign_disc.df_on_date_curve(fx_swap.far_date) / df_as_of_for;

        // Resolve near spot rate
        let near_rate = match fx_swap.near_rate {
            Some(rate) => rate,
            None => {
                let fx_matrix = curves.fx.as_ref().ok_or_else(|| {
                    finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                        id: "fx_matrix".to_string(),
                    })
                })?;
                (**fx_matrix)
                    .rate(FxQuery::new(
                        fx_swap.base_currency,
                        fx_swap.quote_currency,
                        as_of,
                    ))?
                    .rate
            }
        };

        // Resolve far forward rate from curves when not provided
        // Covered interest parity: F = S × (DF_for_far/DF_for_near) / (DF_dom_far/DF_dom_near)
        // When r_dom > r_for, forward is at premium (F > S) as required by no-arbitrage.
        // Derivation: F = S × (1 + r_dom × T) / (1 + r_for × T) = S × DF_for / DF_dom
        let far_rate = match fx_swap.far_rate {
            Some(rate) => rate,
            None => {
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
                near_rate * for_ratio / dom_ratio
            }
        };

        Ok(far_rate - near_rate)
    }
}
