use crate::cashflow::traits::CashflowProvider;
use crate::instruments::Bond;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::dates::Date;
use finstack_core::money::Money;
use finstack_core::F;

/// Calculates yield to maturity for bonds.
///
/// Computes the internal rate of return that equates the present value of
/// all future cashflows to the current market price. This is a fundamental
/// metric for bond valuation and comparison across different bonds.
///
/// # Dependencies
/// Requires `Accrued` metric to be computed first.
///
/// See unit tests and `examples/` for usage.
pub struct YtmCalculator;

impl MetricCalculator for YtmCalculator {
    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::Accrued]
    }

    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        // Extract fields we need from the bond without cloning it
        let (clean_px, currency, dc, disc_id, notional, coupon, freq, built_flows) = {
            let bond: &Bond = context.instrument_as()?;

            let built_flows = if context.cashflows.is_none() {
                Some(bond.build_schedule(&context.curves, context.as_of)?)
            } else {
                None
            };

            (
                bond.pricing_overrides.quoted_clean_price.ok_or_else(|| {
                    finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                        id: "bond.pricing_overrides.quoted_clean_price".to_string(),
                    })
                })?,
                bond.notional.currency(),
                bond.dc,
                bond.disc_id.clone(),
                bond.notional,
                bond.coupon,
                bond.freq,
                built_flows,
            )
        };

        // Get accrued from computed metrics
        let ai = context
            .computed
            .get(&MetricId::Accrued)
            .copied()
            .ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                    id: "metric:Accrued".to_string(),
                })
            })?;

        // Compute dirty price
        let dirty_amt = clean_px + ai;
        let dirty = Money::new(dirty_amt, currency);

        // Cache flows and hints if we built them
        if context.cashflows.is_none() {
            if let Some(flows) = &built_flows {
                context.cashflows = Some(flows.clone());
            }
            context.discount_curve_id = Some(disc_id.clone());
            context.day_count = Some(dc);
        }

        let flows: Vec<(Date, Money)> = if let Some(f) = &context.cashflows {
            f.clone()
        } else {
            // Should not happen, but fallback to empty
            built_flows.unwrap_or_default()
        };

        // Solve for YTM using shared solver with Street compounding (default)
        let ytm = crate::instruments::bond::pricing::ytm_solver::solve_ytm(
            &flows,
            context.as_of,
            dirty,
            crate::instruments::bond::pricing::ytm_solver::YtmPricingSpec {
                day_count: dc,
                notional,
                coupon_rate: coupon,
                compounding: crate::instruments::bond::pricing::helpers::YieldCompounding::Street,
                frequency: freq,
            },
        )?;

        Ok(ytm)
    }
}


