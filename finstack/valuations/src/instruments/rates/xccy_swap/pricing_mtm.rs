//! MtM-resetting cross-currency swap PV path.
//!
//! Implements the cashflow stream under the CIP no-FX-vol approximation. The constant leg
//! behaves like a vanilla fixed-notional XCCY leg (initial exchange, periodic coupons on
//! `N_C`, final exchange). The resetting leg's notional is re-marked at each accrual-period
//! start using `N_j^R = N_C / X_j^FRA` where `X_j^FRA = X_0 * P_C(T_j) / P_R(T_j)`, with
//! coupons accruing on the new notional and a rebalancing cashflow paid on the resetting
//! leg only to fund the notional change.
//!
//! The constant leg has **no** rebalancing cashflow — this matches standard MtM-XCCY market
//! convention (QuantLib's `MtMCrossCurrencyBasisSwap` is structured the same way). Under
//! CIP no-FX-vol, the constant-currency half of the FX swap that funds the rebalancing is
//! PV-fair from today's perspective, so emitting it explicitly would double-count.
//!
//! The whole PV reduces to a Neumaier-accumulated sum of reporting-currency-converted
//! discounted cashflows, requiring no additional FX surface beyond what
//! `pv_leg_in_reporting_ccy` already needs for fixed-notional XCCY.
//!
//! See `docs/superpowers/specs/2026-05-10-xccy-mtm-reset-design.md` for the spec.

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

    // FX rate (resetting -> constant) at the valuation date. The forward FX at any
    // curve-time T is derived as `spot_x_at_as_of * P_C(T) / P_R(T)` via CIP; the
    // spec's `X_0` is this value at the swap's start date, NOT necessarily spot.
    let spot_x_at_as_of = fx
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

    // Compute the per-period notional at T_start (the swap's start date). This is the
    // resetting-leg principal amount exchanged at initial exchange AND seeded into the
    // per-period loop. Distinct from `n_c / spot_x_at_as_of` for forward-starting swaps.
    let df_c_t_start = disc_c.df_on_date_curve(constant_leg.start)?;
    let df_c_t_start = require_positive_df(df_c_t_start, &swap.id, "constant-leg", constant_leg.start)?;
    let df_r_t_start = disc_r.df_on_date_curve(constant_leg.start)?;
    let df_r_t_start = require_positive_df(df_r_t_start, &swap.id, "resetting-leg", constant_leg.start)?;
    let x_start = spot_x_at_as_of * (df_c_t_start / df_r_t_start);
    if !x_start.is_finite() || x_start <= 0.0 {
        return Err(finstack_core::Error::Validation(format!(
            "XccySwap '{}': non-positive forward FX at swap start date {}",
            swap.id, constant_leg.start
        )));
    }
    let n_r_initial = n_c / x_start;

    // Initial principal exchange at start. We use `initial_principal_sign` exactly as the
    // existing fixed-notional path does (`pv_leg_in_reporting_ccy`): a `Receive` leg's
    // initial sign is -1, which yields a negative-PV cashflow (the leg "pays out" notional
    // at start). The resetting-leg notional at start is `N_0^R = N_C / X_0`.
    if constant_leg.start > as_of {
        let df_c0 = robust_relative_df(disc_c.as_ref(), as_of, constant_leg.start)?;
        let df_r0 = robust_relative_df(disc_r.as_ref(), as_of, resetting_leg.start)?;

        let cf_c = constant_leg.side.initial_principal_sign() * n_c * df_c0;
        pv.add(convert(cf_c, constant_leg.currency, constant_leg.start)?);

        let cf_r = resetting_leg.side.initial_principal_sign() * n_r_initial * df_r0;
        pv.add(convert(cf_r, resetting_leg.currency, resetting_leg.start)?);
    }

    // Per-period loop. For each accrual period [T_j, T_{j+1}]:
    //   - Constant leg accrues a coupon on its fixed notional `N_C`.
    //   - Resetting leg accrues a coupon on `N_j^R = N_C / X_j^FRA`, the notional captured at
    //     the START of the period (i.e. at accrual_start = T_j).
    //   - At each interior reset T_j (j = 1..n-1), the resetting leg emits a rebalancing
    //     cashflow of `(N_j^R - N_{j-1}^R)` in its own currency. There is NO corresponding
    //     constant-leg rebalancing: under CIP-no-vol the FX swap that funds the notional
    //     change is PV-fair from today's perspective, and the constant leg's net
    //     contribution is implicit in its unchanged principal-and-coupon schedule.
    //     (Cross-check: QuantLib's MtM-XCCY example
    //     https://www.implementingquantlib.com/2023/09/cross-currency-swaps.html
    //     emits rebalancing only on the resetting leg.)
    let mut n_r_prev = n_r_initial;
    for (j, period) in periods.iter().enumerate() {
        // Notional captured at the start of THIS period (T_j) = N_j^R.
        let n_r_j = if j == 0 {
            n_r_initial
        } else {
            compute_resetting_notional(
                n_c,
                spot_x_at_as_of,
                period.accrual_start,
                disc_c.as_ref(),
                disc_r.as_ref(),
                &swap.id,
            )?
        };

        if period.payment_date <= as_of {
            n_r_prev = n_r_j;
            continue;
        }

        let df_c_pay = disc_c.df_on_date_curve(period.payment_date)?;
        let df_c_pay = require_positive_df(df_c_pay, &swap.id, "constant-leg", period.payment_date)?;
        let df_r_pay = disc_r.df_on_date_curve(period.payment_date)?;
        let df_r_pay = require_positive_df(df_r_pay, &swap.id, "resetting-leg", period.payment_date)?;

        // 1. Constant-leg floating coupon (notional N_C).
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

        // 2. Resetting-leg floating coupon on N_j^R (notional captured at this period's start,
        //    NOT n_r_prev which is the prior period's notional). Includes the basis spread.
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
            * n_r_j
            * (rate_r + spread_decimal)
            * period.accrual_year_fraction
            * df_r_pay;
        pv.add(convert(coupon_r, resetting_leg.currency, period.payment_date)?);

        // 3. Rebalancing on the resetting leg only, at the START of this period (T_j).
        //    Skip the very first period — no rebalancing before initial exchange.
        //    The resetting leg ends its old notional (`N_{j-1}^R`) and starts a fresh one
        //    (`N_j^R`). Net cashflow uses `initial_principal_sign` on the delta, which gives
        //    the correct sign for both Pay/Receive resetting sides. The constant leg has no
        //    corresponding rebalancing cashflow (see comment above).
        if j > 0 {
            let df_r_reset = disc_r.df_on_date_curve(period.accrual_start)?;
            let df_r_reset = require_positive_df(df_r_reset, &swap.id, "resetting-leg", period.accrual_start)?;
            let delta_n_r = n_r_j - n_r_prev;
            let rebal_r = resetting_leg.side.initial_principal_sign() * delta_n_r * df_r_reset;
            pv.add(convert(rebal_r, resetting_leg.currency, period.accrual_start)?);
        }

        n_r_prev = n_r_j;
    }

    // Final principal exchange: constant leg receives N_C; resetting leg pays N_n^R = n_r_prev.
    let df_c_end = disc_c.df_on_date_curve(constant_leg.end)?;
    let df_c_end = require_positive_df(df_c_end, &swap.id, "constant-leg", constant_leg.end)?;
    let df_r_end = disc_r.df_on_date_curve(resetting_leg.end)?;
    let df_r_end = require_positive_df(df_r_end, &swap.id, "resetting-leg", resetting_leg.end)?;

    let cf_c_final = constant_leg.side.final_principal_sign() * n_c * df_c_end;
    pv.add(convert(cf_c_final, constant_leg.currency, constant_leg.end)?);

    let cf_r_final = resetting_leg.side.final_principal_sign() * n_r_prev * df_r_end;
    pv.add(convert(cf_r_final, resetting_leg.currency, resetting_leg.end)?);

    Ok(Money::new(pv.total(), reporting_ccy))
}

