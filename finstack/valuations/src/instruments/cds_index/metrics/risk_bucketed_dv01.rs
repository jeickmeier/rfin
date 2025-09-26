//! Bucketed DV01 for CDS Index premium discount sensitivity.

use crate::instruments::cds_index::types::CDSIndex;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::F;

pub struct BucketedDv01Calculator;

impl MetricCalculator for BucketedDv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let idx_ref: &CDSIndex = context.instrument_as()?;
        let index = idx_ref.clone();
        let disc_id = index.premium.disc_id.clone();

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

        let curves = context.curves.clone();
        let as_of = context.as_of;
        let reval = move |
            bumped_disc: &finstack_core::market_data::term_structures::discount_curve::DiscountCurve|
         {
            use crate::instruments::cds_index::pricing::engine::CDSIndexPricer;
            let _ = CDSIndexPricer::new();
            // Reuse engine NPV by substituting discount in engine internals: call pv_legs-like path via npv with discount in MarketContext is not exposed,
            // but we can price SingleCurve by converting to synthetic CDS and calling CDS pricer directly.
            match index.pricing {
                crate::instruments::cds_index::types::IndexPricing::SingleCurve => {
                    let cds = index.to_synthetic_cds();
                    let surv = curves.get_hazard_ref(cds.protection.credit_id.clone())?;
                    let cds_pricer = crate::instruments::cds::pricing::engine::CDSPricer::new();
                    cds_pricer.npv(&cds, bumped_disc, surv, as_of)
                }
                crate::instruments::cds_index::types::IndexPricing::Constituents => {
                    // Approximate by pricing constituents individually with bumped discount
                    let mut sum = finstack_core::money::Money::new(0.0, index.notional.currency());
                    // Build synthetic constituents via public API: iterate declared constituents
                    if index.constituents.is_empty() {
                        return Ok(sum);
                    }
                    let cds_pricer = crate::instruments::cds::pricing::engine::CDSPricer::new();
                    for cons in &index.constituents {
                        let cds = crate::instruments::cds::CreditDefaultSwap::buy_protection(
                            index.id.clone(),
                            finstack_core::money::Money::new(index.notional.amount() * cons.weight, index.notional.currency()),
                            index.premium.spread_bp,
                            index.premium.start,
                            index.premium.end,
                            index.premium.disc_id.clone(),
                            cons.credit.credit_curve_id.clone(),
                        );
                        let surv = curves.get_hazard_ref(cds.protection.credit_id.clone())?;
                        sum = (sum + cds_pricer.npv(&cds, bumped_disc, surv, as_of)?)?;
                    }
                    Ok(sum)
                }
            }
        };

        crate::metrics::compute_bucketed_dv01_series(
            context,
            &disc_id,
            labels,
            map_label,
            1.0,
            reval,
        )
    }
}


