//! MtM-resetting cross-currency swap PV path.
//!
//! Implements the cashflow stream in `docs/superpowers/specs/2026-05-10-xccy-mtm-reset-design.md`
//! under the CIP no-FX-vol approximation. The constant leg behaves like a vanilla fixed-notional
//! XCCY leg; the resetting leg's notional is re-marked at each accrual-period start using
//! `N_j^R = N_C / X_j^FRA` where `X_j^FRA = X_0 * P_C(T_j) / P_R(T_j)`. Rebalancing cashflows
//! are emitted in both currencies on each reset date.
//!
//! The whole PV reduces to a single Neumaier-accumulated sum of reporting-currency-converted
//! discounted cashflows, with no additional FX surface required beyond what
//! `pv_leg_in_reporting_ccy` already needs for fixed-notional XCCY.

use crate::cashflow::builder::periods::{build_periods, BuildPeriodsParams};
use crate::instruments::common_impl::pricing::swap_legs::robust_relative_df;
use crate::instruments::common_impl::pricing::time::rate_period_on_dates;
use crate::instruments::common_impl::numeric::decimal_to_f64;
use crate::instruments::rates::xccy_swap::types::{ResettingSide, XccySwap};
use finstack_core::dates::Date;
use finstack_core::math::summation::NeumaierAccumulator;
use finstack_core::money::fx::FxQuery;
use finstack_core::money::Money;
use finstack_core::Result;

