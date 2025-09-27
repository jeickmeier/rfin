use crate::instruments::basis_swap::types::BasisSwap;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::{Error, Result, F};

/// Calculator for the discounted accrual sum (annuity) of a basis swap leg.
///
/// The annuity represents the sum of discounted year fractions for a leg,
/// which is used in DV01 calculations and par spread computations.
///
/// # Examples
/// ```rust
/// use finstack_valuations::instruments::basis_swap::metrics::AnnuityCalculator;
///
/// let primary_calc = AnnuityCalculator::primary();
/// let reference_calc = AnnuityCalculator::reference();
/// ```
pub struct AnnuityCalculator {
    /// Whether this calculator is for the primary leg (true) or reference leg (false).
    pub is_primary: bool,
}

impl AnnuityCalculator {
    /// Creates a calculator for the primary leg.
    pub const fn primary() -> Self {
        Self { is_primary: true }
    }

    /// Creates a calculator for the reference leg.
    pub const fn reference() -> Self {
        Self { is_primary: false }
    }
}

impl MetricCalculator for AnnuityCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let instrument = context.instrument.clone();
        let swap = instrument
            .as_any()
            .downcast_ref::<BasisSwap>()
            .ok_or(Error::Input(finstack_core::error::InputError::Invalid))?;
        let curves = context.curves.clone();

        let leg = if self.is_primary {
            &swap.primary_leg
        } else {
            &swap.reference_leg
        };
        let schedule = swap.leg_schedule(leg);

        // Use the instrument's own annuity method
        swap.annuity_for_leg(leg, &schedule, curves.as_ref())
    }
}
