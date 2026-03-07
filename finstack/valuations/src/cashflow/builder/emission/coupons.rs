//! Coupon cashflow emission (fixed and floating).
//!
//! # Future Extensions
//!
//! TODO: Add explicit inflation-linked coupon emission logic. This would support:
//! - CPI-linked coupons with interpolation (e.g., 2-month or 3-month lag)
//! - Index ratio calculations for principal adjustment
//! - Real vs nominal rate decomposition
//! - Support for different inflation indices (CPI-U, HICP, RPI, etc.)

use crate::cashflow::primitives::{CFKind, CashFlow};
use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DateExt, Tenor};
use finstack_core::market_data::term_structures::ForwardCurve;
use finstack_core::money::Money;
use finstack_core::InputError;
use rust_decimal::prelude::ToPrimitive;
use tracing::{info, warn};

use super::super::compiler::{FixedSchedule, FloatSchedule};
use super::helpers::{add_pik_flow_if_nonzero, compute_reset_date};
use crate::cashflow::builder::calendar::resolve_calendar_strict;

/// Emit inflation-linked coupon cashflows.
///
/// Each tuple is `(payment_date, indexed_coupon_amount, accrual_factor, real_coupon_rate)`.
pub(crate) fn emit_inflation_coupons(
    ccy: Currency,
    coupons: &[(Date, f64, f64, f64)],
    out_flows: &mut Vec<CashFlow>,
) {
    for &(date, amount, accrual_factor, real_coupon_rate) in coupons {
        out_flows.push(CashFlow {
            date,
            reset_date: None,
            amount: Money::new(amount, ccy),
            kind: CFKind::InflationCoupon,
            accrual_factor,
            rate: Some(real_coupon_rate),
        });
    }
}

// Shared f64 ↔ Decimal conversion helpers live in the parent `emission` module
// so that `fees.rs` can use them too. Access via `super::`.
use super::{decimal_to_f64, f64_to_decimal};

/// Compute the index maturity date based on reset date and index tenor.
///
/// For a floating rate index (e.g., 3M LIBOR), the forward rate should be projected
/// from the reset (fixing) date to the index maturity date, not the payment date.
/// This ensures correct rate projection for instruments where the payment date
/// differs from the index tenor end.
fn compute_index_maturity(reset_date: Date, index_tenor: Tenor) -> Date {
    use finstack_core::dates::TenorUnit;
    match index_tenor.unit {
        TenorUnit::Months => reset_date.add_months(index_tenor.count as i32),
        TenorUnit::Days => reset_date + time::Duration::days(index_tenor.count as i64),
        TenorUnit::Years => reset_date.add_months((index_tenor.count * 12) as i32),
        TenorUnit::Weeks => reset_date + time::Duration::days((index_tenor.count * 7) as i64),
    }
}

fn rate_when_curve_missing(
    index_id: &str,
    reset_date: Date,
    spread_bp: f64,
    fallback: &crate::cashflow::builder::specs::FloatingRateFallback,
    params: &crate::cashflow::builder::rate_helpers::FloatingRateParams,
    context_suffix: &str,
) -> finstack_core::Result<f64> {
    use crate::cashflow::builder::specs::FloatingRateFallback;

    match fallback {
        FloatingRateFallback::Error => Err(finstack_core::Error::Input(InputError::NotFound {
            id: format!(
                "forward curve '{}' not found for reset date {}{}",
                index_id, reset_date, context_suffix
            ),
        })),
        FloatingRateFallback::SpreadOnly => {
            warn!(
                reset_date = %reset_date,
                spread_bp = %spread_bp,
                "No forward curve resolved{context_suffix}, using fallback (spread-only) rate"
            );
            Ok(super::super::rate_helpers::project_fallback_rate(params))
        }
        FloatingRateFallback::FixedRate(r) => {
            let index_rate = r.to_f64().unwrap_or(0.0);
            info!(
                reset_date = %reset_date,
                fixed_rate = %r,
                "No forward curve resolved{context_suffix}, using fixed index rate"
            );
            Ok(super::super::rate_helpers::calculate_floating_rate(
                index_rate, params,
            ))
        }
    }
}

