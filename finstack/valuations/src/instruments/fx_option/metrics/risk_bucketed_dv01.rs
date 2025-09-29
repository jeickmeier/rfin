//! Bucketed DV01 for FX Options with separate domestic and foreign buckets.

use crate::instruments::fx_option::FxOption;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};


pub struct BucketedDv01Calculator;

impl MetricCalculator for BucketedDv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let opt_ref: &FxOption = context.instrument_as()?;
        let opt = opt_ref.clone();

        // Two groups: domestic curve buckets and foreign curve buckets
        let buckets = crate::metrics::standard_ir_dv01_buckets();

        let as_of = context.as_of;

        // Domestic bucketed dv01 stored under custom base id
        let dom_total = crate::metrics::compute_key_rate_series_with_context_for_id(
            context,
            MetricId::custom("bucketed_dv01_domestic"),
            &opt.domestic_disc_id,
            buckets.iter().copied(),
            1.0,
            {
                let opt_dom = opt.clone();
                move |temp_ctx| opt_dom.npv(temp_ctx, as_of)
            },
        )?;

        // Store per-currency totals under composite keys
        // Note: series are already stored by helper under base metric id.

        // Foreign bucketed dv01 stored under custom base id
        let for_total = crate::metrics::compute_key_rate_series_with_context_for_id(
            context,
            MetricId::custom("bucketed_dv01_foreign"),
            &opt.foreign_disc_id,
            buckets.into_iter(),
            1.0,
            {
                let opt_for = opt.clone();
                move |temp_ctx| opt_for.npv(temp_ctx, as_of)
            },
        )?;

        Ok(dom_total + for_total)
    }
}
