//! Cashflow emission helpers.
//!
//! This module contains functions that emit cashflows on specific dates based on
//! coupon schedules, amortization specs, and fee specifications. These functions
//! are called by the build pipeline to generate deterministic cashflow sequences.
//!
//! ## Responsibilities
//!
//! - Emit fixed coupon cashflows with PIK capitalization
//! - Emit floating coupon cashflows with forward rate lookups
//! - Emit amortization payments according to various schedules
//! - Emit periodic and fixed fee cashflows
//! - Track outstanding balances through PIK and amortization
//!
//! ## Design
//!
//! Each `emit_*_on` function takes a date and relevant schedules, computes the
//! appropriate cashflows for that date, and returns both the flows and any PIK
//! amount that should capitalize into the outstanding balance.

use crate::cashflow::primitives::{AmortizationSpec, Notional, CFKind, CashFlow};
use finstack_core::currency::Currency;
use finstack_core::dates::{adjust, Date};
use finstack_core::dates::calendar::calendar_by_id;
use finstack_core::error::InputError;
use finstack_core::money::Money;
use time::Duration;

use super::compiler::{FixedSchedule, FloatSchedule, PeriodicFee};
use super::specs::FeeBase;

// -------------------------------------------------------------------------
// Helper functions
// -------------------------------------------------------------------------

/// Add a PIK cashflow if the amount is nonzero.
///
/// Returns the PIK amount for outstanding balance tracking.
#[inline]
pub(super) fn add_pik_flow_if_nonzero(
    flows: &mut Vec<CashFlow>,
    date: Date,
    pik_amt: f64,
    ccy: Currency,
) -> f64 {
    if pik_amt > 0.0 {
        flows.push(CashFlow {
            date,
            reset_date: None,
            amount: Money::new(pik_amt, ccy),
            kind: CFKind::PIK,
            accrual_factor: 0.0,
            rate: None,
        });
        pik_amt
    } else {
        0.0
    }
}

/// Compute reset date with calendar adjustment.
///
/// Applies business day convention and calendar adjustment to the reset date
/// derived from the payment date minus reset lag days.
#[inline]
pub(super) fn compute_reset_date(
    payment_date: Date,
    reset_lag_days: i32,
    bdc: finstack_core::dates::BusinessDayConvention,
    calendar_id: &Option<String>,
) -> finstack_core::Result<Date> {
    let mut reset_date = payment_date - Duration::days(reset_lag_days as i64);
    if let Some(id) = calendar_id {
        if let Some(cal) = calendar_by_id(id) {
            reset_date = adjust(reset_date, bdc, cal)?;
        }
    }
    Ok(reset_date)
}

// -------------------------------------------------------------------------
// Emission functions
// -------------------------------------------------------------------------

