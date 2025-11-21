//! Shared IRR calculation helpers for term loan yield metrics.
//!
//! This module provides unified IRR solving for YTM, YTC, YTW, and YT2Y/3Y/4Y metrics.

// Allow dead_code warnings for pub(super) functions used by sibling modules
#![allow(dead_code)]

use crate::instruments::TermLoan;
use finstack_core::cashflow::xirr::xirr_with_daycount;
use finstack_core::dates::Date;
use finstack_core::money::Money;

/// Solve IRR to an exercise date using holder-view cashflows and explicit redemption.
///
/// This is the core IRR solver used by YTC and YTW metrics.
///
/// # Arguments
///
/// * `loan` - The term loan instrument
/// * `curves` - Market context for cashflow generation
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
    curves: &finstack_core::market_data::MarketContext,
    as_of: Date,
    target_price: Money,
    exercise_date: Date,
    redemption: Money,
) -> finstack_core::Result<f64> {
    use crate::cashflow::traits::CashflowProvider;

    // Get holder-view flows (coupons, amortization, positive redemptions only)
    let holder_flows = loan.build_schedule(curves, as_of)?;

    let mut flows: Vec<(Date, Money)> = Vec::new();

    // Initial price leg (negative = cash outflow for purchase)
    flows.push((
        as_of,
        Money::new(-target_price.amount(), target_price.currency()),
    ));

    // Add holder-view flows up to exercise date
    for (date, amount) in holder_flows {
        if date > as_of && date <= exercise_date {
            flows.push((date, amount));
        }
    }

    // Add redemption at exercise date
    flows.push((exercise_date, redemption));

    // Convert flows to (Date, f64) for XIRR
    let flows_f64: Vec<(Date, f64)> = flows.iter().map(|(d, m)| (*d, m.amount())).collect();

    xirr_with_daycount(&flows_f64, loan.day_count, None)
}

/// Solve IRR to a fixed horizon using holder-view cashflows and outstanding at horizon.
///
/// This is the core IRR solver used by YT2Y/3Y/4Y metrics. The redemption is computed
/// as outstanding principal at the exercise date.
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
    curves: &finstack_core::market_data::MarketContext,
    as_of: Date,
    target_price: Money,
    exercise_date: Date,
) -> finstack_core::Result<f64> {
    // Build full schedule to get outstanding path
    let schedule =
        crate::instruments::term_loan::cashflows::generate_cashflows(loan, curves, as_of)?;

    // Get outstanding at exercise date
    let out_path = schedule.outstanding_by_date_including_notional();
    let mut outstanding_at = Money::new(0.0, loan.currency);
    for (d, amt) in &out_path {
        if *d <= exercise_date {
            outstanding_at = *amt;
        } else {
            break;
        }
    }

    // Use the common helper with outstanding as redemption
    solve_irr_to_exercise(
        loan,
        curves,
        as_of,
        target_price,
        exercise_date,
        outstanding_at,
    )
}
