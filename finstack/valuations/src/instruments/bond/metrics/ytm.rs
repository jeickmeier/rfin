use crate::cashflow::traits::CashflowProvider;
use crate::instruments::Bond;
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
        // Extract fields we need from the bond
        let (clean_px, notional, dc, disc_id, coupon, freq) = {
            let bond: &Bond = context.instrument_as()?;
            (
                bond.pricing_overrides.quoted_clean_price.ok_or_else(|| {
                    finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                        id: "bond.pricing_overrides.quoted_clean_price".to_string(),
                    })
                })?,
                bond.notional,
                bond.dc,
                bond.disc_id.clone(),
                bond.coupon,
                bond.freq,
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

        // Compute dirty price in currency: clean is quoted % of par
        let dirty_amt = (clean_px * notional.amount() / 100.0) + ai;
        let dirty = Money::new(dirty_amt, notional.currency());

        // Build and cache flows and hints if not already present
        if context.cashflows.is_none() {
            let bond: &Bond = context.instrument_as()?;
            let flows = bond.build_schedule(&context.curves, context.as_of)?;
            context.cashflows = Some(flows);
            context.discount_curve_id = Some(disc_id);
            context.day_count = Some(dc);
        }
        let flows = context.cashflows.as_ref().unwrap();

        // Solve for YTM using shared solver with Street compounding (default)
        let ytm = crate::instruments::bond::pricing::ytm_solver::solve_ytm(
            flows,
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
