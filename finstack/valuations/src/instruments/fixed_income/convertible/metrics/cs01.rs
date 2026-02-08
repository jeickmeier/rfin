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
        let base_pv = context.base_value.amount();

        // Check if expired
        if as_of >= bond.maturity {
            return Ok(0.0);
        }

        // Bump the credit curve if available, otherwise fall back to the discount curve.
        // The credit curve captures the issuer's spread; bumping it gives the true CS01.
        let bump_bp = 0.0001; // 1bp for credit spread

        let curve_to_bump = bond
            .credit_curve_id
            .as_ref()
            .unwrap_or(&bond.discount_curve_id);

        let curves_bumped = bump_discount_curve_parallel(&context.curves, curve_to_bump, bump_bp)?;

        // Reprice with bumped curve
        let pv_bumped = bond.value(&curves_bumped, as_of)?.amount();

        // CS01 = PV_change per 1bp credit spread move
        // Standard convention: CS01 = PV_bumped - PV_base (positive when spread widens increases value for protection buyer)
        let cs01 = pv_bumped - base_pv;

        Ok(cs01)
    }
}
