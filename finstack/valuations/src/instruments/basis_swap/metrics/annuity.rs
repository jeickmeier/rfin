use crate::instruments::basis_swap::pricing::engine::BasisEngine;
use crate::instruments::basis_swap::types::BasisSwap;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::{Error, Result, F};

/// Discounted accrual sum for a leg (no notional multiplier)
pub struct AnnuityCalculator {
    pub is_primary: bool,
}

impl AnnuityCalculator {
    pub const fn primary() -> Self { Self { is_primary: true } }
    pub const fn reference() -> Self { Self { is_primary: false } }
}

impl MetricCalculator for AnnuityCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let instrument = context.instrument.clone();
        let swap = instrument
            .as_any()
            .downcast_ref::<BasisSwap>()
            .ok_or(Error::Input(finstack_core::error::InputError::Invalid))?;
        let curves = context.curves.clone();

        let leg = if self.is_primary { &swap.primary_leg } else { &swap.reference_leg };
        let schedule = swap.leg_schedule(leg);
        BasisEngine::annuity_for_leg(&schedule, leg.day_count, swap.discount_curve_id.as_str(), curves.as_ref())
    }
}


