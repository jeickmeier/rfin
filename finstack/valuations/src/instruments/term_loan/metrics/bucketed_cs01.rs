//! Bucketed CS01 for term loans using discount-curve key-rate bumps.

use crate::instruments::common::traits::Instrument;
use crate::instruments::TermLoan;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::market_data::bumps::BumpSpec;
use hashbrown::HashMap;

#[derive(Debug, Default, Clone, Copy)]
pub struct BucketedCs01Calculator;

impl MetricCalculator for BucketedCs01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let loan: &TermLoan = context.instrument_as()?;
        let as_of = context.as_of;
        let base_ctx = context.curves.as_ref();

        // Use standard IR buckets for CS01
        let buckets = crate::metrics::standard_ir_dv01_buckets();
        let discount_curve_id = &loan.discount_curve_id;

        // Recalculate base PV
        let base_pv = loan.value(base_ctx, as_of)?;

        let mut series: Vec<(String, f64)> = Vec::new();

        for &time_years in &buckets {
            let label = format_bucket_label(time_years);

            // Create key-rate bump spec
            let mut bumps = HashMap::new();
            bumps.insert(
                discount_curve_id.clone(),
                BumpSpec::key_rate_bp(time_years, 1.0),
            );

            let bumped_ctx = base_ctx.bump(bumps)?;
            let bumped_pv = loan.value(&bumped_ctx, as_of)?;
            let cs01 = (bumped_pv.amount() - base_pv.amount()) / 1.0;

            series.push((label, cs01));
        }

        context.store_bucketed_series(MetricId::BucketedCs01, series.clone());
        let total: f64 = series.iter().map(|(_, v)| *v).sum();
        Ok(total)
    }
}

/// Generate bucket label from years.
#[inline]
fn format_bucket_label(years: f64) -> String {
    if years < 1.0 {
        format!("{:.0}m", (years * 12.0).round())
    } else {
        format!("{:.0}y", years)
    }
}
