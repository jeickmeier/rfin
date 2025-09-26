//! Bucketed DV01 calculator for IRS using structured metric storage.

use crate::instruments::irs::types::InterestRateSwap;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::F;

/// Compute Bucketed DV01 for IRS by revaluing against a bumped discount curve
/// for each standard IR bucket. Stores series via `MetricContext.store_bucketed_series`
/// and returns the total DV01.
pub struct BucketedDv01Calculator;

impl MetricCalculator for BucketedDv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let irs_ref: &InterestRateSwap = context.instrument_as()?;
        let irs = irs_ref.clone();
        let disc_id = irs.fixed.disc_id.clone();

        // Standard IR buckets to string labels
        let buckets = crate::metrics::standard_ir_dv01_buckets();
        let labels: Vec<String> = buckets
            .iter()
            .map(|y| if *y < 1.0 { format!("{:.0}m", (y * 12.0).round()) } else { format!("{:.0}y", y) })
            .collect();

        // Map label → (t1, t2) in years (placeholder for future segment bumps)

        // Revaluation with bumped discount curve: recompute PV of both legs using bumped disc and original forward curve
        let curves = context.curves.clone();
        let reval = move |
            bumped_disc: &finstack_core::market_data::term_structures::discount_curve::DiscountCurve|
         {
            let fwd = curves.get_forward_ref(irs.float.fwd_id.as_str())?;

            // Compute leg PVs directly using helper methods to ensure discounting uses bumped curve
            let pv_fixed = irs.pv_fixed_leg(bumped_disc)?;
            let pv_float = irs.pv_float_leg(bumped_disc, fwd)?;

            let npv = match irs.side {
                crate::instruments::irs::types::PayReceive::PayFixed => (pv_float - pv_fixed)?,
                crate::instruments::irs::types::PayReceive::ReceiveFixed => (pv_fixed - pv_float)?,
            };
            Ok(npv)
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


