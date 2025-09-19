use crate::instruments::basis_swap::pricing::engine::{BasisEngine, FloatLegParams};
use crate::instruments::basis_swap::types::BasisSwap;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::{Error, Result, F};

/// Par spread (bp) on the primary leg that sets NPV to zero
pub struct ParSpreadCalculator;

impl MetricCalculator for ParSpreadCalculator {
    fn dependencies(&self) -> &[MetricId] { &[MetricId::BasisAnnuityPrimary] }

    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        // Use dependency and drop borrow
        let annuity = context
            .computed
            .get(&MetricId::BasisAnnuityPrimary)
            .copied()
            .unwrap_or(0.0);
        if annuity == 0.0 { return Ok(0.0); }

        let instrument = context.instrument.clone();
        let swap = instrument
            .as_any()
            .downcast_ref::<BasisSwap>()
            .ok_or(Error::Input(finstack_core::error::InputError::Invalid))?;
        let curves = context.curves.clone();
        let as_of = context.as_of;

        // PV of reference leg
        let leg_ref = &swap.reference_leg;
        let schedule_ref = swap.leg_schedule(leg_ref);
        let params_ref = FloatLegParams {
            schedule: &schedule_ref,
            notional: swap.notional,
            disc_id: swap.discount_curve_id.as_str(),
            fwd_id: leg_ref.forward_curve_id.as_str(),
            accrual_dc: leg_ref.day_count,
            spread: leg_ref.spread,
            base_date: swap.start_date,
        };
        let pv_ref = BasisEngine::pv_float_leg(params_ref, curves.as_ref(), as_of)?.amount();

        // PV of primary at zero spread
        let leg = &swap.primary_leg;
        let schedule = swap.leg_schedule(leg);
        let params = FloatLegParams {
            schedule: &schedule,
            notional: swap.notional,
            disc_id: swap.discount_curve_id.as_str(),
            fwd_id: leg.forward_curve_id.as_str(),
            accrual_dc: leg.day_count,
            spread: 0.0,
            base_date: swap.start_date,
        };
        let pv_primary_no_spread = BasisEngine::pv_float_leg(params, curves.as_ref(), as_of)?.amount();

        // Solve for s (decimal). Convert to bp.
        let s_decimal = (pv_ref - pv_primary_no_spread) / (swap.notional.amount() * annuity);
        Ok(s_decimal * 1e4)
    }
}


