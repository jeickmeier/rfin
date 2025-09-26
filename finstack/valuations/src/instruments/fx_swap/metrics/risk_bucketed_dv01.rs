//! Bucketed DV01 for FX Swap domestic discount curve.

use crate::instruments::fx_swap::types::FxSwap;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::F;

pub struct BucketedDv01Calculator;

impl MetricCalculator for BucketedDv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let inst_ref: &FxSwap = context.instrument_as()?;
        let inst = inst_ref.clone();
        let disc_id = finstack_core::types::CurveId::from(inst.domestic_disc_id);

        let labels: Vec<String> = crate::metrics::standard_ir_dv01_buckets()
            .iter()
            .map(|y| {
                if *y < 1.0 {
                    format!("{:.0}m", (y * 12.0).round())
                } else {
                    format!("{:.0}y", y)
                }
            })
            .collect();

        let curves = context.curves.clone();
        let as_of = context.as_of;
        let reval = move |
            bumped_disc: &finstack_core::market_data::term_structures::discount_curve::DiscountCurve|
         {
            // Pull foreign curve and spot/forward from context; use bumped domestic DF
            let foreign_disc = curves.get_discount_ref(inst.foreign_disc_id)?;
            let df_dom_near = bumped_disc.df_on_date_curve(inst.near_date);
            let df_dom_far = bumped_disc.df_on_date_curve(inst.far_date);
            let df_for_far = foreign_disc.df_on_date_curve(inst.far_date);
            let spot = if let Some(rate) = inst.near_rate {
                rate
            } else {
                let fx_matrix = curves.fx.as_ref().ok_or_else(|| finstack_core::Error::from(
                    finstack_core::error::InputError::NotFound { id: "fx_matrix".to_string() },
                ))?;
                (**fx_matrix)
                    .rate(finstack_core::money::fx::FxQuery::new(inst.base_currency, inst.quote_currency, as_of))?
                    .rate
            };
            let fwd = if let Some(rate) = inst.far_rate { rate } else { spot * df_for_far / df_dom_far };
            if inst.base_notional.currency() != inst.base_currency {
                return Err(finstack_core::Error::from(finstack_core::error::InputError::Invalid));
            }
            let n_base = inst.base_notional.amount();
            let pv_foreign_dom = n_base * spot * df_dom_near - n_base * fwd * df_dom_far;
            let pv_dom_leg = -n_base * spot * df_dom_near + n_base * fwd * df_dom_far;
            let total_pv = pv_foreign_dom + pv_dom_leg;
            Ok(finstack_core::money::Money::new(total_pv, inst.quote_currency))
        };

        crate::metrics::compute_bucketed_dv01_series(context, &disc_id, labels, 1.0, reval)
    }
}
