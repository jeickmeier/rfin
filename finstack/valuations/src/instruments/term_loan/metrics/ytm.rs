//! YTM metric for term loans via IRR solving.

use crate::instruments::TermLoan;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::money::Money;

/// Yield-to-maturity calculator for term loans
pub struct YtmCalculator;

impl MetricCalculator for YtmCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let loan: &TermLoan = context.instrument_as()?;
        let as_of = context.as_of;

        // Build full schedule and convert to (Date, Money)
        let sched = crate::instruments::term_loan::cashflows::generate_cashflows(
            loan,
            &context.curves,
            as_of,
        )?;

        let mut flows: Vec<(finstack_core::dates::Date, Money)> =
            Vec::with_capacity(sched.flows.len() + 1);
        // Add initial outflow equal to -PV at as_of to emulate market price
        let base_pv = context.base_value;
        flows.push((as_of, Money::new(-base_pv.amount(), base_pv.currency())));
        for cf in &sched.flows {
            if cf.date <= as_of {
                continue;
            }
            flows.push((cf.date, cf.amount));
        }

        crate::instruments::private_markets_fund::metrics::calculate_irr(&flows, loan.day_count)
    }
}
