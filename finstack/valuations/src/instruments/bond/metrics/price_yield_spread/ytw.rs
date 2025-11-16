use crate::cashflow::traits::CashflowProvider;
use crate::instruments::bond::CashflowSpec;
use crate::instruments::Bond;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::dates::Date;
use finstack_core::money::Money;

/// Calculates yield-to-worst for bonds with call/put schedules.
pub struct YtwCalculator;

impl MetricCalculator for YtwCalculator {
    fn dependencies(&self) -> &[MetricId] {
        // YTW is defined off the market price (quoted clean + accrued), so we
        // require Accrued to be computed first to construct the dirty price.
        &[MetricId::Accrued]
    }

    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        // Build and cache flows and hints if not already present
        let flows = if let Some(ref flows) = context.cashflows {
            flows
        } else {
            let (discount_curve_id, dc, built) = {
                let bond: &Bond = context.instrument_as()?;
                (
                    bond.discount_curve_id.to_owned(),
                    bond.cashflow_spec.day_count(),
                    bond.build_schedule(&context.curves, context.as_of)?,
                )
            };
            context.cashflows = Some(built);
            context.discount_curve_id = Some(discount_curve_id);
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
            candidates.push((
                bond.maturity,
                // Redemption at maturity is already included in the cashflow schedule,
                // so we do not add an extra principal flow here to avoid double-counting.
                Money::new(0.0, bond.notional.currency()),
            ));
            candidates
        };

        // Construct current dirty market price from quoted clean price + accrued interest.
        //
        // This mirrors the YTM and DirtyPrice calculators so that YTW is
        // defined relative to the same market price, not the model PV.
        let bond: &Bond = context.instrument_as()?;
        let clean_px = bond
            .pricing_overrides
            .quoted_clean_price
            .ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                    id: "bond.pricing_overrides.quoted_clean_price".to_string(),
                })
            })?;

        // Get accrued from computed metrics (dependency ensures this is present).
        let accrued = context
            .computed
            .get(&MetricId::Accrued)
            .copied()
            .ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                    id: "metric:Accrued".to_string(),
                })
            })?;

        // Dirty price in currency: quoted clean is % of par.
        let dirty_amt = (clean_px * bond.notional.amount() / 100.0) + accrued;
        let dirty_now = Money::new(dirty_amt, bond.notional.currency());

        // Find worst yield
        let mut best_ytm = f64::INFINITY;
        for (exercise_date, redemption) in candidates {
            let y = self.solve_ytm_with_exercise(
                bond,
                flows,
                context.as_of,
                dirty_now,
                exercise_date,
                redemption,
            )?;

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
                day_count: bond.cashflow_spec.day_count(),
                notional: bond.notional,
                coupon_rate: match &bond.cashflow_spec {
                    CashflowSpec::Fixed(spec) => spec.rate,
                    _ => 0.0,
                },
                compounding: crate::instruments::bond::pricing::helpers::YieldCompounding::Street,
                frequency: bond.cashflow_spec.frequency(),
            },
        )
    }
}
