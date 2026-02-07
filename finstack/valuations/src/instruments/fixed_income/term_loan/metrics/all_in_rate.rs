//! All-in rate metric for term loans.
//!
//! Computes the effective annualized borrower **cash cost** including cash interest
//! and periodic fees, divided by time-weighted outstanding principal.
//!
//! This metric uses the corrected outstanding path from the full cashflow schedule,
//! accounting for DDTL draw timing, amortization, and PIK capitalization.

use crate::instruments::TermLoan;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::dates::DayCountCtx;
use finstack_core::money::Money;
use rust_decimal::prelude::ToPrimitive;

/// All-in rate calculator for term loans.
///
/// Returns the cash-cost all-in rate: (cash interest + fees) / time-weighted outstanding.
/// PIK interest is excluded from the numerator (cash cost only) but affects outstanding.
pub struct AllInRateCalculator;

impl MetricCalculator for AllInRateCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let loan: &TermLoan = context.instrument_as()?;
        let market = &context.curves;
        let as_of = context.as_of;

        // Build full cashflow schedule to get outstanding path
        let schedule = crate::instruments::fixed_income::term_loan::cashflows::generate_cashflows(
            loan, market, as_of,
        )?;

        // Get outstanding path including notional draws/repays, amortization, and PIK
        let out_path = schedule.outstanding_by_date()?;

        // Helper to look up outstanding at a given date (piecewise-constant: last value <= target)
        let outstanding_at = |target: finstack_core::dates::Date| -> finstack_core::Result<Money> {
            let mut last = Money::new(0.0, loan.currency);
            for (d, amt) in &out_path {
                if *d <= target {
                    last = *amt;
                } else {
                    break;
                }
            }
            Ok(last)
        };

        // Build coupon dates
        let mut schedule_builder =
            finstack_core::dates::ScheduleBuilder::new(loan.issue, loan.maturity)?
                .frequency(loan.pay_freq)
                .stub_rule(loan.stub);
        if let Some(ref cal_id) = loan.calendar_id {
            schedule_builder = schedule_builder.adjust_with_id(loan.bdc, cal_id);
        }
        let sched = schedule_builder.build()?;
        let mut dates: Vec<finstack_core::dates::Date> = sched.into_iter().collect();
        if dates.first().copied() != Some(loan.issue) {
            dates.insert(0, loan.issue);
        }

        let dc = loan.day_count;
        let mut prev = dates[0];
        let mut fee_interest_sum = 0.0;
        let mut time_weighted_outstanding = 0.0;

        for &d in dates.iter().skip(1) {
            if d <= as_of {
                prev = d;
                continue;
            }
            let yf = dc.year_fraction(prev, d, DayCountCtx::default())?;

            // Outstanding at start of period
            let outstanding = outstanding_at(prev)?;

            // Compute period rate with centralized projection
            let rate = match &loan.rate {
                crate::instruments::fixed_income::term_loan::types::RateSpec::Fixed { rate_bp } => {
                    f64::from(*rate_bp) * 1e-4
                }
                crate::instruments::fixed_income::term_loan::types::RateSpec::Floating(spec) => {
                    // Use shared margin helper (includes base spread + covenant step-ups + overrides)
                    let total_spread =
                        crate::instruments::fixed_income::term_loan::cashflows::margin_bp_at(
                            loan, d,
                        );

                    // Use actual coupon date as period end for forward rate lookup
                    // (avoids approximation error from `yf * 365.25`)
                    let period_end = d;

                    let params = crate::cashflow::builder::FloatingRateParams::with_full(
                        total_spread,
                        spec.gearing.to_f64().unwrap_or(1.0),
                        spec.floor_bp.and_then(|d| d.to_f64()),
                        spec.cap_bp.and_then(|d| d.to_f64()),
                    );
                    crate::cashflow::builder::project_floating_rate_from_market(
                        prev,
                        period_end,
                        spec.index_id.as_str(),
                        &params,
                        market,
                    )?
                }
            };
            let cash_interest = outstanding.amount() * rate * yf; // ignore PIK for all-in cash cost

            // Fees
            let mut fees = 0.0;
            if let Some(ddtl) = &loan.ddtl {
                // Step-downs
                let mut limit = ddtl.commitment_limit;
                for sd in &ddtl.commitment_step_downs {
                    if sd.date <= d {
                        limit = sd.new_limit;
                    }
                }

                // Calculate cumulative draws to match cashflows logic
                let mut cumulative_drawn_amt = 0.0;
                // Draw stop logic also applies
                let cov_stop = loan
                    .covenants
                    .as_ref()
                    .and_then(|c| c.draw_stop_dates.iter().min().copied());
                let override_stop = loan
                    .pricing_overrides
                    .term_loan
                    .as_ref()
                    .and_then(|ov| ov.draw_stop_date);
                let draw_stop = match (cov_stop, override_stop) {
                    (Some(a), Some(b)) => Some(a.min(b)),
                    (Some(a), None) => Some(a),
                    (None, Some(b)) => Some(b),
                    (None, None) => None,
                };

                for ev in &ddtl.draws {
                    if ev.date < ddtl.availability_start || ev.date > ddtl.availability_end {
                        continue;
                    }
                    if let Some(ds) = draw_stop {
                        if ev.date >= ds {
                            continue;
                        }
                    }
                    if ev.date <= d {
                        cumulative_drawn_amt += ev.amount.amount();
                    }
                }

                // Use same fee base logic as cashflow engine
                let undrawn = match ddtl.fee_base {
                    crate::instruments::fixed_income::term_loan::spec::CommitmentFeeBase::Undrawn => {
                        // Term Loan Standard
                        (limit.amount() - cumulative_drawn_amt).max(0.0)
                    }
                    crate::instruments::fixed_income::term_loan::spec::CommitmentFeeBase::CommitmentMinusOutstanding => {
                        // Revolver Standard
                        (limit.amount() - outstanding.amount()).max(0.0)
                    }
                };
                if ddtl.commitment_fee_bp != 0 {
                    fees += undrawn * f64::from(ddtl.commitment_fee_bp) * 1e-4 * yf;
                }
                if ddtl.usage_fee_bp != 0 {
                    fees += outstanding.amount() * f64::from(ddtl.usage_fee_bp) * 1e-4 * yf;
                }
            }

            fee_interest_sum += cash_interest + fees;
            time_weighted_outstanding += outstanding.amount() * yf;

            prev = d;
        }

        if time_weighted_outstanding <= 0.0 {
            return Ok(0.0);
        }
        Ok(fee_interest_sum / time_weighted_outstanding)
    }
}
