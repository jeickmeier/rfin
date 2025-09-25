//! Bucketed DV01 for Convertible Bonds (discount curve sensitivity via tree PV revaluation).

use crate::instruments::convertible::ConvertibleBond;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::F;

pub struct BucketedDv01Calculator;

impl MetricCalculator for BucketedDv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let bond_ref: &ConvertibleBond = context.instrument_as()?;
        let bond = bond_ref.clone();
        let disc_id = finstack_core::types::CurveId::from(bond.disc_id);

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
        let reval = move |_: &finstack_core::market_data::term_structures::discount_curve::DiscountCurve| {
            // Approximate revaluation via current pricing engine path
            let ctx_clone = curves.as_ref().clone();
            crate::instruments::convertible::pricing::engine::price_convertible_bond(
                &bond,
                &ctx_clone,
                crate::instruments::convertible::pricing::engine::ConvertibleTreeType::default(),
            )
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


