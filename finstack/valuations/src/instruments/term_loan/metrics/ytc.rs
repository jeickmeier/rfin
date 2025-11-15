//! Yield-to-first-call for term loans.

use crate::instruments::TermLoan;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::dates::Date;
use finstack_core::money::Money;

/// Yield-to-call calculator for callable term loans
pub struct YtcCalculator;

impl MetricCalculator for YtcCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let loan: &TermLoan = context.instrument_as()?;
        let as_of = context.as_of;

        // No calls → fallback to YTM
        let first_call = match &loan.call_schedule {
            Some(cs) => cs
                .calls
                .iter()
                .filter(|c| c.date >= as_of && c.date <= loan.maturity)
                .min_by_key(|c| c.date)
                .cloned(),
            None => None,
        };

        if first_call.is_none() {
            // use YTM calculator already registered
            return crate::instruments::term_loan::metrics::ytm::YtmCalculator.calculate(context);
        }
        let call = first_call
            .expect("First call should exist when YTC calculation is requested");

        // Build full schedule and compute outstanding at call
        let schedule = crate::instruments::term_loan::cashflows::generate_cashflows(
            loan,
            &context.curves,
            as_of,
        )?;
        let out_path = schedule.outstanding_path();
        let mut outstanding_at = Money::new(0.0, loan.currency);
        for (d, amt) in &out_path {
            if *d <= call.date {
                outstanding_at = *amt;
            } else {
                break;
            }
        }
        let redemption = Money::new(
            outstanding_at.amount() * (call.price_pct_of_par / 100.0),
            loan.currency,
        );

        solve_irr_to_exercise(
            loan,
            &schedule,
            as_of,
            context.base_value,
            call.date,
            redemption,
        )
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
