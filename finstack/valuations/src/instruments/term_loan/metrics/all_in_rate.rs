//! All-in rate metric for term loans.
//!
//! Approximates the effective annualized borrower cost including cash interest
//! and periodic fees, divided by time-weighted outstanding principal.

use crate::instruments::TermLoan;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::dates::DayCountCtx;
use finstack_core::money::Money;

pub struct AllInRateCalculator;

impl MetricCalculator for AllInRateCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let loan: &TermLoan = context.instrument_as()?;
        let market = &context.curves;
        let as_of = context.as_of;

        // Build coupon schedule
        let sched = finstack_core::dates::ScheduleBuilder::new(loan.issue, loan.maturity)
            .frequency(loan.pay_freq)
            .stub_rule(loan.stub)
            .build()?;
        let mut dates: Vec<finstack_core::dates::Date> = sched.into_iter().collect();
        if dates.first().copied() != Some(loan.issue) { dates.insert(0, loan.issue); }

        // Track outstanding using simple path: initial draws and PIK add; sweeps/amort subtract
        let mut outstanding = Money::new(0.0, loan.currency);
        if let Some(ddtl) = &loan.ddtl {
            let draw_stop = loan
                .covenants
                .as_ref()
                .and_then(|c| c.draw_stop_dates.iter().min().copied());
            for ev in &ddtl.draws {
                if ev.date < ddtl.availability_start || ev.date > ddtl.availability_end { continue; }
                if let Some(ds) = draw_stop { if ev.date >= ds { continue; } }
                outstanding = outstanding.checked_add(ev.amount)?;
            }
        } else {
            outstanding = outstanding.checked_add(loan.notional_limit)?;
        }

        let dc = loan.day_count;
        let mut prev = dates[0];
        let mut fee_interest_sum = 0.0;
        let mut time_weighted_outstanding = 0.0;

        for &d in dates.iter().skip(1) {
            if d <= as_of { prev = d; continue; }
            let yf = dc.year_fraction(prev, d, DayCountCtx::default())?;

            // Base rate
            let base_rate = match &loan.rate {
                crate::instruments::term_loan::types::RateSpec::Fixed { rate_bp } => (*rate_bp as f64) * 1e-4,
                crate::instruments::term_loan::types::RateSpec::Floating { index_id, floor_bp, .. } => {
                    let fwd = market.get_forward_ref(index_id.as_str())?;
                    let mut r = fwd.rate(yf);
                    if let Some(floor) = floor_bp { r = r.max((*floor as f64) * 1e-4); }
                    r
                }
            };
            // Margin including step-ups
            let margin_bp = loan
                .covenants
                .as_ref()
                .map(|c| c.margin_stepups.iter().filter(|m| m.date <= d).map(|m| m.delta_bp).sum::<i32>())
                .unwrap_or(0)
                + match &loan.rate { crate::instruments::term_loan::types::RateSpec::Floating{ margin_bp, .. } => *margin_bp, _ => 0 };
            let margin = (margin_bp as f64) * 1e-4;

            let rate = base_rate + margin;
            let cash_interest = outstanding.amount() * rate * yf; // ignore PIK for all-in cash cost

            // Fees
            let mut fees = 0.0;
            if let Some(ddtl) = &loan.ddtl {
                // Step-downs
                let mut limit = ddtl.commitment_limit;
                for sd in &ddtl.commitment_step_downs { if sd.date <= d { limit = sd.new_limit; } }
                let undrawn = (limit.amount() - outstanding.amount()).max(0.0);
                if ddtl.commitment_fee_bp != 0 { fees += undrawn * (ddtl.commitment_fee_bp as f64) * 1e-4 * yf; }
                if ddtl.usage_fee_bp != 0 { fees += outstanding.amount() * (ddtl.usage_fee_bp as f64) * 1e-4 * yf; }
            }

            fee_interest_sum += cash_interest + fees;
            time_weighted_outstanding += outstanding.amount() * yf;

            // Apply sweeps/amortization
            if let Some(cov) = &loan.covenants { for s in cov.cash_sweeps.iter().filter(|s| s.date == d) { outstanding = outstanding.checked_sub(s.amount)?; } }

            prev = d;
        }

        if time_weighted_outstanding <= 0.0 { return Ok(0.0); }
        Ok(fee_interest_sum / time_weighted_outstanding)
    }
}


