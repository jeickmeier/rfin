use crate::instruments::basis_swap::pricing::engine::{BasisEngine, FloatLegParams};
use crate::instruments::basis_swap::types::BasisSwap;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::{Error, Result, F};

/// Calculator for the present value of a basis swap leg.
///
/// This calculator computes the present value of either the primary or reference
/// leg of a basis swap, including the spread and forward rate components.
///
/// # Examples
/// ```rust
/// use finstack_valuations::instruments::basis_swap::metrics::PvCalculator;
///
/// let primary_calc = PvCalculator::primary();
/// let reference_calc = PvCalculator::reference();
/// ```
pub struct PvCalculator {
    /// Whether this calculator is for the primary leg (true) or reference leg (false).
    pub is_primary: bool,
}

impl PvCalculator {
    /// Creates a calculator for the primary leg.
    pub const fn primary() -> Self { Self { is_primary: true } }
    
    /// Creates a calculator for the reference leg.
    pub const fn reference() -> Self { Self { is_primary: false } }
}

impl MetricCalculator for PvCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let instrument = context.instrument.clone();
        let swap = instrument
            .as_any()
            .downcast_ref::<BasisSwap>()
            .ok_or(Error::Input(finstack_core::error::InputError::Invalid))?;
        let curves = context.curves.clone();
        let as_of = context.as_of;

        let leg = if self.is_primary { &swap.primary_leg } else { &swap.reference_leg };
        let schedule = swap.leg_schedule(leg);
        let params = FloatLegParams {
            schedule: &schedule,
            notional: swap.notional,
            disc_id: swap.discount_curve_id.as_str(),
            fwd_id: leg.forward_curve_id.as_str(),
            accrual_dc: leg.day_count,
            spread: leg.spread,
            base_date: swap.start_date,
        };
        let pv = BasisEngine::pv_float_leg(params, curves.as_ref(), as_of)?;
        Ok(pv.amount())
    }
}


