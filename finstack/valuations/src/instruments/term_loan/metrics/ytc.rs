//! Yield-to-first-call for term loans.
//!
//! Computes the IRR to the earliest valid call date, using the holder-view cashflows
//! up to the call date plus the call redemption based on outstanding principal.

use crate::instruments::TermLoan;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::dates::Date;
use finstack_core::money::Money;

/// Yield-to-call calculator for callable term loans.
///
/// For loans with call schedules, solves for IRR to the first valid call date.
/// Redemption amount equals outstanding principal at call date times the call price.
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
        let call = first_call.expect("First call should exist when YTC calculation is requested");

        // Build full schedule to get outstanding path including notional draws/repays
        let schedule = crate::instruments::term_loan::cashflows::generate_cashflows(
            loan,
            &context.curves,
            as_of,
        )?;
        
        // Use outstanding_by_date_including_notional to get correct principal path
        let out_path = schedule.outstanding_by_date_including_notional();
        let mut outstanding_at = Money::new(0.0, loan.currency);
        for (d, amt) in &out_path {
            if *d <= call.date {
                outstanding_at = *amt;
            } else {
                break;
            }
        }
        
        // Redemption = outstanding * call price (as percentage of par)
        let redemption = Money::new(
            outstanding_at.amount() * (call.price_pct_of_par / 100.0),
            loan.currency,
        );

        solve_irr_to_exercise(
            loan,
            &context.curves,
            as_of,
            context.base_value,
            call.date,
            redemption,
        )
    }
}

/// Solve IRR to an exercise date using holder-view cashflows.
///
/// Builds a flow sequence: initial price leg, holder-view flows up to exercise, and redemption.
fn solve_irr_to_exercise(
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
    
    // Add call redemption
    flows.push((exercise_date, redemption));
    
    crate::instruments::private_markets_fund::metrics::calculate_irr(&flows, loan.day_count)
}
