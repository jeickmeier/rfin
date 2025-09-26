//! Bucketed DV01 for CDS premium discounting.

use crate::instruments::cds::types::CreditDefaultSwap;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::F;

pub struct BucketedDv01Calculator;

impl MetricCalculator for BucketedDv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let cds_ref: &CreditDefaultSwap = context.instrument_as()?;
        let cds = cds_ref.clone();
        let disc_id = cds.premium.disc_id.clone();

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

        let as_of = context.as_of;
        crate::metrics::compute_bucketed_dv01_series_with_context(
            context,
            &disc_id,
            labels,
            1.0,
            move |temp_ctx| {
                let disc = temp_ctx.get_discount_ref(cds.premium.disc_id.clone())?;
                let surv = temp_ctx.get_hazard_ref(cds.protection.credit_id.clone())?;
                crate::instruments::cds::pricing::engine::CDSPricer::new()
                    .npv(&cds, disc, surv, as_of)
            },
        )
    }
}
