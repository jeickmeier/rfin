//! Bucketed DV01 for CDS Options (discount curve sensitivity via revaluation).

use crate::instruments::cds_option::CdsOption;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::F;

pub struct BucketedDv01Calculator;

impl MetricCalculator for BucketedDv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let opt_ref: &CdsOption = context.instrument_as()?;
        let opt = opt_ref.clone();
        let disc_id = opt.disc_id.clone();

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
            use crate::instruments::cds_option::pricing::engine::CdsOptionPricer;
            let hazard = curves.get_hazard_ref(opt.credit_id.clone())?;
            let pricer = CdsOptionPricer::default();
            let t = opt.day_count.year_fraction(as_of, opt.expiry, finstack_core::dates::DayCountCtx::default())?;
            let forward_bp = pricer.forward_spread_bp(&opt, &curves, as_of)?;
            let cds = crate::instruments::cds::CreditDefaultSwap::new_isda(
                "CDS-UND",
                finstack_core::money::Money::new(opt.notional.amount(), opt.notional.currency()),
                crate::instruments::cds::PayReceive::PayProtection,
                crate::instruments::cds::CDSConvention::IsdaNa,
                0.0,
                opt.expiry,
                opt.cds_maturity,
                opt.recovery_rate,
                opt.disc_id.clone(),
                opt.credit_id.clone(),
            );
            let cds_pricer = crate::instruments::cds::pricing::engine::CDSPricer::new();
            let ra = cds_pricer.risky_annuity(&cds, bumped_disc, hazard, as_of)?;
            let df = bumped_disc.df(t);
            pricer.credit_option_price(&opt, forward_bp, df, ra, {
                if let Some(v) = opt.pricing_overrides.implied_volatility { v } else { curves.surface_ref(opt.vol_id)?.value_clamped(t, opt.strike_spread_bp) }
            }, t)
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


