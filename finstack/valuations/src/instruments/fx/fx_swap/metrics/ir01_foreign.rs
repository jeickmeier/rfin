//! Foreign IR01 for FX Swaps.
//!
//! Computes sensitivity to a 1bp parallel bump in the foreign (base) discount curve
//! by revaluing with bumped foreign discount factors.

use crate::instruments::common::traits::Instrument;
use crate::instruments::fx_swap::FxSwap;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::money::fx::FxQuery;

/// Foreign IR01 (sensitivity to 1bp parallel shift in foreign curve).
pub struct ForeignIR01;

impl MetricCalculator for ForeignIR01 {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let fx_swap: &FxSwap = context.instrument_as()?;
        let curves = context.curves.clone();
        let as_of = context.as_of;
        let original_pv = fx_swap.value(&curves, as_of)?;

        let domestic_disc = curves.get_discount_ref(fx_swap.domestic_discount_curve_id.as_str())?;
        let foreign_disc = curves.get_discount_ref(fx_swap.foreign_discount_curve_id.as_str())?;

        // Settlement checks
        let include_near = fx_swap.near_date >= as_of;
        let include_far = fx_swap.far_date >= as_of;

        let df_dom_near = domestic_disc.df_between_dates(as_of, fx_swap.near_date)?;
        let df_dom_far = domestic_disc.df_between_dates(as_of, fx_swap.far_date)?;
        let df_for_near = foreign_disc.df_between_dates(as_of, fx_swap.near_date)?;
        let df_for_far = foreign_disc.df_between_dates(as_of, fx_swap.far_date)?;

        // Bump foreign curve by 1bp relative to as_of
        let bump = 0.0001;

        let t_near = foreign_disc
            .day_count()
            .year_fraction(
                as_of,
                fx_swap.near_date,
                finstack_core::dates::DayCountCtx::default(),
            )
            .unwrap_or(0.0);
        let df_for_near_b = df_for_near * (-bump * t_near).exp();

        let t_far = foreign_disc
            .day_count()
            .year_fraction(
                as_of,
                fx_swap.far_date,
                finstack_core::dates::DayCountCtx::default(),
            )
            .unwrap_or(0.0);
        let df_for_far_b = df_for_far * (-bump * t_far).exp();

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

        // Far rate uses bumped foreign df in parity: F = S × DF_for / DF_dom
        // Only needed if not fixed
        let far_rate = match fx_swap.far_rate {
            Some(rate) => rate,
            None => {
                let dom_ratio = if df_dom_near.abs() > 1e-12 {
                    df_dom_far / df_dom_near
                } else {
                    1.0
                };
                let for_ratio = if df_for_near_b.abs() > 1e-12 {
                    df_for_far_b / df_for_near_b
                } else {
                    1.0
                };
                near_rate * for_ratio / dom_ratio
            }
        };

        let base_amt = fx_swap.base_notional.amount();

        let mut pv_for_leg = 0.0;
        if include_near {
            pv_for_leg += base_amt * df_for_near_b;
        }
        if include_far {
            pv_for_leg -= base_amt * df_for_far_b;
        }

        let mut pv_dom_leg = 0.0;
        if include_near {
            pv_dom_leg -= base_amt * near_rate * df_dom_near;
        }
        if include_far {
            pv_dom_leg += base_amt * far_rate * df_dom_far;
        }

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
