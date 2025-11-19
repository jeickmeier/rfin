//! Coupon cashflow emission (fixed and floating).

use crate::cashflow::primitives::{CFKind, CashFlow};
use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::term_structures::ForwardCurve;
use finstack_core::money::Money;
use std::sync::Arc;

use super::super::compiler::{FixedSchedule, FloatSchedule};
use super::helpers::{add_pik_flow_if_nonzero, compute_reset_date};

/// Emit fixed coupon cashflows on a specific date.
///
/// Processes all fixed coupon schedules for the given date, computing coupon
/// amounts based on outstanding balances and splitting into cash/PIK according
/// to the coupon type.
///
/// Returns `(pik_to_add, flows)` where `pik_to_add` is the total PIK amount
/// to capitalize into the outstanding balance.
pub(in crate::cashflow::builder) fn emit_fixed_coupons_on(
    d: Date,
    fixed_schedules: &[FixedSchedule],
    outstanding_after: &hashbrown::HashMap<Date, f64>,
    outstanding_fallback: f64,
    ccy: Currency,
) -> finstack_core::Result<(f64, Vec<CashFlow>)> {
    let mut pik_to_add = 0.0;
    let mut new_flows: Vec<CashFlow> = Vec::new();

    for (spec, _dates, prev_map, first_last) in fixed_schedules {
        if let Some(prev) = prev_map.get(&d).copied() {
            let base_out = *outstanding_after
                .get(&prev)
                .unwrap_or(&outstanding_fallback);

            // Resolve calendar if present for Bus/252 and similar conventions
            let calendar = spec
                .calendar_id
                .as_deref()
                .and_then(finstack_core::dates::calendar::calendar_by_id);

            let yf = spec.dc.year_fraction(
                prev,
                d,
                finstack_core::dates::DayCountCtx {
                    calendar,
                    frequency: Some(spec.freq),
                    bus_basis: None,
                },
            )?;
            let coupon_total = base_out * (spec.rate * yf);
            let (cash_pct, pik_pct) = spec.coupon_type.split_parts()?;

            let cash_amt = coupon_total * cash_pct;
            let pik_amt = coupon_total * pik_pct;

            if cash_amt > 0.0 {
                let kind = if first_last.contains(&d) {
                    CFKind::Stub
                } else {
                    CFKind::Fixed
                };
                new_flows.push(CashFlow {
                    date: d,
                    reset_date: None,
                    amount: Money::new(cash_amt, ccy),
                    kind,
                    accrual_factor: yf,
                    rate: Some(spec.rate),
                });
            }

            pik_to_add += add_pik_flow_if_nonzero(&mut new_flows, d, pik_amt, ccy);
        }
    }
    Ok((pik_to_add, new_flows))
}

/// Emit floating coupon cashflows on a specific date.
///
/// Processes all floating coupon schedules for the given date, looking up forward
/// rates from the optional market context and computing coupon amounts based on
/// `forward_rate * gearing + margin`. Splits into cash/PIK according to coupon type.
///
/// Returns `(pik_to_add, flows)` where `pik_to_add` is the total PIK amount
/// to capitalize into the outstanding balance.
pub(in crate::cashflow::builder) fn emit_float_coupons_on(
    d: Date,
    float_schedules: &[FloatSchedule],
    outstanding_after: &hashbrown::HashMap<Date, f64>,
    outstanding_fallback: f64,
    ccy: Currency,
    _curves: Option<&finstack_core::market_data::MarketContext>,
    resolved_curves: &[Option<Arc<ForwardCurve>>],
) -> finstack_core::Result<(f64, Vec<CashFlow>)> {
    let mut pik_to_add = 0.0;
    let mut new_flows: Vec<CashFlow> = Vec::new();

    for ((spec, _dates, prev_map), resolved_curve) in
        float_schedules.iter().zip(resolved_curves.iter())
    {
        if let Some(prev) = prev_map.get(&d).copied() {
            let base_out = *outstanding_after
                .get(&prev)
                .unwrap_or(&outstanding_fallback);

            // Resolve calendar if present for Bus/252 and similar conventions
            let calendar = spec
                .rate_spec
                .calendar_id
                .as_deref()
                .and_then(finstack_core::dates::calendar::calendar_by_id);

            let yf = spec.rate_spec.dc.year_fraction(
                prev,
                d,
                finstack_core::dates::DayCountCtx {
                    calendar,
                    frequency: Some(spec.rate_spec.reset_freq),
                    bus_basis: None,
                },
            )?;

            // Compute reset date once
            let reset_date = compute_reset_date(
                d,
                spec.rate_spec.reset_lag_days,
                spec.rate_spec.bdc,
                &spec.rate_spec.calendar_id,
            )?;

            // Compute total rate using centralized projection with floor/cap support
            let total_rate = if let Some(fwd) = resolved_curve {
                // Use centralized floating rate projection
                match super::super::rate_helpers::project_floating_rate_with_curve(
                    reset_date,
                    d, // Use payment date as period end approximation
                    spec.rate_spec.spread_bp,
                    spec.rate_spec.gearing,
                    spec.rate_spec.floor_bp,
                    spec.rate_spec.cap_bp,
                    fwd,
                ) {
                    Ok(rate) => rate,
                    Err(_) => {
                        // Curve not found, fall back to spread only with gearing
                        (spec.rate_spec.spread_bp * 1e-4) * spec.rate_spec.gearing
                    }
                }
            } else {
                // No curves provided, use spread only with gearing
                (spec.rate_spec.spread_bp * 1e-4) * spec.rate_spec.gearing
            };

            let coupon_total = base_out * (total_rate * yf);

            let (cash_pct, pik_pct) = spec.coupon_type.split_parts()?;
            let cash_amt = coupon_total * cash_pct;
            let pik_amt = coupon_total * pik_pct;

            if cash_amt > 0.0 {
                new_flows.push(CashFlow {
                    date: d,
                    reset_date: Some(reset_date),
                    amount: Money::new(cash_amt, ccy),
                    kind: CFKind::FloatReset,
                    accrual_factor: yf,
                    rate: Some(total_rate),
                });
            }

            pik_to_add += add_pik_flow_if_nonzero(&mut new_flows, d, pik_amt, ccy);
        }
    }
    Ok((pik_to_add, new_flows))
}