/// CIP forward FX × N_C / X_0 == per-period resetting notional at `date`.
///
/// Each curve uses its own day-count and base date via `df_on_date_curve`.
fn compute_resetting_notional(
    n_constant: f64,
    spot_x_at_as_of: f64,
    date: Date,
    disc_c: &finstack_core::market_data::term_structures::DiscountCurve,
    disc_r: &finstack_core::market_data::term_structures::DiscountCurve,
    swap_id: &finstack_core::types::InstrumentId,
) -> Result<f64> {
    let p_c = disc_c.df_on_date_curve(date)?;
    let p_c = require_positive_df(p_c, swap_id, "constant-leg", date)?;
    let p_r = disc_r.df_on_date_curve(date)?;
    let p_r = require_positive_df(p_r, swap_id, "resetting-leg", date)?;
    let x_t = spot_x_at_as_of * (p_c / p_r);
    if !x_t.is_finite() || x_t <= 0.0 {
        return Err(finstack_core::Error::Validation(format!(
            "XccySwap '{swap_id}': non-positive forward FX at date {date}"
        )));
    }
    Ok(n_constant / x_t)
}

/// Guard that a discount factor is finite and strictly positive; returns the value on success.
fn require_positive_df(
    df: f64,
    swap_id: &finstack_core::types::InstrumentId,
    curve_role: &str,
    date: Date,
) -> Result<f64> {
    if !df.is_finite() || df <= 0.0 {
        return Err(finstack_core::Error::Validation(format!(
            "XccySwap '{swap_id}': non-positive/non-finite {curve_role} DF at {date}: {df}"
        )));
    }
    Ok(df)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_resetting_notional_matches_formula() {
        use finstack_core::market_data::term_structures::DiscountCurve;
        use finstack_core::math::interp::{ExtrapolationPolicy, InterpStyle};
        use finstack_core::types::{CurveId, InstrumentId};
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
        // Use a date 2.5 years from base (approximately, using Act/365F).
        // 2.5 * 365.25 ≈ 913 days — round to 912 for even arithmetic.
        let date = Date::from_calendar_date(2027, Month::July, 2).expect("date");
        let swap_id = InstrumentId::new("TEST-XCCY-SWAP");

        // Reference values computed via df_on_date_curve (each curve uses its own axis).
        let p_c = disc_c.df_on_date_curve(date).expect("p_c");
        let p_r = disc_r.df_on_date_curve(date).expect("p_r");
        let expected = n_c / (spot * p_c / p_r);

        let actual = compute_resetting_notional(n_c, spot, date, &disc_c, &disc_r, &swap_id)
            .expect("formula ok");
        assert!(
            (actual - expected).abs() < 1e-6,
            "got {actual}, expected {expected}"
        );
    }
}