fn rate_when_projection_fails(
    error: &finstack_core::Error,
    reset_date: Date,
    index_maturity: Date,
    spread_bp: f64,
    fallback: &crate::cashflow::builder::specs::FloatingRateFallback,
    params: &crate::cashflow::builder::rate_helpers::FloatingRateParams,
) -> finstack_core::Result<f64> {
    use crate::cashflow::builder::specs::FloatingRateFallback;

    match fallback {
        FloatingRateFallback::Error => Err(error.clone()),
        FloatingRateFallback::SpreadOnly => {
            warn!(
                reset_date = %reset_date,
                index_maturity = %index_maturity,
                spread_bp = %spread_bp,
                error = %error,
                "Floating rate projection failed, using fallback (spread-only) rate"
            );
            Ok(super::super::rate_helpers::project_fallback_rate(params))
        }
        FloatingRateFallback::FixedRate(r) => {
            let index_rate = r.to_f64().unwrap_or(0.0);
            info!(
                reset_date = %reset_date,
                fixed_rate = %r,
                error = %error,
                "Floating rate projection failed, using fixed index rate"
            );
            Ok(super::super::rate_helpers::calculate_floating_rate(
                index_rate, params,
            ))
        }
    }
}

/// Emit fixed coupon cashflows on a specific date.
///
/// Processes all fixed coupon schedules for the given date, computing coupon
/// amounts based on outstanding balances and splitting into cash/PIK according
/// to the coupon type.
///
/// Returns `pik_to_add`, the total PIK amount to capitalize into the
/// outstanding balance. Cash and PIK flows are appended directly into
/// the provided `out_flows` buffer to avoid per-date allocations.
pub(crate) fn emit_fixed_coupons_on(
    d: Date,
    fixed_schedules: &[FixedSchedule],
    outstanding_after: &finstack_core::HashMap<Date, f64>,
    outstanding_fallback: f64,
    ccy: Currency,
    out_flows: &mut Vec<CashFlow>,
) -> finstack_core::Result<f64> {
    let mut pik_to_add = 0.0;

    for (spec, dates, prev_map, first_last) in fixed_schedules {
        // Early exit: skip schedules where `d` is outside the date range.
        // This reduces iteration from O(N × M) to O(N + M) for multi-window instruments.
        if let (Some(&first), Some(&last)) = (dates.first(), dates.last()) {
            if d < first || d > last {
                continue;
            }
        }

        if let Some(period) = prev_map.get(&d).copied() {
            let accrual_start = period.accrual_start;
            let accrual_end = period.accrual_end;
            let base_out = *outstanding_after
                .get(&accrual_start)
                .unwrap_or(&outstanding_fallback);

            // Resolve calendar for Bus/252 and similar conventions
            let calendar = resolve_calendar_strict(&spec.calendar_id)?;
            let yf = spec.dc.year_fraction(
                accrual_start,
                accrual_end,
                finstack_core::dates::DayCountCtx {
                    calendar: Some(calendar),
                    frequency: Some(spec.freq),
                    bus_basis: None,
                },
            )?;

            // Convert f64 values to Decimal with proper error handling for NaN/Infinity.
            // This prevents silent masking of invalid values as zero.
            let base_out_dec = f64_to_decimal(base_out, "outstanding balance")?;
            let yf_dec = f64_to_decimal(yf, "year fraction")?;
            let coupon_total_dec = base_out_dec * spec.rate * yf_dec;
            let coupon_total = decimal_to_f64(coupon_total_dec, "coupon total")?;

            let (cash_pct, pik_pct) = spec.coupon_type.split_parts()?;
            let cash_pct_f64 = decimal_to_f64(cash_pct, "cash percentage")?;
            let pik_pct_f64 = decimal_to_f64(pik_pct, "pik percentage")?;

            let cash_amt = coupon_total * cash_pct_f64;
            let pik_amt = coupon_total * pik_pct_f64;

            // Convert rate to f64 for CashFlow storage
            let rate_f64 = decimal_to_f64(spec.rate, "coupon rate")?;

            if cash_amt > 0.0 {
                let kind = if first_last.contains(&d) {
                    CFKind::Stub
                } else {
                    CFKind::Fixed
                };
                out_flows.push(CashFlow {
                    date: d,
                    reset_date: None,
                    amount: Money::new(cash_amt, ccy),
                    kind,
                    accrual_factor: yf,
                    rate: Some(rate_f64),
                });
            }

            pik_to_add += add_pik_flow_if_nonzero(out_flows, d, pik_amt, ccy, Some(rate_f64), yf);
        }
    }
    Ok(pik_to_add)
}

