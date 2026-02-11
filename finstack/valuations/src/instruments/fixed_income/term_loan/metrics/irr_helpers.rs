//! Shared IRR calculation helpers for term loan yield metrics.
//!
//! This module provides unified IRR solving for YTM, YTC, YTW, and YT2Y/3Y/4Y metrics.
//!
//! All helpers use **kind-aware cashflow filtering** from the full schedule to ensure
//! correct treatment at exercise boundaries:
//! - Coupons/fees (all variants: `Fee`, `CommitmentFee`, `UsageFee`, `FacilityFee`):
//!   included up to AND including the exercise date
//! - Amortization/Notional: included only BEFORE the exercise date (implicitly
//!   captured in the pre-exercise outstanding used for the redemption leg)
//! - PIK and negative Notional (funding): always excluded

use crate::cashflow::builder::schedule::CashFlowSchedule;
use crate::instruments::TermLoan;
use finstack_core::cashflow::{CFKind, InternalRateOfReturn};
use finstack_core::dates::Date;
use finstack_core::money::Money;

/// Solve IRR to an exercise date using kind-aware cashflow filtering.
///
/// This is the core IRR solver used by YTC and YTW metrics.
///
/// # Flow selection
///
/// Uses the full cashflow schedule for precise kind-based filtering:
/// - **Coupons and fees** (`Fixed`, `FloatReset`, `Stub`, `Fee`, `CommitmentFee`,
///   `UsageFee`, `FacilityFee`): included up to AND including `exercise_date` --
///   the holder receives accrued interest and fee payments on the exercise date.
/// - **Amortization** and positive **Notional** (redemptions): included only
///   BEFORE `exercise_date`.  At the exercise date, amortization is implicitly
///   captured in the pre-exercise outstanding used for the redemption parameter.
/// - **PIK** and negative **Notional** (funding legs): always excluded.
///
/// # Arguments
///
/// * `loan` - The term loan instrument
/// * `schedule` - Pre-computed full cashflow schedule (avoids regeneration)
/// * `as_of` - Valuation date
/// * `target_price` - Purchase price (dirty price, typically base PV)
/// * `exercise_date` - Exercise/call/maturity date
/// * `redemption` - Redemption amount at exercise date
///
/// # Returns
///
/// IRR (as decimal) that equates the initial price to the present value of
/// holder-view flows plus redemption.
pub(super) fn solve_irr_to_exercise(
    loan: &TermLoan,
    schedule: &CashFlowSchedule,
    as_of: Date,
    target_price: Money,
    exercise_date: Date,
    redemption: Money,
) -> finstack_core::Result<f64> {
    // Compute settlement date using loan calendar/business-day conventions.
    let settlement_date = loan.settlement_date(as_of)?;

    let mut flows: Vec<(Date, f64)> = Vec::with_capacity(schedule.flows.len() + 2);

    // Initial price leg at settlement date (negative = cash outflow for purchase)
    flows.push((settlement_date, -target_price.amount()));

    // Kind-aware flow selection from the full schedule.
    // At the exercise date: include coupon/fee flows (holder receives accrued
    // interest) but exclude Amortization and Notional (the explicit redemption
    // parameter replaces them, using the pre-exercise outstanding).
    for cf in &schedule.flows {
        if cf.date <= settlement_date || cf.date > exercise_date {
            continue;
        }
        match cf.kind {
            // Coupons, interest, and all fee variants: include up to AND including
            // exercise date.  We match all fee kinds (`Fee`, `CommitmentFee`,
            // `UsageFee`, `FacilityFee`) to be forward-compatible if the cashflow
            // builder ever emits these specific fee variants directly.
            CFKind::Fixed
            | CFKind::FloatReset
            | CFKind::Stub
            | CFKind::Fee
            | CFKind::CommitmentFee
            | CFKind::UsageFee
            | CFKind::FacilityFee => {
                flows.push((cf.date, cf.amount.amount()));
            }
            // Amortization: include only BEFORE exercise date.
            // At exercise date, amort is implicitly captured in the pre-amort
            // outstanding used for the redemption calculation.
            CFKind::Amortization if cf.date < exercise_date => {
                flows.push((cf.date, cf.amount.amount()));
            }
            // Positive Notional (redemptions): include only BEFORE exercise date.
            // At exercise date, the explicit redemption parameter replaces any
            // scheduled Notional to avoid double-counting.
            CFKind::Notional if cf.date < exercise_date && cf.amount.amount() > 0.0 => {
                flows.push((cf.date, cf.amount.amount()));
            }
            // Exclude: PIK, negative Notional (funding), exercise-date
            // Amortization/Notional
            _ => {}
        }
    }

    // Add explicit redemption at exercise date
    flows.push((exercise_date, redemption.amount()));

    flows.as_slice().irr_with_daycount(loan.day_count, None)
}

/// Solve IRR to a fixed horizon using kind-aware filtering and outstanding at horizon.
///
/// This is the core IRR solver used by YT2Y/3Y/4Y metrics.  The redemption is
/// the pre-exercise outstanding principal (the "sale" price at the horizon).
///
/// Uses the same kind-aware filtering convention as [`solve_irr_to_exercise`].
///
/// # Arguments
///
/// * `loan` - The term loan instrument
/// * `curves` - Market context for cashflow generation
/// * `as_of` - Valuation date
/// * `target_price` - Purchase price (dirty price, typically base PV)
/// * `exercise_date` - Horizon date (typically 2Y/3Y/4Y from as_of)
///
/// # Returns
///
/// IRR (as decimal) that equates the initial price to the present value of
/// holder-view flows plus outstanding principal at the horizon.
pub(super) fn solve_irr_to_date(
    loan: &TermLoan,
    curves: &finstack_core::market_data::context::MarketContext,
    as_of: Date,
    target_price: Money,
    exercise_date: Date,
) -> finstack_core::Result<f64> {
    // Build full schedule to get outstanding path and flows
    let schedule = crate::instruments::fixed_income::term_loan::cashflows::generate_cashflows(
        loan, curves, as_of,
    )?;

    // Pre-exercise outstanding (used as "sale" price at horizon)
    let out_path = schedule.outstanding_by_date()?;
    let outstanding = outstanding_before(&out_path, exercise_date, loan.currency);

    // Delegate to the unified exercise solver
    solve_irr_to_exercise(
        loan,
        &schedule,
        as_of,
        target_price,
        exercise_date,
        outstanding,
    )
}

/// Look up outstanding BEFORE a target date from the outstanding path.
///
/// Uses `<` comparison since `outstanding_by_date()` returns balances AFTER
/// processing all flows on each date.  This gives the balance just before
/// any events (amortization, notional, PIK) on the target date.
///
/// # Precondition
///
/// `out_path` must be sorted by date (as returned by `outstanding_by_date()`).
pub(super) fn outstanding_before(
    out_path: &[(Date, Money)],
    target: Date,
    currency: finstack_core::currency::Currency,
) -> Money {
    let mut last = Money::new(0.0, currency);
    for (d, amt) in out_path {
        if *d < target {
            last = *amt;
        } else {
            break;
        }
    }
    last
}
