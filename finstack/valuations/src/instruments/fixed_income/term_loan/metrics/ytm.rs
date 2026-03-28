//! YTM metric for term loans via IRR solving.
//!
//! Yield-to-maturity is computed using the signed canonical schedule (excluding
//! funding legs) with an initial price leg at `as_of` equal to the negative base PV.
//! Uses the same IRR engine and day-count as the loan for consistency.

use crate::cashflow::traits::CashflowProvider;
use crate::instruments::TermLoan;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::money::Money;

use super::irr_helpers::target_price_from_quote_or_model;

/// Yield-to-maturity calculator for term loans.
///
/// Solves for the IRR using signed canonical schedule flows (coupons, amortization, redemptions only)
/// plus an initial price leg at as_of.
pub struct YtmCalculator;

impl MetricCalculator for YtmCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let loan: &TermLoan = context.instrument_as()?;
        let as_of = context.as_of;

        // Compute settlement date using loan calendar/business-day conventions.
        let settlement_date = loan.settlement_date(as_of)?;

        // Use signed canonical schedule (via CashflowProvider::dated_cashflows)
        // This filters to contractual inflows: coupons, amortization, positive redemptions
        let holder_flows = loan.dated_cashflows(&context.curves, as_of)?;

        let mut flows: Vec<(finstack_core::dates::Date, Money)> =
            Vec::with_capacity(holder_flows.len() + 1);

        // Add initial price leg at settlement_date (negative = outflow for purchase)
        let target_price = target_price_from_quote_or_model(loan, context.base_value);
        flows.push((
            settlement_date,
            Money::new(-target_price.amount(), target_price.currency()),
        ));

        // Add signed canonical schedule flows after settlement_date
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
