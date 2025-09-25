//! Bucketed DV01 for FX Options with separate domestic and foreign buckets.

use crate::instruments::fx_option::FxOption;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::F;

pub struct BucketedDv01Calculator;

impl MetricCalculator for BucketedDv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let opt_ref: &FxOption = context.instrument_as()?;
        let opt = opt_ref.clone();

        // Two groups: domestic curve buckets and foreign curve buckets
        let buckets = crate::metrics::standard_ir_dv01_buckets();
        let labels: Vec<String> = buckets
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

        let as_of = context.as_of;

        // Domestic bucketed dv01 stored under custom base id
        let dom_total = crate::metrics::compute_bucketed_series_with_context_for_id(
            context,
            MetricId::custom("bucketed_dv01_domestic"),
            &finstack_core::types::CurveId::from(opt.domestic_disc_id),
            labels.clone(),
            map_label,
            1.0,
            {
                let opt_dom = opt.clone();
                move |temp_ctx| {
                    crate::instruments::fx_option::pricing::engine::FxOptionPricer::npv(
                        &opt_dom,
                        temp_ctx,
                        as_of,
                    )
                }
            },
        )?;

        // Store per-currency totals under composite keys
        // Note: series are already stored by helper under base metric id.

        // Foreign bucketed dv01 stored under custom base id
        let for_total = crate::metrics::compute_bucketed_series_with_context_for_id(
            context,
            MetricId::custom("bucketed_dv01_foreign"),
            &finstack_core::types::CurveId::from(opt.foreign_disc_id),
            labels.clone(),
            map_label,
            1.0,
            {
                let opt_for = opt.clone();
                move |temp_ctx| {
                    crate::instruments::fx_option::pricing::engine::FxOptionPricer::npv(
                        &opt_for,
                        temp_ctx,
                        as_of,
                    )
                }
            },
        )?;

        Ok(dom_total + for_total)
    }
}


