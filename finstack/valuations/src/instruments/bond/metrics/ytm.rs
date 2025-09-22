use crate::cashflow::traits::CashflowProvider;
use crate::instruments::Bond;
use crate::instruments::common::traits::Instrument;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::money::Money;

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

    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let bond: &Bond = context.instrument_as()?;
        // Use cashflows and ytm_solver to compute YTM from quoted price override if present
        let flows = bond.build_schedule(&context.curves, context.as_of)?;
        let quoted_clean = bond.pricing_overrides.quoted_clean_price
            .map(|p| Money::new(p, bond.notional.currency()));
        let dirty_price = if let Some(clean) = quoted_clean {
            // Add accrued to clean to get dirty
            let accrued = crate::instruments::bond::pricing::helpers::compute_accrued_interest_with_context(
                bond, &context.curves, context.as_of,
            )?;
            Money::new(clean.amount() + accrued, clean.currency())
        } else {
            // Fallback to model PV
            bond.value(&context.curves, context.as_of)?
        };
        let ytm = crate::instruments::bond::pricing::ytm_solver::solve_ytm(
            &flows,
            context.as_of,
            dirty_price,
            crate::instruments::bond::pricing::ytm_solver::YtmPricingSpec {
                day_count: bond.schedule.dc,
                notional: bond.notional,
                coupon_rate: bond.coupon,
                compounding: crate::instruments::bond::pricing::helpers::YieldCompounding::Street,
                frequency: bond.schedule.freq,
            },
        )?;
        Ok(ytm)
    }
}
