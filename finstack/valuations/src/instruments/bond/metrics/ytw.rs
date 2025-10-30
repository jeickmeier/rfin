use crate::cashflow::traits::CashflowProvider;
use crate::instruments::Bond;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::dates::Date;
use finstack_core::money::Money;

/// Calculates yield-to-worst for bonds with call/put schedules.
pub struct YtwCalculator;

impl MetricCalculator for YtwCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        // Build and cache flows and hints if not already present
        let flows = if let Some(ref flows) = context.cashflows {
            flows
        } else {
            let (disc_id, dc, built) = {
                let bond: &Bond = context.instrument_as()?;
                (
                    bond.disc_id.to_owned(),
                    bond.dc,
                    bond.build_schedule(&context.curves, context.as_of)?,
                )
            };
            context.cashflows = Some(built);
            context.discount_curve_id = Some(disc_id);
            context.day_count = Some(dc);
            context.cashflows.as_ref().ok_or_else(|| {
            finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                id: "cashflows".to_string(),
            })
        })?
        };

        // Build candidate exercise dates
        let candidates = {
            let bond: &Bond = context.instrument_as()?;
            let mut candidates: Vec<(Date, Money)> = Vec::new();
            if let Some(cp) = &bond.call_put {
                for c in &cp.calls {
                    if c.date >= context.as_of && c.date <= bond.maturity {
                        let redemption = bond.notional * (c.price_pct_of_par / 100.0);
                        candidates.push((c.date, redemption));
                    }
                }
                for p in &cp.puts {
                    if p.date >= context.as_of && p.date <= bond.maturity {
                        let redemption = bond.notional * (p.price_pct_of_par / 100.0);
                        candidates.push((p.date, redemption));
                    }
                }
            }
            // Always include maturity
            candidates.push((bond.maturity, bond.notional));
            candidates
        };

        // Get current dirty price from PV
        let dirty_now = context.base_value;

        // Find worst yield
        let mut best_ytm = f64::INFINITY;
        for (exercise_date, redemption) in candidates {
            let y = {
                let bond: &Bond = context.instrument_as()?;
                self.solve_ytm_with_exercise(
                    bond,
                    flows,
                    context.as_of,
                    dirty_now,
                    exercise_date,
                    redemption,
                )?
            };

            if y < best_ytm {
                best_ytm = y;
            }
        }

        Ok(best_ytm)
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
    ) -> finstack_core::Result<f64> {
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
                day_count: bond.dc,
                notional: bond.notional,
                coupon_rate: bond.coupon,
                compounding: crate::instruments::bond::pricing::helpers::YieldCompounding::Street,
                frequency: bond.freq,
            },
        )
    }
}
