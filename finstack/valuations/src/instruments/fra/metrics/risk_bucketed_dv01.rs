//! Bucketed DV01 calculator for FRAs using structured metric storage.

use crate::instruments::fra::types::ForwardRateAgreement;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::F;

pub struct BucketedDv01Calculator;

impl MetricCalculator for BucketedDv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let fra_ref: &ForwardRateAgreement = context.instrument_as()?;
        let fra = fra_ref.clone();
        let disc_id = fra.disc_id.clone();

        // Standard IR buckets to labels
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

        let curves = context.curves.clone();
        let reval = move |
            bumped_disc: &finstack_core::market_data::term_structures::discount_curve::DiscountCurve|
         {
            let fwd = curves.get_forward_ref(fra.forward_id.as_str())?;

            // Recompute FRA PV using bumped discount and original forward
            let base_date = bumped_disc.base_date();
            let t_start = fra
                .day_count
                .year_fraction(base_date, fra.start_date, finstack_core::dates::DayCountCtx::default())?
                .max(0.0);
            let t_end = fra
                .day_count
                .year_fraction(base_date, fra.end_date, finstack_core::dates::DayCountCtx::default())?
                .max(t_start);
            let tau = fra
                .day_count
                .year_fraction(fra.start_date, fra.end_date, finstack_core::dates::DayCountCtx::default())?
                .max(0.0);
            if tau == 0.0 {
                return Ok(finstack_core::money::Money::new(0.0, fra.notional.currency()));
            }
            let forward_rate = fwd.rate_period(t_start, t_end);
            let df_settlement = bumped_disc.df_on_date_curve(fra.start_date);
            let rate_diff = forward_rate - fra.fixed_rate;
            let pv = fra.notional.amount() * rate_diff * tau * df_settlement;
            let signed_pv = if fra.pay_fixed { -pv } else { pv };
            Ok(finstack_core::money::Money::new(signed_pv, fra.notional.currency()))
        };

        let total =
            crate::metrics::compute_bucketed_dv01_series(context, &disc_id, labels, 1.0, reval)?;

        Ok(total)
    }
}
