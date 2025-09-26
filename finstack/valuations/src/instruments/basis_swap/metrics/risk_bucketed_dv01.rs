//! Bucketed DV01 for BasisSwap (discount curve sensitivity on PV).

use crate::instruments::basis_swap::BasisSwap;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::F;

pub struct BucketedDv01Calculator;

impl MetricCalculator for BucketedDv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let swap_ref: &BasisSwap = context.instrument_as()?;
        let swap = swap_ref.clone();
        let disc_id = swap.discount_curve_id.clone();

        // Standard labels
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
        let as_of = context.as_of;
        let reval = move |
            bumped_disc: &finstack_core::market_data::term_structures::discount_curve::DiscountCurve|
         {
            use crate::instruments::basis_swap::pricing::engine::{BasisEngine, FloatLegParams};

            // Build schedules
            let prim_sched = swap.leg_schedule(&swap.primary_leg);
            let ref_sched = swap.leg_schedule(&swap.reference_leg);

            // Compose leg params with bumped disc
            let prim = FloatLegParams {
                schedule: &prim_sched,
                notional: swap.notional,
                disc_id: bumped_disc.id().clone(),
                fwd_id: swap.primary_leg.forward_curve_id.clone(),
                accrual_dc: swap.primary_leg.day_count,
                spread: swap.primary_leg.spread,
            };
            let refe = FloatLegParams {
                schedule: &ref_sched,
                notional: swap.notional,
                disc_id: bumped_disc.id().clone(),
                fwd_id: swap.reference_leg.forward_curve_id.clone(),
                accrual_dc: swap.reference_leg.day_count,
                spread: swap.reference_leg.spread,
            };

            let pv_primary = BasisEngine::pv_float_leg(prim, &curves, as_of)?;
            let pv_reference = BasisEngine::pv_float_leg(refe, &curves, as_of)?;
            pv_primary - pv_reference
        };

        crate::metrics::compute_bucketed_dv01_series(context, &disc_id, labels, 1.0, reval)
    }
}
