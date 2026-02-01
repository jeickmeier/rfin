//! Yield-to-worst for term loans with callable schedules.
//!
//! Computes the minimum yield across all valid call dates and final maturity,
//! using holder-view cashflows and outstanding principal for redemption amounts.

use crate::instruments::TermLoan;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::dates::Date;
use finstack_core::money::Money;

use super::irr_helpers::solve_irr_to_exercise;

/// Yield-to-worst calculator for callable term loans.
///
/// Solves for the worst (minimum) yield across all call dates and maturity.
pub struct YtwCalculator;

impl MetricCalculator for YtwCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let loan: &TermLoan = context.instrument_as()?;
        let as_of = context.as_of;

        // Build full schedule to get outstanding path
        let schedule = crate::instruments::fixed_income::term_loan::cashflows::generate_cashflows(
            loan,
            &context.curves,
            as_of,
        )?;

        // Use outstanding_by_date for correct principal path
        let out_path = schedule.outstanding_by_date()?;

        // Candidate exercises: each call and final maturity
        let mut candidates: Vec<(Date, Money)> = Vec::new();
        if let Some(cs) = &loan.call_schedule {
            for c in &cs.calls {
                if c.date < as_of || c.date > loan.maturity {
                    continue;
                }
                // Get outstanding BEFORE the call date (use < not <= because
                // outstanding_by_date returns outstanding AFTER each date)
                let mut outstanding_before = Money::new(0.0, loan.currency);
                for (d, amt) in &out_path {
                    if *d < c.date {
                        outstanding_before = *amt;
                    } else {
                        break;
                    }
                }
                let redemption = Money::new(
                    outstanding_before.amount() * (c.price_pct_of_par / 100.0),
                    loan.currency,
                );
                candidates.push((c.date, redemption));
            }
        }

        // Always include maturity: get outstanding BEFORE maturity
        // (at maturity itself, outstanding is 0 after the final redemption)
        let mut final_out = Money::new(0.0, loan.currency);
        for (d, amt) in &out_path {
            if *d < loan.maturity {
                final_out = *amt;
            } else {
                break;
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
                &context.curves,
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
