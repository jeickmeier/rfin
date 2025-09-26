//! Bucketed DV01 for InterestRateOption (cap/floor) using discount curve bumps.

use crate::instruments::cap_floor::types::InterestRateOption;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::F;

pub struct BucketedDv01Calculator;

impl MetricCalculator for BucketedDv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let opt_ref: &InterestRateOption = context.instrument_as()?;
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
            use crate::cashflow::builder::schedule_utils::build_dates;
            use crate::instruments::cap_floor::pricing::black as black_ir;
            use crate::instruments::cap_floor::types::RateOptionType;

            let fwd_curve = curves.get_forward_ref(opt.forward_id.as_ref())?;

            let mut total_pv = finstack_core::money::Money::new(0.0, opt.notional.currency());

            // Single period caplet/floorlet
            if matches!(
                opt.rate_option_type,
                RateOptionType::Caplet | RateOptionType::Floorlet
            ) {
                let t_fix = opt
                    .day_count
                    .year_fraction(as_of, opt.start_date, finstack_core::dates::DayCountCtx::default())?
                    .max(0.0);
                let t_pay = opt
                    .day_count
                    .year_fraction(as_of, opt.end_date, finstack_core::dates::DayCountCtx::default())?;
                let tau = opt
                    .day_count
                    .year_fraction(opt.start_date, opt.end_date, finstack_core::dates::DayCountCtx::default())?;
                let forward = fwd_curve.rate_period(t_fix, t_pay);
                let df = bumped_disc.df(t_pay);
                let sigma = if let Some(impl_vol) = opt.pricing_overrides.implied_volatility {
                    impl_vol
                } else {
                    curves.surface_ref(opt.vol_id)?.value_clamped(t_fix, opt.strike_rate)
                };
                let is_cap = matches!(opt.rate_option_type, RateOptionType::Caplet | RateOptionType::Cap);
                total_pv = black_ir::price_caplet_floorlet(black_ir::CapletFloorletInputs {
                    is_cap,
                    notional: opt.notional.amount(),
                    strike: opt.strike_rate,
                    forward,
                    discount_factor: df,
                    volatility: sigma,
                    time_to_fixing: t_fix,
                    accrual_year_fraction: tau,
                    currency: opt.notional.currency(),
                })?;
                return Ok(total_pv);
            }

            // Multi-period cap/floor
            let schedule = build_dates(
                opt.start_date,
                opt.end_date,
                opt.frequency,
                opt.stub_kind,
                opt.bdc,
                opt.calendar_id,
            );
            if schedule.dates.len() < 2 {
                return Ok(total_pv);
            }
            let is_cap = matches!(opt.rate_option_type, RateOptionType::Caplet | RateOptionType::Cap);
            let mut prev = schedule.dates[0];
            for &pay in &schedule.dates[1..] {
                let t_fix = opt
                    .day_count
                    .year_fraction(as_of, prev, finstack_core::dates::DayCountCtx::default())?;
                let t_pay = opt
                    .day_count
                    .year_fraction(as_of, pay, finstack_core::dates::DayCountCtx::default())?;
                let tau = opt
                    .day_count
                    .year_fraction(prev, pay, finstack_core::dates::DayCountCtx::default())?;
                if t_fix > 0.0 {
                    let forward = fwd_curve.rate_period(t_fix, t_pay);
                    let df = bumped_disc.df(t_pay);
                    let sigma = if let Some(impl_vol) = opt.pricing_overrides.implied_volatility {
                        impl_vol
                    } else {
                        curves.surface_ref(opt.vol_id)?.value_clamped(t_fix, opt.strike_rate)
                    };
                    let leg_pv = black_ir::price_caplet_floorlet(black_ir::CapletFloorletInputs {
                        is_cap,
                        notional: opt.notional.amount(),
                        strike: opt.strike_rate,
                        forward,
                        discount_factor: df,
                        volatility: sigma,
                        time_to_fixing: t_fix,
                        accrual_year_fraction: tau,
                        currency: opt.notional.currency(),
                    })?;
                    total_pv = (total_pv + leg_pv)?;
                }
                prev = pay;
            }
            Ok(total_pv)
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


