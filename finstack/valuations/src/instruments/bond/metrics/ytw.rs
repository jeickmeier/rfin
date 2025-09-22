use crate::cashflow::traits::CashflowProvider;
use crate::instruments::Bond;
use crate::instruments::common::traits::Instrument;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::dates::Date;
use finstack_core::money::Money;
use finstack_core::F;

/// Calculates yield-to-worst for bonds with call/put schedules.
pub struct YtwCalculator;

impl MetricCalculator for YtwCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let bond: &Bond = context.instrument_as()?;
        // Compute dirty price target from quoted clean + accrued or from model PV
        let dirty_price = if let Some(clean) = bond.pricing_overrides.quoted_clean_price {
            let accrued = crate::instruments::bond::pricing::helpers::compute_accrued_interest_with_context(
                bond, &context.curves, context.as_of,
            )?;
            Money::new(clean + accrued, bond.notional.currency())
        } else {
            bond.value(&context.curves, context.as_of)?
        };
        // Use helper to compute price from YTW and then invert via solver path; here reuse helper price_from_ytw
        let flows = bond.build_schedule(&context.curves, context.as_of)?;
        // Scan candidates and pick min yield by solving per exercise date
        let mut best_yield = f64::INFINITY;
        // Generate candidate (exercise_date, redemption) pairs
        let mut candidates: Vec<(Date, Money)> = Vec::new();
        if let Some(cp) = &bond.call_put {
            for c in &cp.calls {
                if c.date >= context.as_of && c.date <= bond.maturity {
                    candidates.push((c.date, bond.notional * (c.price_pct_of_par / 100.0)));
                }
            }
            for p in &cp.puts {
                if p.date >= context.as_of && p.date <= bond.maturity {
                    candidates.push((p.date, bond.notional * (p.price_pct_of_par / 100.0)));
                }
            }
        }
        candidates.push((bond.maturity, bond.notional));
        for (exercise_date, redemption) in candidates {
            let y = self.solve_ytm_with_exercise(
                bond,
                &flows,
                context.as_of,
                dirty_price,
                exercise_date,
                redemption,
            )?;
            if y < best_yield {
                best_yield = y;
            }
        }
        Ok(best_yield)
    }
}

impl YtwCalculator {
    fn solve_ytm_with_exercise(
        &self,
        bond: &Bond,
        flows: &[(Date, Money)],
        as_of: Date,
        target_price: Money,
        exercise_date: Date,
        redemption: Money,
    ) -> finstack_core::Result<F> {
        // Build truncated flows up to exercise plus redemption and reuse solver
        let mut ex_flows: Vec<(Date, Money)> = Vec::with_capacity(flows.len());
        for &(date, amount) in flows {
            if date <= as_of || date > exercise_date {
                continue;
            }
            ex_flows.push((date, amount));
        }
        ex_flows.push((exercise_date, redemption));

        crate::instruments::bond::pricing::ytm_solver::solve_ytm(
            &ex_flows,
            as_of,
            target_price,
            crate::instruments::bond::pricing::ytm_solver::YtmPricingSpec {
                day_count: bond.schedule.dc,
                notional: bond.notional,
                coupon_rate: bond.coupon,
                compounding: crate::instruments::bond::pricing::helpers::YieldCompounding::Street,
                frequency: bond.schedule.freq,
            },
        )
    }
}
