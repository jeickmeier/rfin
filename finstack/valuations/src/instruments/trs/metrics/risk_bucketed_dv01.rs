//! Bucketed DV01 for FIIndexTotalReturnSwap financing leg (discount curve sensitivity).

use crate::instruments::trs::FIIndexTotalReturnSwap;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::F;

pub struct BucketedDv01Calculator;

impl MetricCalculator for BucketedDv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        // Only implement for FIIndexTotalReturnSwap; Equity TRS DV01 wrt discount is zero under current model
        if let Ok(trs) = context.instrument_as::<FIIndexTotalReturnSwap>() {
            let disc_id = finstack_core::types::CurveId::from(trs.financing.disc_id.as_str());

            let labels: Vec<String> = crate::metrics::standard_ir_dv01_buckets()
                .iter()
                .map(|y| if *y < 1.0 { format!("{:.0}m", (y * 12.0).round()) } else { format!("{:.0}y", y) })
                .collect();

            let curves = context.curves.clone();
            let as_of = context.as_of;
            let trs_clone = trs.clone();
            let reval = move |
                bumped_disc: &finstack_core::market_data::term_structures::discount_curve::DiscountCurve|
             {
                use crate::instruments::trs::pricing::engine::TrsEngine;
                use crate::instruments::trs::pricing::fixed_income_index::pv_total_return_leg;

                // Total return leg PV unchanged vs discount; financing leg PV with bumped disc
                let tr_pv = pv_total_return_leg(&trs_clone, &curves, as_of)?;

                // Financing leg PV using bumped disc for discounting
                let disc_curve_id = bumped_disc.id().clone();
                let mut financing = trs_clone.financing.clone();
                financing.disc_id = disc_curve_id;
                let fin_pv = TrsEngine::pv_financing_leg(&financing, &trs_clone.schedule, trs_clone.notional, &curves, as_of)?;

                // Net PV depends on side
                let net = match trs_clone.side {
                    crate::instruments::trs::TrsSide::ReceiveTotalReturn => (tr_pv - fin_pv)?,
                    crate::instruments::trs::TrsSide::PayTotalReturn => (fin_pv - tr_pv)?,
                };
                Ok(net)
            };

            return crate::metrics::compute_bucketed_dv01_series(
                context,
                &disc_id,
                labels,
                1.0,
                reval,
            );
        }
        Ok(0.0)
    }
}


