//! Yield-to-worst for term loans with callable schedules.

use crate::instruments::TermLoan;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::dates::Date;
use finstack_core::money::Money;

/// Yield-to-worst calculator for callable term loans
pub struct YtwCalculator;

impl MetricCalculator for YtwCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let loan: &TermLoan = context.instrument_as()?;
        let as_of = context.as_of;

        // Build full schedule once
        let schedule = crate::instruments::term_loan::cashflows::generate_cashflows(
            loan,
            &context.curves,
            as_of,
        )?;

        // Candidate exercises: each call and final maturity
        let mut candidates: Vec<(Date, Money)> = Vec::new();
        if let Some(cs) = &loan.call_schedule {
            // Precompute outstanding path for good redemption base
            let out_path = schedule.outstanding_path();
            for c in &cs.calls {
                if c.date < as_of || c.date > loan.maturity {
                    continue;
                }
                let mut outstanding_at = Money::new(0.0, loan.currency);
                for (d, amt) in &out_path {
                    if *d <= c.date {
                        outstanding_at = *amt;
                    } else {
                        break;
                    }
                }
                let redemption = Money::new(
                    outstanding_at.amount() * (c.price_pct_of_par / 100.0),
                    loan.currency,
                );
                candidates.push((c.date, redemption));
            }
        }
        // Always include maturity redemption of remaining outstanding
        let out_path = schedule.outstanding_path();
        let mut final_out = Money::new(0.0, loan.currency);
        for (d, amt) in &out_path {
            if *d <= loan.maturity {
                final_out = *amt;
            }
        }
        candidates.push((loan.maturity, final_out));

        // Target price: use base PV from context
        let dirty_now = context.base_value;

        // Worst yield = min yield across candidates
        let mut y_worst = f64::INFINITY;
        for (exercise_date, redemption) in candidates {
            let y = solve_irr_to_exercise(
                loan,
                &schedule,
                as_of,
                dirty_now,
                exercise_date,
                redemption,
            )?;
            if y < y_worst {
                y_worst = y;
            }
        }
        Ok(y_worst)
    }
}

fn solve_irr_to_exercise(
    loan: &TermLoan,
    schedule: &crate::cashflow::builder::schedule::CashFlowSchedule,
    as_of: Date,
    target_price: Money,
    exercise_date: Date,
    redemption: Money,
) -> finstack_core::Result<f64> {
    let mut flows: Vec<(Date, Money)> = Vec::new();
    // Include initial outflow equal to current price
    flows.push((
        as_of,
        Money::new(-target_price.amount(), target_price.currency()),
    ));
    for cf in &schedule.flows {
        if cf.date <= as_of || cf.date > exercise_date {
            continue;
        }
        flows.push((cf.date, cf.amount));
    }
    flows.push((exercise_date, redemption));

    crate::instruments::private_markets_fund::metrics::calculate_irr(&flows, loan.day_count)
}
