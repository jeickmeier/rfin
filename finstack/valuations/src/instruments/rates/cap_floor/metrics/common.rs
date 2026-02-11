//! Common utilities for interest rate option metrics.
//!
//! Provides a DRY aggregation helper to iterate caplets/floorlets and sum
//! contributions for a given functional form (e.g., delta/gamma/vega/theta).

use crate::instruments::rates::cap_floor::InterestRateOption;
use crate::market::conventions::ids::IndexId;
use crate::market::conventions::ConventionRegistry;
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
    let disc_curve = context
        .curves
        .get_discount(option.discount_curve_id.as_ref())?;
    let fwd_curve = context.curves.get_forward(option.forward_id.as_ref())?;

    // Helper to compute contribution for a single period
    let mut accumulate = |start: finstack_core::dates::Date,
                          end: finstack_core::dates::Date,
                          payment_date: finstack_core::dates::Date,
                          tau: f64|
     -> finstack_core::Result<f64> {
        let t_fix = option.day_count.year_fraction(
            context.as_of,
            start,
            finstack_core::dates::DayCountCtx::default(),
        )?;

        if t_fix <= 0.0 {
            return Ok(0.0);
        }

        let forward = crate::instruments::common_impl::pricing::time::rate_period_on_dates(
            fwd_curve.as_ref(),
            start,
            end,
        )?;
        let df = crate::instruments::common_impl::pricing::time::relative_df_discount_curve(
            disc_curve.as_ref(),
            context.as_of,
            payment_date,
        )?;
        let sigma = if let Some(impl_vol) = option.pricing_overrides.implied_volatility {
            impl_vol
        } else {
            context
                .curves
                .surface(option.vol_surface_id.as_str())?
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
        let payment_date = crate::cashflow::builder::calendar::adjust_date(
            option.end_date,
            option.bdc,
            option
                .calendar_id
                .as_deref()
                .unwrap_or(crate::cashflow::builder::calendar::WEEKENDS_ONLY_ID),
        )?;
        let tau = option.day_count.year_fraction(
            option.start_date,
            option.end_date,
            finstack_core::dates::DayCountCtx::default(),
        )?;
        return accumulate(option.start_date, option.end_date, payment_date, tau);
    }

    // Cap/floor: iterate schedule
    let (payment_lag_days, reset_lag_days) = match ConventionRegistry::try_global() {
        Ok(registry) => {
            let idx = IndexId::new(option.forward_id.as_str());
            match registry.require_rate_index(&idx) {
                Ok(conv) => (
                    conv.default_payment_delay_days,
                    Some(conv.default_reset_lag_days),
                ),
                Err(_) => (0, None),
            }
        }
        Err(_) => (0, None),
    };
    let periods = crate::cashflow::builder::periods::build_periods(
        crate::cashflow::builder::periods::BuildPeriodsParams {
            start: option.start_date,
            end: option.end_date,
            frequency: option.frequency,
            stub: option.stub_kind,
            bdc: option.bdc,
            calendar_id: option
                .calendar_id
                .as_deref()
                .unwrap_or(crate::cashflow::builder::calendar::WEEKENDS_ONLY_ID),
            end_of_month: false,
            day_count: option.day_count,
            payment_lag_days,
            reset_lag_days,
        },
    )?;
    if periods.is_empty() {
        return Ok(0.0);
    }

    let mut sum = 0.0;
    for period in periods {
        sum += accumulate(
            period.accrual_start,
            period.accrual_end,
            period.payment_date,
            period.accrual_year_fraction,
        )?;
    }
    Ok(sum)
}
