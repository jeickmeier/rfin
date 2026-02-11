//! All-in rate metric for term loans.
//!
//! Computes the effective annualized borrower **cash cost** including cash interest
//! and periodic fees, divided by time-weighted outstanding principal.
//!
//! This metric reads cash flows directly from the full cashflow schedule,
//! ensuring perfect consistency with the cashflow generator. PIK interest is
//! excluded from the numerator (cash cost only) but affects outstanding.

use crate::instruments::TermLoan;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::cashflow::CFKind;
use finstack_core::dates::DayCountCtx;

/// All-in rate calculator for term loans.
///
/// Returns the cash-cost all-in rate: (cash interest + fees) / time-weighted outstanding.
/// PIK interest is excluded from the numerator (cash cost only) but affects outstanding
/// (through the outstanding path derived from the full schedule).
///
/// Cash flows are read directly from the generated `CashFlowSchedule` to ensure
/// perfect consistency with the cashflow generator. No fee or rate logic is
/// duplicated.
pub struct AllInRateCalculator;

impl MetricCalculator for AllInRateCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let loan: &TermLoan = context.instrument_as()?;
        let market = &context.curves;
        let as_of = context.as_of;

        // Build full cashflow schedule — the single source of truth for all flows
        let schedule = crate::instruments::fixed_income::term_loan::cashflows::generate_cashflows(
            loan, market, as_of,
        )?;

        // Sum cash interest and fee flows from the schedule (exclude PIK).
        // Only include flows after as_of to match the time-weighted denominator.
        let cash_cost: f64 = schedule
            .flows
            .iter()
            .filter(|cf| cf.date > as_of)
            .filter_map(|cf| match cf.kind {
                CFKind::Fixed | CFKind::FloatReset | CFKind::Stub => Some(cf.amount.amount()),
                CFKind::Fee | CFKind::CommitmentFee | CFKind::UsageFee | CFKind::FacilityFee => {
                    Some(cf.amount.amount())
                }
                _ => None,
            })
            .sum();

        // Compute time-weighted outstanding from the outstanding path.
        // The outstanding path gives balances after all events on each date.
        // We integrate outstanding × year_fraction between consecutive dates.
        let out_path = schedule.outstanding_by_date()?;

        let dc = loan.day_count;
        let mut time_weighted_outstanding = 0.0;

        // Integrate piecewise-constant outstanding over the loan life after as_of.
        let mut prev_date = as_of;
        let mut prev_outstanding = {
            // Look up outstanding at as_of (last entry <= as_of)
            let mut last = 0.0_f64;
            for (d, amt) in &out_path {
                if *d <= as_of {
                    last = amt.amount();
                } else {
                    break;
                }
            }
            last
        };

        // Walk through outstanding path entries after as_of
        for (d, amt) in &out_path {
            if *d <= as_of {
                continue;
            }
            let target = (*d).min(loan.maturity);
            let yf = dc.year_fraction(prev_date, target, DayCountCtx::default())?;
            time_weighted_outstanding += prev_outstanding * yf;
            prev_date = target;
            prev_outstanding = amt.amount();
        }

        // Extend to maturity if the last outstanding entry is before maturity
        if prev_date < loan.maturity {
            let yf = dc.year_fraction(prev_date, loan.maturity, DayCountCtx::default())?;
            time_weighted_outstanding += prev_outstanding * yf;
        }

        if time_weighted_outstanding <= 0.0 {
            return Ok(0.0);
        }
        Ok(cash_cost / time_weighted_outstanding)
    }
}
