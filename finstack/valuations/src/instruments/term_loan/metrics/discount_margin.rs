//! Discount Margin for floating-rate term loans.
//!
//! Solves for an additive spread (decimal) to the projected index such that
//! discounted PV matches observed price (or base PV if no quote provided).

use crate::instruments::TermLoan;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::dates::DayCountCtx;
use finstack_core::math::solver::{BrentSolver, Solver};
use finstack_core::money::Money;

pub struct DiscountMarginCalculator;

impl DiscountMarginCalculator {
    fn pv_given_dm(
        loan: &TermLoan,
        curves: &finstack_core::market_data::MarketContext,
        as_of: finstack_core::dates::Date,
        dm: f64,
    ) -> finstack_core::Result<f64> {
        // Recreate a simplified cashflow accrual over coupon periods but add dm to base rate
        let disc = curves.get_discount_ref(loan.discount_curve_id.as_str())?;
        let disc_dc = disc.day_count();

        let sched = finstack_core::dates::ScheduleBuilder::new(loan.issue, loan.maturity)
            .frequency(loan.pay_freq)
            .stub_rule(loan.stub)
            .build()?;
        let mut dates: Vec<finstack_core::dates::Date> = sched.into_iter().collect();
        if dates.first().copied() != Some(loan.issue) {
            dates.insert(0, loan.issue);
        }

        let mut outstanding = Money::new(0.0, loan.currency);
        if let Some(ddtl) = &loan.ddtl {
            for ev in &ddtl.draws {
                if ev.date >= loan.issue && ev.date <= loan.maturity {
                    outstanding = outstanding.checked_add(ev.amount)?;
                }
            }
        } else {
            outstanding = outstanding.checked_add(loan.notional_limit)?;
        }

        let mut pv = 0.0;
        let mut prev = dates[0];
        for &d in dates.iter().skip(1) {
            if d <= as_of {
                prev = d;
                continue;
            }
            let yf = loan
                .day_count
                .year_fraction(prev, d, DayCountCtx::default())?;
            let rate = match &loan.rate {
                crate::instruments::term_loan::types::RateSpec::Fixed { rate_bp } => {
                    (*rate_bp as f64) * 1e-4
                }
                crate::instruments::term_loan::types::RateSpec::Floating(spec) => {
                    // For discount margin calculation, we add DM to the forward rate
                    // Then apply spread on top
                    let fwd = curves.get_forward_ref(spec.index_id.as_str())?;
                    let mut index_with_dm = fwd.rate(yf) + dm;

                    // Apply floor to index+DM
                    if let Some(floor) = spec.floor_bp {
                        index_with_dm = index_with_dm.max(floor * 1e-4);
                    }

                    // Add spread
                    let mut all_in = (index_with_dm + spec.spread_bp * 1e-4) * spec.gearing;

                    // Apply cap
                    if let Some(cap) = spec.cap_bp {
                        all_in = all_in.min(cap * 1e-4);
                    }

                    all_in
                }
            };
            let interest = outstanding.amount() * rate * yf;

            // Discount to as_of
            let t = disc_dc.year_fraction(disc.base_date(), d, DayCountCtx::default())?;
            let df_abs = disc.df(t);
            let t_asof = disc_dc.year_fraction(disc.base_date(), as_of, DayCountCtx::default())?;
            let df = if t_asof != 0.0 {
                df_abs / disc.df(t_asof)
            } else {
                1.0
            };
            pv += interest * df;

            prev = d;
        }
        Ok(pv)
    }
}

impl MetricCalculator for DiscountMarginCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let loan: &TermLoan = context.instrument_as()?;
        // If not floating, DM = 0.0
        if let crate::instruments::term_loan::types::RateSpec::Fixed { .. } = loan.rate {
            return Ok(0.0);
        }

        // Target price: quoted_clean_price% of par if set, else base PV
        let target = if let Some(px) = loan.pricing_overrides.quoted_clean_price {
            // Interpreting as % of notional_limit
            px * loan.notional_limit.amount() / 100.0
        } else {
            context.base_value.amount()
        };

        let objective = |dm: f64| -> f64 {
            match Self::pv_given_dm(loan, &context.curves, context.as_of, dm) {
                Ok(pv) => pv - target,
                Err(_) => 1e12 * dm.signum(),
            }
        };

        let solver = BrentSolver::new()
            .with_tolerance(1e-12)
            .with_initial_bracket_size(Some(0.05));
        solver.solve(objective, 0.0)
    }
}
