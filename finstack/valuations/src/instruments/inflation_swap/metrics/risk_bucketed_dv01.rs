//! Bucketed DV01 for Inflation Swaps (discount curve sensitivity).

use crate::instruments::inflation_swap::types::InflationSwap;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::F;

pub struct BucketedDv01Calculator;

impl MetricCalculator for BucketedDv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let inst_ref: &InflationSwap = context.instrument_as()?;
        let inst = inst_ref.clone();
        let disc_id = finstack_core::types::CurveId::from(inst.disc_id);

        let labels: Vec<String> = crate::metrics::standard_ir_dv01_buckets()
            .iter()
            .map(|y| if *y < 1.0 { format!("{:.0}m", (y * 12.0).round()) } else { format!("{:.0}y", y) })
            .collect();

        let curves = context.curves.clone();
        let as_of = context.as_of;
        let reval = move |
            _bumped_disc: &finstack_core::market_data::term_structures::discount_curve::DiscountCurve|
         {
            let pv_fixed = crate::instruments::inflation_swap::pricing::engine::InflationSwapPricer::new()
                .pv_fixed_leg(&inst, &curves, as_of)?;
            // Inflation leg PV uses discount DF to maturity as well; reuse engine and then substitute discount where used
            let pv_infl = crate::instruments::inflation_swap::pricing::engine::InflationSwapPricer::new()
                .pv_inflation_leg(&inst, &curves, as_of)?;
            let net = match inst.side {
                crate::instruments::inflation_swap::types::PayReceiveInflation::ReceiveFixed => (pv_fixed - pv_infl)?,
                crate::instruments::inflation_swap::types::PayReceiveInflation::PayFixed => (pv_infl - pv_fixed)?,
            };
            Ok(net)
        };

        crate::metrics::compute_bucketed_dv01_series(
            context,
            &disc_id,
            labels,
            1.0,
            reval,
        )
    }
}


