//! CS01 calculator for convertible bonds.
//!
//! Computes CS01 (credit spread sensitivity) using finite differences.
//! For convertible bonds, credit spread sensitivity can be measured by
//! bumping the discount curve (which may include credit spread) or a
//! separate credit curve if available.
//!
//! # Note
//!
//! Convertible bonds are hybrid instruments with both debt and equity components.
//! Credit spread sensitivity affects the bond component more than the equity
//! conversion option. If the discount curve includes credit spread, bumping
//! it directly captures CS01. If a separate credit curve exists, it should
//! be used instead.

use crate::instruments::common_impl::traits::Instrument;
use crate::instruments::fixed_income::convertible::ConvertibleBond;
use crate::metrics::bump_discount_curve_parallel;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// CS01 calculator for convertible bonds.
pub struct Cs01Calculator;

impl MetricCalculator for Cs01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let bond: &ConvertibleBond = context.instrument_as()?;
        let as_of = context.as_of;

        // Check if expired
        if as_of >= bond.maturity {
            return Ok(0.0);
        }

        // CS01 measures credit spread sensitivity. This requires a separate credit
        // curve; if none is provided, credit risk is not modeled independently and
        // CS01 is zero (bumping the discount curve would give Rho, not CS01).
        let bump_bp = 0.0001; // 1bp for credit spread

        let curve_to_bump = match &bond.credit_curve_id {
            Some(id) => id,
            None => return Ok(0.0),
        };

        // Central finite difference: bump both up and down for O(h^2) accuracy,
        // consistent with the rho and dividend01 calculators.
        let curves_up = bump_discount_curve_parallel(&context.curves, curve_to_bump, bump_bp)?;
        let curves_down = bump_discount_curve_parallel(&context.curves, curve_to_bump, -bump_bp)?;

        let pv_up = bond.value(&curves_up, as_of)?.amount();
        let pv_down = bond.value(&curves_down, as_of)?.amount();

        // CS01 = (PV_up - PV_down) / 2 per 1bp credit spread move
        let cs01 = (pv_up - pv_down) / 2.0;

        Ok(cs01)
    }
}
