//! Foreign IR01 for FX Swaps.
//!
//! Computes sensitivity to a 1bp parallel bump in the foreign (base) discount curve
//! by revaluing with bumped foreign discount factors.

use crate::instruments::common::traits::Priceable;
use crate::instruments::fx_swap::FxSwap;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::money::fx::{FxConversionPolicy, FxQuery};
use finstack_core::F;

/// Foreign IR01 (sensitivity to 1bp parallel shift in foreign curve).
pub struct ForeignIR01;

impl MetricCalculator for ForeignIR01 {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let fx_swap: &FxSwap = context.instrument_as()?;
        let curves = context.curves.clone();
        let as_of = context.as_of;
        let original_pv = fx_swap.value(&curves, as_of)?;

        let domestic_disc =
            curves
                .get::<finstack_core::market_data::term_structures::discount_curve::DiscountCurve>(
                    fx_swap.domestic_disc_id,
                )?;
        let foreign_disc =
            curves
                .get::<finstack_core::market_data::term_structures::discount_curve::DiscountCurve>(
                    fx_swap.foreign_disc_id,
                )?;

        // Bump foreign curve by 1bp
        let bump = 0.0001;
        let df_dom_near = domestic_disc.df_on_date_curve(fx_swap.near_date);
        let df_dom_far = domestic_disc.df_on_date_curve(fx_swap.far_date);

        let df_for_near_b = {
            let base = foreign_disc.base_date();
            let t = foreign_disc
                .day_count()
                .year_fraction(
                    base,
                    fx_swap.near_date,
                    finstack_core::dates::DayCountCtx::default(),
                )
                .unwrap_or(0.0);
            foreign_disc.df_on_date_curve(fx_swap.near_date) * (-bump * t).exp()
        };
        let df_for_far_b = {
            let base = foreign_disc.base_date();
            let t = foreign_disc
                .day_count()
                .year_fraction(
                    base,
                    fx_swap.far_date,
                    finstack_core::dates::DayCountCtx::default(),
                )
                .unwrap_or(0.0);
            foreign_disc.df_on_date_curve(fx_swap.far_date) * (-bump * t).exp()
        };

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
                    .rate(FxQuery {
                        from: fx_swap.base_currency,
                        to: fx_swap.quote_currency,
                        on: as_of,
                        policy: FxConversionPolicy::CashflowDate,
                        closure_check: None,
                        want_meta: false,
                    })?
                    .rate
            }
        };

        // Far rate uses bumped foreign df in parity
        let far_rate = match fx_swap.far_rate {
            Some(rate) => rate,
            None => near_rate * df_for_far_b / df_dom_far,
        };

        let base_amt = fx_swap.base_notional.amount();
        let pv_for_leg = base_amt * df_for_near_b - base_amt * df_for_far_b; // bumped in base leg
        let pv_dom_leg = -base_amt * near_rate * df_dom_near + base_amt * far_rate * df_dom_far;

        let spot = (**fx_matrix)
            .rate(FxQuery {
                from: fx_swap.base_currency,
                to: fx_swap.quote_currency,
                on: as_of,
                policy: FxConversionPolicy::CashflowDate,
                closure_check: None,
                want_meta: false,
            })?
            .rate;

        let bumped_pv = pv_for_leg * spot + pv_dom_leg;
        Ok(bumped_pv - original_pv.amount())
    }
}
