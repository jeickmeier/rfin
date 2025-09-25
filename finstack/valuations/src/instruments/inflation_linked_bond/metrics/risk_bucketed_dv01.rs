//! Bucketed DV01 for ILBs using discount curve bumps.

use crate::instruments::inflation_linked_bond::types::InflationLinkedBond;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::F;

pub struct BucketedDv01Calculator;

impl MetricCalculator for BucketedDv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let ilb_ref: &InflationLinkedBond = context.instrument_as()?;
        let ilb = ilb_ref.clone();
        let disc_id = ilb.disc_id.clone();

        let labels: Vec<String> = crate::metrics::standard_ir_dv01_buckets()
            .iter()
            .map(|y| if *y < 1.0 { format!("{:.0}m", (y * 12.0).round()) } else { format!("{:.0}y", y) })
            .collect();
        let map_label = |label: &str| -> (F, F) {
            if let Some(m) = label.strip_suffix('m') {
                let months: F = m.parse::<F>().unwrap_or(0.0);
                let y = (months / 12.0).max(0.0);
                (y, y)
            } else if let Some(y) = label.strip_suffix('y') {
                let yv: F = y.parse::<F>().unwrap_or(0.0);
                (yv, yv)
            } else {
                (0.0, 0.0)
            }
        };

        let curves = context.curves.clone();
        let as_of = context.as_of;
        let reval = move |
            bumped_disc: &finstack_core::market_data::term_structures::discount_curve::DiscountCurve|
         {
            use crate::instruments::inflation_linked_bond::pricing::InflationLinkedBondEngine;
            use crate::instruments::common::discountable::npv_static;
            let flows = InflationLinkedBondEngine::build_schedule(&ilb, &curves, as_of)?;
            let base = bumped_disc.base_date();
            let dc = bumped_disc.day_count();
            npv_static(bumped_disc, base, dc, &flows)
        };

        crate::metrics::compute_bucketed_dv01_series(
            context,
            &disc_id,
            labels,
            map_label,
            1.0,
            reval,
        )
    }
}


