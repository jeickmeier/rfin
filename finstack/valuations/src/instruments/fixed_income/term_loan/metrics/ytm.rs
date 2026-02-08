//! YTM metric for term loans via IRR solving.
//!
//! Yield-to-maturity is computed using the holder-view cashflow schedule (excluding
//! funding legs) with an initial price leg at `as_of` equal to the negative base PV.
//! Uses the same IRR engine and day-count as the loan for consistency.

use crate::cashflow::traits::CashflowProvider;
use crate::instruments::TermLoan;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::money::Money;

/// Yield-to-maturity calculator for term loans.
///
/// Solves for the IRR using holder-view flows (coupons, amortization, redemptions only)
/// plus an initial price leg at as_of.
pub struct YtmCalculator;

impl MetricCalculator for YtmCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let loan: &TermLoan = context.instrument_as()?;
        let as_of = context.as_of;

        // Compute settlement date (T+n per LSTA conventions)
        let settlement_date = as_of + time::Duration::days(i64::from(loan.settlement_days));

        // Use holder-view schedule (via CashflowProvider::build_dated_flows)
        // This filters to contractual inflows: coupons, amortization, positive redemptions
        let holder_flows = loan.build_dated_flows(&context.curves, as_of)?;

        let mut flows: Vec<(finstack_core::dates::Date, Money)> =
            Vec::with_capacity(holder_flows.len() + 1);

        // Add initial price leg at settlement_date (negative = outflow for purchase)
        let base_pv = context.base_value;
        flows.push((
            settlement_date,
            Money::new(-base_pv.amount(), base_pv.currency()),
        ));

        // Add holder-view flows after settlement_date
        for (date, amount) in holder_flows {
            if date > settlement_date {
                flows.push((date, amount));
            }
        }

        // Convert flows to (Date, f64) for XIRR
        let flows_f64: Vec<(finstack_core::dates::Date, f64)> =
            flows.iter().map(|(d, m)| (*d, m.amount())).collect();

        // Solve IRR using the loan's day-count convention
        use finstack_core::cashflow::InternalRateOfReturn;
        flows_f64.irr_with_daycount(loan.day_count, None)
    }
}
