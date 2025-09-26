//! Bucketed DV01 for CDS Tranche discount curve sensitivity by tenor.

use crate::instruments::cds_tranche::CdsTranche;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::F;

pub struct BucketedDv01Calculator;

impl MetricCalculator for BucketedDv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let inst_ref: &CdsTranche = context.instrument_as()?;
        let inst = inst_ref.clone();
        let disc_id = inst.disc_id.clone();

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
                crate::instruments::cds_tranche::pricing::engine::CDSTranchePricer::new()
                    .price_tranche(&inst, temp_ctx, as_of)
            },
        )
    }
}
