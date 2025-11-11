use crate::instruments::basis_swap::types::BasisSwap;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::market_data::context::BumpSpec;
use finstack_core::{Error, Result};
use hashbrown::HashMap;

/// Calculator for the DV01 (dollar value of 1 basis point) of a basis swap leg using FD.
///
/// DV01 represents the change in present value for a 1 basis point parallel bump to discount curve.
/// Uses finite-difference: reprice the leg with bumped discount curve.
///
/// See unit tests and `examples/` for usage.
pub struct Dv01Calculator {
    /// Whether this calculator is for the primary leg (true) or reference leg (false).
    pub is_primary: bool,
}

impl Dv01Calculator {
    /// Creates a calculator for the primary leg.
    pub const fn primary() -> Self {
        Self { is_primary: true }
    }

    /// Creates a calculator for the reference leg.
    pub const fn reference() -> Self {
        Self { is_primary: false }
    }
}

impl MetricCalculator for Dv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let instrument = context.instrument.clone();
        let swap = instrument
            .as_any()
            .downcast_ref::<BasisSwap>()
            .ok_or(Error::Input(finstack_core::error::InputError::Invalid))?;
        
        let leg = if self.is_primary {
            &swap.primary_leg
        } else {
            &swap.reference_leg
        };
        let schedule = swap.leg_schedule(leg);
        let curves = context.curves.as_ref();
        let as_of = context.as_of;

        // Base PV of this leg
        let base_pv = swap.pv_float_leg(leg, &schedule, curves, as_of)?;

        // Bump discount curve by 1bp
        let mut bumps = HashMap::new();
        bumps.insert(swap.discount_curve_id.clone(), BumpSpec::parallel_bp(1.0));
        let bumped_ctx = curves.bump(bumps)?;

        // Reprice leg with bumped discount
        let bumped_pv = swap.pv_float_leg(leg, &schedule, &bumped_ctx, as_of)?;

        // DV01 = base - bumped (positive when rates rise causes value to fall)
        let dv01 = base_pv.checked_sub(bumped_pv)?;

        Ok(dv01.amount())
    }
}