/// Compute the PV of an MtM-resetting XCCY swap in reporting currency.
///
/// Dispatched from `XccySwap::base_value` when `notional_exchange` is `MtmResetting`.
/// Assumes the swap has been validated (schedules aligned, legs in distinct currencies,
/// FX matrix reachable).
pub(crate) fn pv_mtm_reset(
    swap: &XccySwap,
    resetting_side: ResettingSide,
    context: &finstack_core::market_data::context::MarketContext,
    as_of: Date,
) -> Result<Money> {
    let (constant_leg, resetting_leg) = swap.partition_legs(resetting_side)?;

    let disc_c = context.get_discount(&constant_leg.discount_curve_id)?;
    let disc_r = context.get_discount(&resetting_leg.discount_curve_id)?;
    let fwd_c = context.get_forward(&constant_leg.forward_curve_id)?;
    let fwd_r = context.get_forward(&resetting_leg.forward_curve_id)?;

    let fx = context.fx().ok_or_else(|| {
        finstack_core::Error::Validation(format!(
            "XccySwap '{}': MtM-reset PV requires an FxMatrix in the MarketContext",
            swap.id
        ))
    })?;

    let n_c = constant_leg.notional.amount();
    let reporting_ccy = swap.reporting_currency;

    // Spot FX (resetting -> constant) at `as_of`.
    let spot_x = fx
        .rate(FxQuery::new(
            resetting_leg.currency,
            constant_leg.currency,
            as_of,
        ))?
        .rate;

    // Build the shared schedule (aligned per `XccySwap::validate`).
    let leg_cal_id = constant_leg
        .calendar_id
        .as_deref()
        .unwrap_or(crate::cashflow::builder::calendar::WEEKENDS_ONLY_ID);
    let periods = build_periods(BuildPeriodsParams {
        start: constant_leg.start,
        end: constant_leg.end,
        frequency: constant_leg.frequency,
        stub: constant_leg.stub,
        bdc: constant_leg.bdc,
        calendar_id: leg_cal_id,
        end_of_month: false,
        day_count: constant_leg.day_count,
        payment_lag_days: constant_leg.payment_lag_days,
        reset_lag_days: constant_leg.reset_lag_days,
        adjust_accrual_dates: false,
    })?;

    if periods.is_empty() {
        return Ok(Money::new(0.0, reporting_ccy));
    }

    let mut pv = NeumaierAccumulator::new();

    // Helper: convert a cashflow at `payment_date` to reporting currency.
    let convert =
        |amount: f64, from_ccy: finstack_core::currency::Currency, payment_date: Date| -> Result<f64> {
            if from_ccy == reporting_ccy {
                return Ok(amount);
            }
            let rate = fx
                .rate(FxQuery::new(from_ccy, reporting_ccy, payment_date))?
                .rate;
            Ok(amount * rate)
        };

    // Initial principal exchange at start. We use `initial_principal_sign` exactly as the
    // existing fixed-notional path does (`pv_leg_in_reporting_ccy`): a `Receive` leg's
    // initial sign is -1, which yields a negative-PV cashflow (the leg "pays out" notional
    // at start). The resetting-leg notional at start is `N_0^R = N_C / X_0`.
    if constant_leg.start > as_of {
        let df_c0 = robust_relative_df(disc_c.as_ref(), as_of, constant_leg.start)?;
        let df_r0 = robust_relative_df(disc_r.as_ref(), as_of, resetting_leg.start)?;

        let cf_c = constant_leg.side.initial_principal_sign() * n_c * df_c0;
        pv.add(convert(cf_c, constant_leg.currency, constant_leg.start)?);

        let n_r0 = n_c / spot_x;
        let cf_r = resetting_leg.side.initial_principal_sign() * n_r0 * df_r0;
        pv.add(convert(cf_r, resetting_leg.currency, resetting_leg.start)?);
    }

    // Per-period processing. Use `disc_c.day_count()` as the time axis for the CIP
    // forward FX formula; both curves share their own day-count under construction
    // semantics, and we use the constant-leg discount curve as canonical reference.
    let curve_base = disc_c.base_date();
    let curve_dc = disc_c.day_count();
    use finstack_core::dates::DayCountContext;

    let yf_from_base = |d: Date| -> Result<f64> {
        Ok(curve_dc.year_fraction(curve_base, d, DayCountContext::default())?)
    };

    let t_start = yf_from_base(constant_leg.start)?;
    let p_c_t_start = disc_c.df(t_start);
    let p_r_t_start = disc_r.df(t_start);
    if p_r_t_start <= 0.0 {
        return Err(finstack_core::Error::Validation(format!(
            "XccySwap '{}': non-positive resetting-leg discount factor at start",
            swap.id
        )));
    }
    // Forward FX at start (used as the reference X_0 for the resetting notional).
    let x_start = spot_x * (p_c_t_start / p_r_t_start);
    let n_r_initial = n_c / x_start;

    let mut n_r_prev = n_r_initial;
    for (j, period) in periods.iter().enumerate() {
        if period.payment_date <= as_of {
            n_r_prev = compute_resetting_notional(
                n_c,
                spot_x,
                yf_from_base(period.accrual_end)?,
                disc_c.as_ref(),
                disc_r.as_ref(),
            )?;
            continue;
        }

        let t_pay = yf_from_base(period.payment_date)?;
        let df_c_pay = disc_c.df(t_pay).max(0.0);
        let df_r_pay = disc_r.df(t_pay).max(0.0);

        // 1. Constant-leg floating coupon.
        let rate_c = rate_period_on_dates(
            fwd_c.as_ref(),
            period.reset_date.unwrap_or(period.accrual_start),
            period.accrual_end,
        )?;
        let coupon_c = constant_leg.side.coupon_sign()
            * n_c
            * rate_c
            * period.accrual_year_fraction
            * df_c_pay;
        pv.add(convert(coupon_c, constant_leg.currency, period.payment_date)?);

        // 2. Resetting-leg floating coupon (uses N_{j-1}^R — the notional captured at the
        //    start of the period). Includes the basis spread on the resetting leg.
        let rate_r = rate_period_on_dates(
            fwd_r.as_ref(),
            period.reset_date.unwrap_or(period.accrual_start),
            period.accrual_end,
        )?;
        let spread_decimal = decimal_to_f64(
            resetting_leg.spread_bp,
            "XccySwap resetting leg spread_bp",
        )? / 10_000.0;
        let coupon_r = resetting_leg.side.coupon_sign()
            * n_r_prev
            * (rate_r + spread_decimal)
            * period.accrual_year_fraction
            * df_r_pay;
        pv.add(convert(coupon_r, resetting_leg.currency, period.payment_date)?);

        // 3. Rebalancing at the START of this period (i.e. at T_j for period j+1).
        //    Skip the very first period — no rebalancing before initial exchange.
        //    The economic model: at each reset the resetting leg ends its old notional
        //    (final-style exchange of N_{j-1}^R) and starts a fresh one (initial-style
        //    exchange of N_j^R). Net cashflow uses `initial_principal_sign` on the
        //    delta = N_j - N_{j-1}, which gives the correct sign for both Pay/Receive.
        if j > 0 {
            let t_reset = yf_from_base(period.accrual_start)?;
            let df_c_reset = disc_c.df(t_reset).max(0.0);
            let df_r_reset = disc_r.df(t_reset).max(0.0);
            let p_r_reset = disc_r.df(t_reset);
            if p_r_reset <= 0.0 {
                return Err(finstack_core::Error::Validation(format!(
                    "XccySwap '{}': non-positive resetting-leg DF at reset date {}",
                    swap.id, period.accrual_start
                )));
            }
            let x_j = spot_x * (df_c_reset / p_r_reset);
            let n_r_j = n_c / x_j;
            let delta_n_r = n_r_j - n_r_prev;

            // Resetting leg: principal-style movement of size delta_n_r at T_j.
            let rebal_r = resetting_leg.side.initial_principal_sign() * delta_n_r * df_r_reset;
            pv.add(convert(
                rebal_r,
                resetting_leg.currency,
                period.accrual_start,
            )?);

            // Constant leg: corresponding FX-equivalent principal movement.
            let rebal_c =
                constant_leg.side.initial_principal_sign() * x_j * delta_n_r * df_c_reset;
            pv.add(convert(
                rebal_c,
                constant_leg.currency,
                period.accrual_start,
            )?);

            n_r_prev = n_r_j;
        }
    }

    // Final principal exchange: constant leg receives N_C; resetting leg pays N_n^R = n_r_prev.
    let t_end = yf_from_base(constant_leg.end)?;
    let df_c_end = disc_c.df(t_end).max(0.0);
    let df_r_end = disc_r.df(t_end).max(0.0);

    let cf_c_final = constant_leg.side.final_principal_sign() * n_c * df_c_end;
    pv.add(convert(cf_c_final, constant_leg.currency, constant_leg.end)?);

    let cf_r_final = resetting_leg.side.final_principal_sign() * n_r_prev * df_r_end;
    pv.add(convert(
        cf_r_final,
        resetting_leg.currency,
        resetting_leg.end,
    )?);

    Ok(Money::new(pv.total(), reporting_ccy))
}

