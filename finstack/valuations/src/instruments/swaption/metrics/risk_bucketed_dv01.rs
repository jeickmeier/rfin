//! Bucketed DV01 for swaptions (discount curve sensitivity via revaluation).

use crate::instruments::swaption::Swaption;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::F;

pub struct BucketedDv01Calculator;

impl MetricCalculator for BucketedDv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let s_ref: &Swaption = context.instrument_as()?;
        let s = s_ref.clone();
        let disc_id = s.disc_id.clone();

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
            // Reuse swaption pricer functions but substitute discount curve directly
            let pricer = crate::instruments::swaption::pricing::SwaptionPricer;
            pricer.price_black(&s, bumped_disc, {
                // Use vol from overrides or surface
                let t = pricer.year_fraction(as_of, s.expiry, s.day_count)?;
                if let Some(v) = s.pricing_overrides.implied_volatility { v } else { curves.surface_ref(s.vol_id)?.value_clamped(t, s.strike_rate) }
            }, as_of)
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