/// Emit fixed coupon cashflows on a specific date.
///
/// Processes all fixed coupon schedules for the given date, computing coupon
/// amounts based on outstanding balances and splitting into cash/PIK according
/// to the coupon type.
///
/// Returns `(pik_to_add, flows)` where `pik_to_add` is the total PIK amount
/// to capitalize into the outstanding balance.
pub(super) fn emit_fixed_coupons_on(
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

            let yf =
                spec.dc
                    .year_fraction(prev, d, finstack_core::dates::DayCountCtx::default())?;
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
pub(super) fn emit_float_coupons_on(
    d: Date,
    float_schedules: &[FloatSchedule],
    outstanding_after: &hashbrown::HashMap<Date, f64>,
    outstanding_fallback: f64,
    ccy: Currency,
    curves: Option<&finstack_core::market_data::MarketContext>,
) -> finstack_core::Result<(f64, Vec<CashFlow>)> {
    let mut pik_to_add = 0.0;
    let mut new_flows: Vec<CashFlow> = Vec::new();

    for (spec, _dates, prev_map) in float_schedules {
        if let Some(prev) = prev_map.get(&d).copied() {
            let base_out = *outstanding_after
                .get(&prev)
                .unwrap_or(&outstanding_fallback);

            let yf = spec
                .rate_spec
                .dc
                .year_fraction(prev, d, finstack_core::dates::DayCountCtx::default())?;

            // Compute reset date once
            let reset_date = compute_reset_date(
                d,
                spec.rate_spec.reset_lag_days,
                spec.rate_spec.bdc,
                &spec.rate_spec.calendar_id,
            )?;

            // Compute total rate using centralized projection with floor/cap support
            let total_rate = if let Some(ctx) = curves {
                // Use centralized floating rate projection
                match super::rate_helpers::project_floating_rate(
                    reset_date,
                    d, // Use payment date as period end approximation
                    spec.rate_spec.index_id.as_str(),
                    spec.rate_spec.spread_bp,
                    spec.rate_spec.gearing,
                    spec.rate_spec.floor_bp,
                    spec.rate_spec.cap_bp,
                    ctx,
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

/// Amortization parameters for emission.
///
/// Contains precomputed values and maps needed by `emit_amortization_on` to
/// process various amortization specifications efficiently.
#[derive(Debug, Clone)]
pub(super) struct AmortizationParams<'a> {
    pub(super) ccy: Currency,
    pub(super) amort_dates: &'a hashbrown::HashSet<Date>,
    pub(super) linear_delta: Option<f64>,
    pub(super) percent_per: Option<f64>,
    pub(super) step_remaining_map: &'a Option<hashbrown::HashMap<Date, Money>>,
}

/// Emit amortization cashflows on a specific date.
///
/// Processes the notional's amortization specification to generate principal
/// repayment flows. Mutates the `outstanding` balance in-place to reflect
/// the reduction from amortization.
///
/// Supports:
/// - LinearTo: Equal installments over schedule
/// - StepRemaining: Specific remaining balance targets
/// - PercentPerPeriod: Percentage of current outstanding
/// - CustomPrincipal: Explicit payment amounts by date
pub(super) fn emit_amortization_on(
    d: Date,
    notional: &Notional,
    outstanding: &mut f64,
    params: &AmortizationParams,
) -> finstack_core::Result<Vec<CashFlow>> {
    let mut new_flows: Vec<CashFlow> = Vec::new();
    match &notional.amort {
        AmortizationSpec::None => {}
        AmortizationSpec::LinearTo { .. } => {
            if params.amort_dates.contains(&d) {
                if let Some(delta) = params.linear_delta {
                    let pay = delta.min(*outstanding);
                    if pay > 0.0 {
                        new_flows.push(CashFlow {
                            date: d,
                            reset_date: None,
                            amount: Money::new(pay, params.ccy),
                            kind: CFKind::Amortization,
                            accrual_factor: 0.0,
                            rate: None,
                        });
                        *outstanding -= pay;
                    }
                }
            }
        }
        AmortizationSpec::StepRemaining { .. } => {
            if let Some(map) = params.step_remaining_map {
                if let Some(rem_after) = map.get(&d) {
                    let target = rem_after.amount();
                    let pay = (*outstanding - target).max(0.0).min(*outstanding);
                    if pay > 0.0 {
                        new_flows.push(CashFlow {
                            date: d,
                            reset_date: None,
                            amount: Money::new(pay, params.ccy),
                            kind: CFKind::Amortization,
                            accrual_factor: 0.0,
                            rate: None,
                        });
                        *outstanding -= pay;
                    }
                }
            }
        }
        AmortizationSpec::PercentPerPeriod { .. } => {
            if params.amort_dates.contains(&d) {
                if let Some(per) = params.percent_per {
                    let pay = per.min(*outstanding);
                    if pay > 0.0 {
                        new_flows.push(CashFlow {
                            date: d,
                            reset_date: None,
                            amount: Money::new(pay, params.ccy),
                            kind: CFKind::Amortization,
                            accrual_factor: 0.0,
                            rate: None,
                        });
                        *outstanding -= pay;
                    }
                }
            }
        }
        AmortizationSpec::CustomPrincipal { items } => {
            for (dd, amt) in items {
                if *dd == d {
                    let pay = amt.amount().max(0.0).min(*outstanding);
                    if pay > 0.0 {
                        new_flows.push(CashFlow {
                            date: d,
                            reset_date: None,
                            amount: Money::new(pay, params.ccy),
                            kind: CFKind::Amortization,
                            accrual_factor: 0.0,
                            rate: None,
                        });
                        *outstanding -= pay;
                    }
                }
            }
        }
    }
    Ok(new_flows)
}

/// Emit commitment fee cashflow (fee on undrawn balance).
///
/// Commitment fees are charged on the undrawn portion of a credit facility.
/// Returns a single cashflow with `CFKind::CommitmentFee`.
///
/// # Arguments
///
/// * `d` - Payment date for the fee
/// * `undrawn_balance` - Undrawn balance amount
/// * `commitment_fee_bp` - Fee rate in basis points
/// * `year_fraction` - Accrual period in years
/// * `ccy` - Currency for the cashflow
///
/// # Returns
///
/// Vector containing zero or one cashflow (empty if fee amount is zero)
pub fn emit_commitment_fee_on(
    d: Date,
    undrawn_balance: f64,
    commitment_fee_bp: f64,
    year_fraction: f64,
    ccy: Currency,
) -> Vec<CashFlow> {
    let fee_amt = undrawn_balance * (commitment_fee_bp * 1e-4 * year_fraction);
    if fee_amt > 0.0 {
        vec![CashFlow {
            date: d,
            reset_date: None,
            amount: Money::new(fee_amt, ccy),
            kind: CFKind::CommitmentFee,
            accrual_factor: year_fraction,
            rate: Some(commitment_fee_bp * 1e-4),
        }]
    } else {
        vec![]
    }
}

/// Emit usage fee cashflow (fee on drawn balance).
///
/// Usage fees are charged on the drawn portion of a credit facility.
/// Returns a single cashflow with `CFKind::UsageFee`.
///
/// # Arguments
///
/// * `d` - Payment date for the fee
/// * `drawn_balance` - Drawn balance amount
/// * `usage_fee_bp` - Fee rate in basis points
/// * `year_fraction` - Accrual period in years
/// * `ccy` - Currency for the cashflow
///
/// # Returns
///
/// Vector containing zero or one cashflow (empty if fee amount is zero)
pub fn emit_usage_fee_on(
    d: Date,
    drawn_balance: f64,
    usage_fee_bp: f64,
    year_fraction: f64,
    ccy: Currency,
) -> Vec<CashFlow> {
    let fee_amt = drawn_balance * (usage_fee_bp * 1e-4 * year_fraction);
    if fee_amt > 0.0 {
        vec![CashFlow {
            date: d,
            reset_date: None,
            amount: Money::new(fee_amt, ccy),
            kind: CFKind::UsageFee,
            accrual_factor: year_fraction,
            rate: Some(usage_fee_bp * 1e-4),
        }]
    } else {
        vec![]
    }
}

/// Emit facility fee cashflow (fee on total commitment).
///
/// Facility fees are charged on the entire commitment amount regardless of utilization.
/// Returns a single cashflow with `CFKind::FacilityFee`.
///
/// # Arguments
///
/// * `d` - Payment date for the fee
/// * `commitment_amount` - Total commitment amount
/// * `facility_fee_bp` - Fee rate in basis points
/// * `year_fraction` - Accrual period in years
/// * `ccy` - Currency for the cashflow
///
/// # Returns
///
/// Vector containing zero or one cashflow (empty if fee amount is zero)
pub fn emit_facility_fee_on(
    d: Date,
    commitment_amount: f64,
    facility_fee_bp: f64,
    year_fraction: f64,
    ccy: Currency,
) -> Vec<CashFlow> {
    let fee_amt = commitment_amount * (facility_fee_bp * 1e-4 * year_fraction);
    if fee_amt > 0.0 {
        vec![CashFlow {
            date: d,
            reset_date: None,
            amount: Money::new(fee_amt, ccy),
            kind: CFKind::FacilityFee,
            accrual_factor: year_fraction,
            rate: Some(facility_fee_bp * 1e-4),
        }]
    } else {
        vec![]
    }
}

/// Emit fee cashflows on a specific date.
///
/// Processes both periodic fees (based on drawn/undrawn balances) and fixed
/// fees (explicit amounts) that fall on the given date.
///
/// For periodic fees, computes the fee amount as `base * bps * year_fraction`
/// where base is either the drawn balance or the undrawn balance (facility_limit - outstanding).
pub(super) fn emit_fees_on(
    d: Date,
    periodic_fees: &[PeriodicFee],
    fixed_fees: &[(Date, Money)],
    outstanding: f64,
    ccy: Currency,
) -> finstack_core::Result<Vec<CashFlow>> {
    let mut new_flows: Vec<CashFlow> = Vec::new();
    for pf in periodic_fees {
        if let Some(&prev) = pf.prev.get(&d) {
            let yf = pf
                .dc
                .year_fraction(prev, d, finstack_core::dates::DayCountCtx::default())?;
            let base_amt = match &pf.base {
                FeeBase::Drawn => outstanding,
                FeeBase::Undrawn { facility_limit } => {
                    if facility_limit.currency() != ccy {
                        return Err(InputError::Invalid.into());
                    }
                    (facility_limit.amount() - outstanding).max(0.0)
                }
            };
            let fee_amt = base_amt * (pf.bps * 1e-4 * yf);
            if fee_amt > 0.0 {
                new_flows.push(CashFlow {
                    date: d,
                    reset_date: None,
                    amount: Money::new(fee_amt, ccy),
                    kind: CFKind::Fee,
                    accrual_factor: 0.0,
                    rate: Some(pf.bps * 1e-4),
                });
            }
        }
    }

    for (fd, amt) in fixed_fees {
        if *fd == d && amt.amount() != 0.0 {
            new_flows.push(CashFlow {
                date: d,
                reset_date: None,
                amount: *amt,
                kind: CFKind::Fee,
                accrual_factor: 0.0,
                rate: None,
            });
        }
    }
    Ok(new_flows)
}