/// CIP forward FX × N_C / X_0 == per-period resetting notional at curve time `t`.
fn compute_resetting_notional(
    n_constant: f64,
    spot_x: f64,
    t: f64,
    disc_c: &finstack_core::market_data::term_structures::DiscountCurve,
    disc_r: &finstack_core::market_data::term_structures::DiscountCurve,
) -> Result<f64> {
    let p_c = disc_c.df(t);
    let p_r = disc_r.df(t);
    if p_r <= 0.0 || p_c <= 0.0 {
        return Err(finstack_core::Error::Validation(format!(
            "MtM-reset: non-positive DF at curve-time {t:.6} (P_C={p_c}, P_R={p_r})"
        )));
    }
    let x_t = spot_x * (p_c / p_r);
    if x_t <= 0.0 {
        return Err(finstack_core::Error::Validation(format!(
            "MtM-reset: non-positive forward FX at curve-time {t:.6}"
        )));
    }
    Ok(n_constant / x_t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_resetting_notional_matches_formula() {
        use finstack_core::market_data::term_structures::DiscountCurve;
        use finstack_core::math::interp::{ExtrapolationPolicy, InterpStyle};
        use finstack_core::types::CurveId;
        use time::Month;

        let base = Date::from_calendar_date(2025, Month::January, 2).expect("date");
        // Flat 2% USD discount, flat 1% EUR discount, Act/365F.
        let disc_c = DiscountCurve::builder(CurveId::new("USD-OIS"))
            .base_date(base)
            .day_count(finstack_core::dates::DayCount::Act365F)
            .knots([(0.0, 1.0), (5.0, (-0.02_f64 * 5.0).exp())])
            .interp(InterpStyle::Linear)
            .extrapolation(ExtrapolationPolicy::FlatZero)
            .build()
            .expect("build USD curve");
        let disc_r = DiscountCurve::builder(CurveId::new("EUR-OIS"))
            .base_date(base)
            .day_count(finstack_core::dates::DayCount::Act365F)
            .knots([(0.0, 1.0), (5.0, (-0.01_f64 * 5.0).exp())])
            .interp(InterpStyle::Linear)
            .extrapolation(ExtrapolationPolicy::FlatZero)
            .build()
            .expect("build EUR curve");

        let spot = 1.10_f64; // USD per EUR
        let n_c = 10_000_000.0;
        let t = 2.5;
        let p_c = disc_c.df(t);
        let p_r = disc_r.df(t);
        let expected = n_c / (spot * p_c / p_r);

        let actual = compute_resetting_notional(n_c, spot, t, &disc_c, &disc_r)
            .expect("formula ok");
        assert!(
            (actual - expected).abs() < 1e-6,
            "got {actual}, expected {expected}"
        );
    }
}
