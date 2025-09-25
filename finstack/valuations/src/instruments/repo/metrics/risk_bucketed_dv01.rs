//! Bucketed DV01 for Repo (discount curve sensitivity across tenor buckets).

use crate::instruments::repo::Repo;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::F;

pub struct BucketedDv01Calculator;

impl MetricCalculator for BucketedDv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let repo_ref: &Repo = context.instrument_as()?;
        let repo = repo_ref.clone();
        let disc_id = finstack_core::types::CurveId::from(repo.disc_id);

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

        let as_of = context.as_of;
        crate::metrics::compute_bucketed_dv01_series_with_context(
            context,
            &disc_id,
            labels,
            map_label,
            1.0,
            move |temp_ctx| {
                crate::instruments::repo::pricing::engine::RepoPricer::new().pv(&repo, temp_ctx, as_of)
            },
        )
    }
}


