//! Common utilities for interest rate option metrics.
//!
//! Provides a DRY aggregation helper to iterate caplets/floorlets and sum
//! contributions for a given functional form (e.g., delta/gamma/vega/theta).

use crate::instruments::rates::cap_floor::InterestRateOption;
use crate::metrics::MetricContext;

const MIN_EFFECTIVE_FIXING_TIME: f64 = 1e-6;

/// Iterate over caplets/floorlets and aggregate contributions.
///
/// The supplied function `f` should return the "per-unit" measure for a single
/// caplet/floorlet given (forward, sigma, time_to_fixing). The helper will
/// scale by notional × period_length × df and sum across periods.
///
/// The fixing date used for vol surface lookup and t_fix computation is the
/// `reset_date` when provided (matching the pricer), falling back to
/// `accrual_start`. This ensures Greeks are computed on the same dates as
/// pricing, which matters for indices with non-zero reset lags (e.g., SOFR 2-day lookback).
pub fn aggregate_over_caplets<FN>(
    option: &InterestRateOption,
    context: &MetricContext,
    mut f: FN,
) -> finstack_core::Result<f64>
where
    FN: FnMut(f64, f64, f64) -> f64,
{
    // Get market curves
    let disc_curve = context
        .curves
        .get_discount(option.discount_curve_id.as_ref())?;
    let fwd_curve = context
        .curves
        .get_forward(option.forward_curve_id.as_ref())?;
    let strike = option.strike_f64()?;

    // Helper to compute contribution for a single period.
    // `fixing_date`: the date on which the rate fixes (used for vol lookup / t_fix).
    // `accrual_start` / `accrual_end`: the period boundaries used for the forward rate.
    let mut accumulate = |fixing_date: finstack_core::dates::Date,
                          accrual_start: finstack_core::dates::Date,
                          accrual_end: finstack_core::dates::Date,
                          payment_date: finstack_core::dates::Date,
                          tau: f64|
     -> finstack_core::Result<f64> {
        if fixing_date < context.as_of {
            return Ok(0.0);
        }

        let t_fix = option.day_count.year_fraction(
            context.as_of,
            fixing_date,
            finstack_core::dates::DayCountCtx::default(),
        )?;
        let effective_t_fix = t_fix.max(MIN_EFFECTIVE_FIXING_TIME);

        let forward = crate::instruments::common_impl::pricing::time::rate_period_on_dates(
            fwd_curve.as_ref(),
            accrual_start,
            accrual_end,
        )?;
        let df = crate::instruments::common_impl::pricing::time::relative_df_discount_curve(
            disc_curve.as_ref(),
            context.as_of,
            payment_date,
        )?;
        let sigma = context
            .curves
            .get_surface(option.vol_surface_id.as_str())?
            .value_clamped(effective_t_fix, strike);

        let per_unit = f(forward, sigma, effective_t_fix);
        Ok(per_unit * option.notional.amount() * tau * df)
    };

    let periods = option.pricing_periods()?;
    if periods.is_empty() {
        return Ok(0.0);
    }

    let mut sum = 0.0;
    for period in periods {
        // Use reset_date (actual fixing date) when available, matching the pricer.
        let fixing_date = period.reset_date.unwrap_or(period.accrual_start);
        sum += accumulate(
            fixing_date,
            period.accrual_start,
            period.accrual_end,
            period.payment_date,
            period.accrual_year_fraction,
        )?;
    }
    Ok(sum)
}
