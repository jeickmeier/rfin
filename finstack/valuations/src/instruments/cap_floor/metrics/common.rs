//! Common utilities for interest rate option metrics.
//!
//! Provides a DRY aggregation helper to iterate caplets/floorlets and sum
//! contributions for a given functional form (e.g., delta/gamma/vega/theta).

use crate::instruments::cap_floor::InterestRateOption;
use crate::metrics::MetricContext;


/// Iterate over caplets/floorlets and aggregate contributions.
///
/// The supplied function `f` should return the "per-unit" measure for a single
/// caplet/floorlet given (forward, sigma, time_to_fixing). The helper will
/// scale by notional × period_length × df and sum across periods.
pub fn aggregate_over_caplets<FN>(
    option: &InterestRateOption,
    context: &MetricContext,
    mut f: FN,
) -> finstack_core::Result<f64>
where
    FN: FnMut(f64, f64, f64) -> f64,
{
    // Get market curves
    let disc_curve = context.curves.get_discount_ref(option.disc_id.as_ref())?;
    let fwd_curve = context.curves.get_forward_ref(option.forward_id.as_ref())?;
    let base_date = disc_curve.base_date();

    // Helper to compute contribution for a single period
    let mut accumulate = |start: finstack_core::dates::Date,
                          end: finstack_core::dates::Date|
     -> finstack_core::Result<f64> {
        let t_fix = option.day_count.year_fraction(
            base_date,
            start,
            finstack_core::dates::DayCountCtx::default(),
        )?;
        let t_pay = option.day_count.year_fraction(
            base_date,
            end,
            finstack_core::dates::DayCountCtx::default(),
        )?;
        let tau = option.day_count.year_fraction(
            start,
            end,
            finstack_core::dates::DayCountCtx::default(),
        )?;

        if t_fix <= 0.0 {
            return Ok(0.0);
        }

        let forward = fwd_curve.rate_period(t_fix, t_pay);
        let df = disc_curve.df(t_pay);
        let sigma = if let Some(impl_vol) = option.pricing_overrides.implied_volatility {
            impl_vol
        } else {
            context
                .curves
                .surface_ref(option.vol_id)?
                .value_clamped(t_fix, option.strike_rate)
        };

        let per_unit = f(forward, sigma, t_fix);
        Ok(per_unit * option.notional.amount() * tau * df)
    };

    // Caplet/floorlet: single period
    if matches!(
        option.rate_option_type,
        super::super::RateOptionType::Caplet | super::super::RateOptionType::Floorlet
    ) {
        return accumulate(option.start_date, option.end_date);
    }

    // Cap/floor: iterate schedule
    use crate::cashflow::builder::schedule_utils::build_dates;
    let schedule = build_dates(
        option.start_date,
        option.end_date,
        option.frequency,
        option.stub_kind,
        option.bdc,
        option.calendar_id,
    );
    if schedule.dates.len() < 2 {
        return Ok(0.0);
    }

    let mut sum = 0.0;
    let mut prev = schedule.dates[0];
    for &pay in &schedule.dates[1..] {
        sum += accumulate(prev, pay)?;
        prev = pay;
    }
    Ok(sum)
}
