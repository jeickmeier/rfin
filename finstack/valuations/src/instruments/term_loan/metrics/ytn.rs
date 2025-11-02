//! Yield-to-N-years calculators for term loans.

use crate::instruments::TermLoan;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::dates::Date;
use finstack_core::money::Money;

fn solve_irr_to_date(
    loan: &TermLoan,
    schedule: &crate::cashflow::builder::schedule::CashFlowSchedule,
    as_of: Date,
    target_price: Money,
    exercise_date: Date,
) -> finstack_core::Result<f64> {
    let mut flows: Vec<(Date, Money)> = Vec::new();
    // Include initial outflow equal to current base PV
    flows.push((as_of, Money::new(-target_price.amount(), target_price.currency())));
    for cf in &schedule.flows {
        if cf.date <= as_of || cf.date > exercise_date { continue; }
        flows.push((cf.date, cf.amount));
    }

    // Redemption = outstanding at exercise date
    let out_path = schedule.outstanding_path();
    let mut outstanding_at = Money::new(0.0, loan.currency);
    for (d, amt) in &out_path { if *d <= exercise_date { outstanding_at = *amt; } else { break; } }
    flows.push((exercise_date, outstanding_at));

    crate::instruments::private_markets_fund::metrics::calculate_irr(&flows, loan.day_count)
}

fn years_ahead(as_of: Date, years: i32) -> Date {
    // Attempt to construct same month/day years ahead; fall back to maturity elsewhere
    // We'll clamp invalid days (e.g., Feb 29) by using last valid day of month via simple fallback to day 28
    // but since we only use min with maturity, we can try direct construct and on error, use as_of end-of-month heuristic.
    match Date::from_calendar_date(as_of.year() + years, as_of.month(), as_of.day()) {
        Ok(d) => d,
        Err(_) => {
            // Fallback to 28th of same month
            let day = 28;
            Date::from_calendar_date(as_of.year() + years, as_of.month(), day).unwrap_or(as_of)
        }
    }
}

macro_rules! define_ytn {
    ($name:ident, $years:expr) => {
        pub struct $name;

        impl MetricCalculator for $name {
            fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
                let loan: &TermLoan = context.instrument_as()?;
                let as_of = context.as_of;

                let target = years_ahead(as_of, $years);
                let exercise_date = if target < loan.maturity { target } else { loan.maturity };
                if exercise_date <= as_of {
                    return Ok(0.0);
                }

                let schedule = crate::instruments::term_loan::cashflows::generate_cashflows(
                    loan,
                    &context.curves,
                    as_of,
                )?;

                solve_irr_to_date(loan, &schedule, as_of, context.base_value, exercise_date)
            }
        }
    };
}

define_ytn!(Yt2yCalculator, 2);
define_ytn!(Yt3yCalculator, 3);
define_ytn!(Yt4yCalculator, 4);


