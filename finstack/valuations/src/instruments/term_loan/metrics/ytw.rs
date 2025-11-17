//! Yield-to-worst for term loans with callable schedules.
//!
//! Computes the minimum yield across all valid call dates and final maturity,
//! using holder-view cashflows and outstanding principal for redemption amounts.

use crate::instruments::TermLoan;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::dates::Date;
use finstack_core::money::Money;

/// Yield-to-worst calculator for callable term loans.
///
/// Solves for the worst (minimum) yield across all call dates and maturity.
pub struct YtwCalculator;

impl MetricCalculator for YtwCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let loan: &TermLoan = context.instrument_as()?;
        let as_of = context.as_of;

        // Build full schedule to get outstanding path
        let schedule = crate::instruments::term_loan::cashflows::generate_cashflows(
            loan,
            &context.curves,
            as_of,
        )?;

        // Use outstanding_by_date_including_notional for correct principal path
        let out_path = schedule.outstanding_by_date_including_notional();

        // Candidate exercises: each call and final maturity
        let mut candidates: Vec<(Date, Money)> = Vec::new();
        if let Some(cs) = &loan.call_schedule {
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

/// Solve IRR to an exercise date using holder-view cashflows.
fn solve_irr_to_exercise(
    loan: &TermLoan,
    curves: &finstack_core::market_data::MarketContext,
    as_of: Date,
    target_price: Money,
    exercise_date: Date,
    redemption: Money,
) -> finstack_core::Result<f64> {
    use crate::cashflow::traits::CashflowProvider;
    
    // Get holder-view flows
    let holder_flows = loan.build_schedule(curves, as_of)?;
    
    let mut flows: Vec<(Date, Money)> = Vec::new();
    // Initial price leg
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
    
    // Add redemption
    flows.push((exercise_date, redemption));

    crate::instruments::private_markets_fund::metrics::calculate_irr(&flows, loan.day_count)
}