/// Sample daily overnight rates from a forward curve for an accrual period.
///
/// For each calendar day in `[accrual_start, accrual_end)`, assigns the overnight
/// rate fixing at the nearest preceding business day. Non-business days before the
/// first fixing accumulate into the first business day's weight; non-business days
/// after a fixing accumulate into the preceding fixing's weight.
///
/// Returns `(daily_rates, total_days)` where:
/// - `daily_rates` is a vec of `(rate, weight_days)` per fixing date.
/// - `total_days` is the total calendar days in the period (used as the denominator
///   for simple-average compounding methods).
///
/// # ISDA 2021 Reference
///
/// Per Section 7.1(g): the rate for each Reset Date accrues for the number of
/// calendar days from that Reset Date to the next Reset Date (or period end).
fn sample_overnight_rates(
    accrual_start: Date,
    accrual_end: Date,
    fwd: &ForwardCurve,
    calendar: &dyn finstack_core::dates::HolidayCalendar,
) -> (Vec<(f64, u32)>, u32) {
    let fwd_dc = fwd.day_count();
    let fwd_base = fwd.base_date();

    let mut daily_rates: Vec<(f64, u32)> = Vec::new();
    let mut pre_first_fixing_days: u32 = 0;
    let mut current = accrual_start;

    while current < accrual_end {
        let next = current + time::Duration::days(1);
        let next_capped = if next > accrual_end {
            accrual_end
        } else {
            next
        };
        let days = (next_capped - current).whole_days().max(1) as u32;

        if current.is_business_day(calendar) {
            let t = if current <= fwd_base {
                0.0
            } else {
                fwd_dc
                    .year_fraction(
                        fwd_base,
                        current,
                        finstack_core::dates::DayCountCtx::default(),
                    )
                    .unwrap_or(0.0)
            };
            let rate = fwd.rate(t);
            // Assign any pre-period non-business days to this first fixing.
            let total = days + pre_first_fixing_days;
            pre_first_fixing_days = 0;
            daily_rates.push((rate, total));
        } else if daily_rates.is_empty() {
            // Non-business day before the first fixing: accumulate to assign
            // to the first fixing's weight once we encounter it.
            pre_first_fixing_days += days;
        } else if let Some(last) = daily_rates.last_mut() {
            // Non-business day after a fixing: add to the preceding fixing.
            last.1 += days;
        }
        current = next_capped;
    }

    let total_days = (accrual_end - accrual_start).whole_days().max(1) as u32;
    (daily_rates, total_days)
}

