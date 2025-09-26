//! Bucketed DV01 calculator for bonds using structured metric storage.

use crate::instruments::bond::types::Bond;
use crate::metrics::{MetricCalculator, MetricContext};
use crate::cashflow::traits::CashflowProvider;
use finstack_core::F;

/// Compute Bucketed DV01 for bonds by revaluing against a bumped discount curve
/// for each standard IR bucket. Stores series via `MetricContext.store_bucketed_series`
/// and returns the total DV01.
pub struct BucketedDv01Calculator;

impl MetricCalculator for BucketedDv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let bond_ref: &Bond = context.instrument_as()?;
        let bond = bond_ref.clone();
        let disc_id = bond.disc_id.clone();

        // Use standard bucket labels mapped from years to string labels
        let buckets = crate::metrics::standard_ir_dv01_buckets();
        let labels: Vec<String> = buckets
            .iter()
            .map(|y| {
                if *y < 1.0 {
                    format!("{:.0}m", (y * 12.0).round())
                } else {
                    format!("{:.0}y", y)
                }
            })
            .collect();


        // Revaluation closure using original flows and bumped discount curve
        let curves = context.curves.clone();
        let as_of = context.as_of;
        let reval = move |
            bumped_disc: &finstack_core::market_data::term_structures::discount_curve::DiscountCurve|
         {
            // Build flows using original curves (for FRN coupons if any), then re-discount
            let flows = bond.build_schedule(&curves, as_of)?;
            let base = bumped_disc.base_date();
            let dc = bumped_disc.day_count();
            crate::instruments::common::discountable::npv_static(
                bumped_disc,
                base,
                dc,
                &flows,
            )
        };

        let total = crate::metrics::compute_bucketed_dv01_series(
            context,
            &disc_id,
            labels,
            1.0,
            reval,
        )?;

        Ok(total)
    }
}


