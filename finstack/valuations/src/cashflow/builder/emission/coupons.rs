//! Coupon cashflow emission (fixed and floating).

use crate::cashflow::primitives::{CFKind, CashFlow};
use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DateExt, Tenor};
use finstack_core::market_data::term_structures::ForwardCurve;
use finstack_core::money::Money;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;

use super::super::compiler::{FixedSchedule, FloatSchedule};
use super::helpers::{add_pik_flow_if_nonzero, compute_reset_date, resolve_calendar};

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

/// Emit fixed coupon cashflows on a specific date.
///
/// Processes all fixed coupon schedules for the given date, computing coupon
/// amounts based on outstanding balances and splitting into cash/PIK according
/// to the coupon type.
///
/// Returns `pik_to_add`, the total PIK amount to capitalize into the
/// outstanding balance. Cash and PIK flows are appended directly into
/// the provided `out_flows` buffer to avoid per-date allocations.
pub(in crate::cashflow::builder) fn emit_fixed_coupons_on(
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

        if let Some(prev) = prev_map.get(&d).copied() {
            let base_out = *outstanding_after
                .get(&prev)
                .unwrap_or(&outstanding_fallback);

            // Resolve calendar if present for Bus/252 and similar conventions
            let calendar = resolve_calendar(spec.calendar_id.as_deref());
            let yf = spec.dc.year_fraction(
                prev,
                d,
                finstack_core::dates::DayCountCtx {
                    calendar,
                    frequency: Some(spec.freq),
                    bus_basis: None,
                },
            )?;

            // Convert Decimal rate to f64 for year fraction computation, then use Decimal for coupon calc
            let base_out_dec = Decimal::try_from(base_out).unwrap_or(Decimal::ZERO);
            let yf_dec = Decimal::try_from(yf).unwrap_or(Decimal::ZERO);
            let coupon_total_dec = base_out_dec * spec.rate * yf_dec;
            let coupon_total = coupon_total_dec.to_f64().unwrap_or(0.0);

            let (cash_pct, pik_pct) = spec.coupon_type.split_parts()?;
            let cash_pct_f64 = cash_pct.to_f64().unwrap_or(0.0);
            let pik_pct_f64 = pik_pct.to_f64().unwrap_or(0.0);

            let cash_amt = coupon_total * cash_pct_f64;
            let pik_amt = coupon_total * pik_pct_f64;

            // Convert rate to f64 for CashFlow storage
            let rate_f64 = spec.rate.to_f64().unwrap_or(0.0);

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

            pik_to_add += add_pik_flow_if_nonzero(out_flows, d, pik_amt, ccy);
        }
    }
    Ok(pik_to_add)
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
pub(in crate::cashflow::builder) fn emit_float_coupons_on(
    d: Date,
    float_schedules: &[FloatSchedule],
    outstanding_after: &finstack_core::HashMap<Date, f64>,
    outstanding_fallback: f64,
    ccy: Currency,
    resolved_curves: &[Option<&ForwardCurve>],
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

        if let Some(prev) = prev_map.get(&d).copied() {
            let base_out = *outstanding_after
                .get(&prev)
                .unwrap_or(&outstanding_fallback);

            // Resolve calendar if present for Bus/252 and similar conventions
            let calendar = resolve_calendar(spec.rate_spec.calendar_id.as_deref());
            let yf = spec.rate_spec.dc.year_fraction(
                prev,
                d,
                finstack_core::dates::DayCountCtx {
                    calendar,
                    frequency: Some(spec.rate_spec.reset_freq),
                    bus_basis: None,
                },
            )?;

            // Resolve fixing calendar for reset date (default to accrual calendar if None)
            let fixing_cal_id = spec
                .rate_spec
                .fixing_calendar_id
                .clone()
                .or_else(|| spec.rate_spec.calendar_id.clone());

            // Compute reset date (fixing date) from accrual start.
            let reset_date = compute_reset_date(
                prev,
                spec.rate_spec.reset_lag_days,
                spec.rate_spec.bdc,
                &fixing_cal_id,
            )?;

            // Compute index maturity based on the index tenor.
            // This ensures the forward rate is projected for the correct period
            // (e.g., 3M LIBOR projects from reset_date to reset_date + 3M),
            // regardless of when the payment actually occurs.
            let index_maturity = compute_index_maturity(reset_date, spec.rate_spec.reset_freq);

            // Construct params for detailed projection (converting Decimal to f64 for rate_helpers)
            let params = crate::cashflow::builder::rate_helpers::FloatingRateParams {
                spread_bp: spec.rate_spec.spread_bp.to_f64().unwrap_or(0.0),
                gearing: spec.rate_spec.gearing.to_f64().unwrap_or(1.0),
                gearing_includes_spread: spec.rate_spec.gearing_includes_spread,
                index_floor_bp: spec.rate_spec.floor_bp.and_then(|d| d.to_f64()),
                index_cap_bp: spec.rate_spec.index_cap_bp.and_then(|d| d.to_f64()),
                all_in_floor_bp: spec.rate_spec.all_in_floor_bp.and_then(|d| d.to_f64()),
                all_in_cap_bp: spec.rate_spec.cap_bp.and_then(|d| d.to_f64()),
            };

            // Compute total rate using centralized projection with floor/cap support
            let total_rate = if let Some(fwd) = *resolved_curve {
                // Use floating rate projection with correct index maturity
                match super::super::rate_helpers::project_floating_rate(
                    reset_date,
                    index_maturity, // Use index tenor end, not payment date
                    fwd,
                    &params,
                ) {
                    Ok(rate) => rate,
                    Err(_) => super::super::rate_helpers::project_fallback_rate(&params),
                }
            } else {
                super::super::rate_helpers::project_fallback_rate(&params)
            };

            // Use Decimal for coupon calculation
            let base_out_dec = Decimal::try_from(base_out).unwrap_or(Decimal::ZERO);
            let total_rate_dec = Decimal::try_from(total_rate).unwrap_or(Decimal::ZERO);
            let yf_dec = Decimal::try_from(yf).unwrap_or(Decimal::ZERO);
            let coupon_total_dec = base_out_dec * total_rate_dec * yf_dec;
            let coupon_total = coupon_total_dec.to_f64().unwrap_or(0.0);

            let (cash_pct, pik_pct) = spec.coupon_type.split_parts()?;
            let cash_pct_f64 = cash_pct.to_f64().unwrap_or(0.0);
            let pik_pct_f64 = pik_pct.to_f64().unwrap_or(0.0);
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

            pik_to_add += add_pik_flow_if_nonzero(out_flows, d, pik_amt, ccy);
        }
    }
    Ok(pik_to_add)
}
