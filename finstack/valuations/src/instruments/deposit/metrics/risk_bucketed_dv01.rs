//! Bucketed DV01 calculator for deposits using structured metric storage.

use crate::cashflow::traits::CashflowProvider;
use crate::instruments::deposit::types::Deposit;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::F;

pub struct BucketedDv01Calculator;

impl MetricCalculator for BucketedDv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let dep_ref: &Deposit = context.instrument_as()?;
        let dep = dep_ref.clone();
        let disc_id = dep.disc_id.clone();

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

        // Revaluation: rebuild flows from instrument, discount with bumped curve
        let curves = context.curves.clone();
        let as_of = context.as_of;
        let reval = move |
            bumped_disc: &finstack_core::market_data::term_structures::discount_curve::DiscountCurve|
         {
            let flows = dep.build_schedule(&curves, as_of)?;
            let base = bumped_disc.base_date();
            let dc = bumped_disc.day_count();
            crate::instruments::common::discountable::npv_static(
                bumped_disc,
                base,
                dc,
                &flows,
            )
        };

        let total =
            crate::metrics::compute_bucketed_dv01_series(context, &disc_id, labels, 1.0, reval)?;

        Ok(total)
    }
}
