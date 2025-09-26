//! Bucketed DV01 for IR Futures using discount curve bumps.

use crate::instruments::ir_future::types::InterestRateFuture;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::F;

pub struct BucketedDv01Calculator;

impl MetricCalculator for BucketedDv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let f_ref: &InterestRateFuture = context.instrument_as()?;
        let fut = f_ref.clone();
        let disc_id = fut.disc_id.clone();

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
        let reval = move |
            bumped_disc: &finstack_core::market_data::term_structures::discount_curve::DiscountCurve|
         {
            use finstack_core::dates::DayCountCtx;

            let fwd = curves.get_forward_ref(fut.forward_id.clone())?;

            let base_date = bumped_disc.base_date();
            let t_fixing = fut
                .day_count
                .year_fraction(base_date, fut.fixing_date, DayCountCtx::default())?
                .max(0.0);
            let t_start = fut
                .day_count
                .year_fraction(base_date, fut.period_start, DayCountCtx::default())?
                .max(0.0);
            let t_end = fut
                .day_count
                .year_fraction(base_date, fut.period_end, DayCountCtx::default())?
                .max(t_start);

            let forward_rate = fwd.rate_period(t_start, t_end);
            let adjusted_rate = if let Some(ca) = fut.contract_specs.convexity_adjustment {
                forward_rate + ca
            } else {
                let vol_estimate = if t_fixing <= 0.25 {
                    0.008
                } else if t_fixing <= 0.5 {
                    0.0085
                } else if t_fixing <= 1.0 {
                    0.009
                } else if t_fixing <= 2.0 {
                    0.0095
                } else {
                    0.01
                };
                let tau_len = t_end - t_start;
                let convexity = 0.5 * vol_estimate * vol_estimate * t_fixing * (t_fixing + tau_len);
                forward_rate + convexity
            };

            let implied_rate = fut.implied_rate();
            let tau = fut
                .day_count
                .year_fraction(fut.period_start, fut.period_end, DayCountCtx::default())?
                .max(0.0);
            if tau == 0.0 {
                return Ok(finstack_core::money::Money::new(0.0, fut.notional.currency()));
            }

            let sign = match fut.position {
                crate::instruments::ir_future::types::Position::Long => 1.0,
                crate::instruments::ir_future::types::Position::Short => -1.0,
            };
            let contracts_scale = if fut.contract_specs.face_value != 0.0 {
                fut.notional.amount() / fut.contract_specs.face_value
            } else {
                1.0
            };
            let pv_per_contract = (implied_rate - adjusted_rate) * fut.contract_specs.face_value * tau;
            let pv_total = sign * contracts_scale * pv_per_contract;
            Ok(finstack_core::money::Money::new(pv_total, fut.notional.currency()))
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