/// Emit floating coupon cashflows on a specific date.
///
/// Processes all floating coupon schedules for the given date, looking up forward
/// rates from the optional market context and computing coupon amounts based on
/// `forward_rate * gearing + margin`. Splits into cash/PIK according to coupon type.
///
/// Returns `pik_to_add`, the total PIK amount to capitalize into the
/// outstanding balance. Cash and PIK flows are appended directly into
/// the provided `out_flows` buffer.
pub(crate) fn emit_float_coupons_on(
    d: Date,
    float_schedules: &[FloatSchedule],
    outstanding_after: &finstack_core::HashMap<Date, f64>,
    outstanding_fallback: f64,
    ccy: Currency,
    resolved_curves: &[Option<std::sync::Arc<ForwardCurve>>],
    out_flows: &mut Vec<CashFlow>,
) -> finstack_core::Result<f64> {
    let mut pik_to_add = 0.0;

    for ((spec, dates, prev_map), resolved_curve) in
        float_schedules.iter().zip(resolved_curves.iter())
    {
        // Early exit: skip schedules where `d` is outside the date range.
        // This reduces iteration from O(N × M) to O(N + M) for multi-window instruments.
        if let (Some(&first), Some(&last)) = (dates.first(), dates.last()) {
            if d < first || d > last {
                continue;
            }
        }

        if let Some(period) = prev_map.get(&d).copied() {
            let accrual_start = period.accrual_start;
            let accrual_end = period.accrual_end;
            let base_out = *outstanding_after
                .get(&accrual_start)
                .unwrap_or(&outstanding_fallback);

            // Resolve calendar for Bus/252 and similar conventions
            let calendar = resolve_calendar_strict(&spec.rate_spec.calendar_id)?;
            let yf = spec.rate_spec.dc.year_fraction(
                accrual_start,
                accrual_end,
                finstack_core::dates::DayCountCtx {
                    calendar: Some(calendar),
                    frequency: Some(spec.rate_spec.reset_freq),
                    bus_basis: None,
                },
            )?;

            // Resolve fixing calendar for reset date (default to accrual calendar if None)
            let fixing_cal_id = spec
                .rate_spec
                .fixing_calendar_id
                .as_deref()
                .unwrap_or(&spec.rate_spec.calendar_id);

            // Compute reset date (fixing date) from accrual start.
            let reset_date = compute_reset_date(
                accrual_start,
                spec.rate_spec.reset_lag_days,
                spec.rate_spec.bdc,
                fixing_cal_id,
            )?;

            // Compute index maturity based on the index tenor.
            // This ensures the forward rate is projected for the correct period
            // (e.g., 3M LIBOR projects from reset_date to reset_date + 3M),
            // regardless of when the payment actually occurs.
            let index_maturity = compute_index_maturity(reset_date, spec.rate_spec.reset_freq);

            // Construct params for detailed projection (converting Decimal to f64 for rate_helpers).
            // Use proper error handling for Decimal->f64 conversion.
            let spread_bp = decimal_to_f64(spec.rate_spec.spread_bp, "spread_bp")?;
            let gearing = decimal_to_f64(spec.rate_spec.gearing, "gearing")?;
            let params = crate::cashflow::builder::rate_helpers::FloatingRateParams {
                spread_bp,
                gearing,
                gearing_includes_spread: spec.rate_spec.gearing_includes_spread,
                index_floor_bp: spec.rate_spec.floor_bp.and_then(|d| {
                    let v = d.to_f64();
                    if v.is_none() {
                        warn!(value = %d, "floor_bp Decimal-to-f64 conversion failed; floor will be ignored");
                    }
                    v
                }),
                index_cap_bp: spec.rate_spec.index_cap_bp.and_then(|d| {
                    let v = d.to_f64();
                    if v.is_none() {
                        warn!(value = %d, "index_cap_bp Decimal-to-f64 conversion failed; cap will be ignored");
                    }
                    v
                }),
                all_in_floor_bp: spec.rate_spec.all_in_floor_bp.and_then(|d| {
                    let v = d.to_f64();
                    if v.is_none() {
                        warn!(value = %d, "all_in_floor_bp Decimal-to-f64 conversion failed; floor will be ignored");
                    }
                    v
                }),
                all_in_cap_bp: spec.rate_spec.cap_bp.and_then(|d| {
                    let v = d.to_f64();
                    if v.is_none() {
                        warn!(value = %d, "cap_bp Decimal-to-f64 conversion failed; cap will be ignored");
                    }
                    v
                }),
            };

            // Compute total rate using centralized projection with floor/cap support.
            // When projection fails (curve error or missing curve), the fallback
            // policy on the spec controls behavior:
            //   Error      -> propagate immediately (strictest, default)
            //   SpreadOnly -> use spread as total rate (legacy)
            //   FixedRate(r) -> use r as the index component
            let total_rate = if let Some(ref method) = spec.rate_spec.overnight_compounding {
                // ── Overnight compounding path ──
                // Sample daily rates from the forward curve and compound them
                // according to the ISDA 2021 method, then apply floor/cap/gearing/spread.
                if let Some(fwd) = resolved_curve.as_deref() {
                    // Sample per-business-day overnight rates and their calendar-day weights
                    // from the forward curve. Extracted to `sample_overnight_rates` for
                    // independent testability and separation of rate-sampling from compounding.
                    let (daily_rates, total_days) =
                        sample_overnight_rates(accrual_start, accrual_end, fwd, calendar);

                    // Determine the annualization basis from the floating rate day count.
                    let day_count_basis = match spec.rate_spec.dc {
                        finstack_core::dates::DayCount::Act365F
                        | finstack_core::dates::DayCount::Act365L => 365.0,
                        _ => 360.0, // Act360, Thirty360, etc.
                    };

                    let compounded_index = super::super::rate_helpers::compute_overnight_rate(
                        *method,
                        &daily_rates,
                        total_days,
                        day_count_basis,
                    );

                    // Apply floor/cap/gearing/spread to the compounded index rate.
                    super::super::rate_helpers::calculate_floating_rate(compounded_index, &params)
                } else {
                    rate_when_curve_missing(
                        spec.rate_spec.index_id.as_str(),
                        reset_date,
                        spread_bp,
                        &spec.rate_spec.fallback,
                        &params,
                        " (overnight compounding)",
                    )?
                }
            } else if let Some(fwd) = resolved_curve.as_deref() {
                // ── Standard term rate projection path ──
                // Use floating rate projection with correct index maturity
                match super::super::rate_helpers::project_floating_rate(
                    reset_date,
                    index_maturity, // Use index tenor end, not payment date
                    fwd,
                    &params,
                ) {
                    Ok(rate) => rate,
                    Err(error) => rate_when_projection_fails(
                        &error,
                        reset_date,
                        index_maturity,
                        spread_bp,
                        &spec.rate_spec.fallback,
                        &params,
                    )?,
                }
            } else {
                rate_when_curve_missing(
                    spec.rate_spec.index_id.as_str(),
                    reset_date,
                    spread_bp,
                    &spec.rate_spec.fallback,
                    &params,
                    "",
                )?
            };

            // Convert f64 values to Decimal with proper error handling for NaN/Infinity.
            // This prevents silent masking of invalid values as zero.
            let base_out_dec = f64_to_decimal(base_out, "outstanding balance")?;
            let total_rate_dec = f64_to_decimal(total_rate, "total rate")?;
            let yf_dec = f64_to_decimal(yf, "year fraction")?;
            let coupon_total_dec = base_out_dec * total_rate_dec * yf_dec;
            let coupon_total = decimal_to_f64(coupon_total_dec, "coupon total")?;

            let (cash_pct, pik_pct) = spec.coupon_type.split_parts()?;
            let cash_pct_f64 = decimal_to_f64(cash_pct, "cash percentage")?;
            let pik_pct_f64 = decimal_to_f64(pik_pct, "pik percentage")?;
            let cash_amt = coupon_total * cash_pct_f64;
            let pik_amt = coupon_total * pik_pct_f64;

            // Emit cash portion of floating coupon if any.
            // Note: PIK portion is emitted separately via add_pik_flow_if_nonzero.
            // For 100% PIK coupons, only the PIK flow is emitted, which is intentional
            // since the schedule structure (dates, accrual factors) is preserved in PIK flows.
            if cash_pct_f64 > 0.0 {
                out_flows.push(CashFlow {
                    date: d,
                    reset_date: Some(reset_date),
                    amount: Money::new(cash_amt, ccy),
                    kind: CFKind::FloatReset,
                    accrual_factor: yf,
                    rate: Some(total_rate),
                });
            }

            pik_to_add += add_pik_flow_if_nonzero(out_flows, d, pik_amt, ccy, Some(total_rate), yf);
        }
    }
    Ok(pik_to_add)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use time::Month;

    #[test]
    fn emit_inflation_coupons_preserves_non_positive_amounts() {
        let mut flows = Vec::new();
        emit_inflation_coupons(
            Currency::USD,
            &[
                (
                    Date::from_calendar_date(2025, Month::January, 1).expect("valid date"),
                    0.0,
                    0.5,
                    0.02,
                ),
                (
                    Date::from_calendar_date(2025, Month::July, 1).expect("valid date"),
                    -12.5,
                    0.5,
                    0.02,
                ),
            ],
            &mut flows,
        );

        assert_eq!(flows.len(), 2);
        assert_eq!(flows[0].kind, CFKind::InflationCoupon);
        assert_eq!(flows[1].amount.amount(), -12.5);
    }
}
