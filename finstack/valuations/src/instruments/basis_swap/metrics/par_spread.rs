use crate::instruments::basis_swap::types::BasisSwap;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::{Error, Result, F};

/// Calculator for the par spread on the primary leg that sets NPV to zero.
///
/// The par spread is the spread that would make the basis swap have zero net present value,
/// calculated by solving for the spread that equates the present values of both legs.
///
/// # Examples
/// ```rust
/// use finstack_valuations::instruments::basis_swap::metrics::ParSpreadCalculator;
///
/// let calc = ParSpreadCalculator;
/// ```
pub struct ParSpreadCalculator;

impl MetricCalculator for ParSpreadCalculator {
    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::BasisAnnuityPrimary]
    }

    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        // Use dependency and drop borrow
        let annuity = context
            .computed
            .get(&MetricId::BasisAnnuityPrimary)
            .copied()
            .unwrap_or(0.0);
        if annuity == 0.0 {
            return Ok(0.0);
        }

        let instrument = context.instrument.clone();
        let swap = instrument
            .as_any()
            .downcast_ref::<BasisSwap>()
            .ok_or(Error::Input(finstack_core::error::InputError::Invalid))?;
        let curves = context.curves.clone();
        let as_of = context.as_of;

        // PV of reference leg
        let schedule_ref = swap.leg_schedule(&swap.reference_leg);
        let pv_ref = swap.pv_float_leg(&swap.reference_leg, &schedule_ref, curves.as_ref(), as_of)?.amount();

        // PV of primary at zero spread - need to create a modified leg
        let primary_leg_no_spread = crate::instruments::basis_swap::BasisSwapLeg {
            forward_curve_id: swap.primary_leg.forward_curve_id.clone(),
            frequency: swap.primary_leg.frequency,
            day_count: swap.primary_leg.day_count,
            bdc: swap.primary_leg.bdc,
            spread: 0.0,
        };
        let schedule = swap.leg_schedule(&primary_leg_no_spread);
        let pv_primary_no_spread = swap.pv_float_leg(&primary_leg_no_spread, &schedule, curves.as_ref(), as_of)?.amount();

        // Solve for s (decimal). Convert to bp.
        let s_decimal = (pv_ref - pv_primary_no_spread) / (swap.notional.amount() * annuity);
        Ok(s_decimal * 1e4)
    }
}
