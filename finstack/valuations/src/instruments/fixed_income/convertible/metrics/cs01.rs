//! CS01 calculator for convertible bonds.
//!
//! Convertible bonds are hybrid instruments with both debt and equity
//! components and are typically priced without a separate hazard curve, so
//! this calculator deviates from the [canonical CS01 convention][canonical]
//! (par CDS curve bump). It instead applies a parallel 1 bp shock to the
//! configured **credit curve ID** (resolved against the discount-curve
//! container, which may already embed the credit spread) and uses the same
//! symmetric (central) finite difference as the canonical helpers:
//!
//! ```text
//! CS01 = (PV(s + 1bp) - PV(s - 1bp)) / 2
//! ```
//!
//! When `credit_curve_id` is `None`, credit risk is not modelled
//! independently and CS01 is reported as `0.0` (bumping a generic discount
//! curve would produce rho, not CS01).
//!
//! Sign convention is identical to the canonical reference:
//! - Long convertible → CS01 negative (wider spreads reduce PV).
//! - Short convertible → CS01 positive.
//!
//! [canonical]: crate::metrics::sensitivities::cs01

use crate::instruments::common_impl::traits::Instrument;
use crate::instruments::fixed_income::convertible::ConvertibleBond;
use crate::metrics::bump_discount_curve_parallel;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// CS01 calculator for convertible bonds.
pub(crate) struct Cs01Calculator;

impl MetricCalculator for Cs01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let bond: &ConvertibleBond = context.instrument_as()?;
        let as_of = context.as_of;

        // Check if expired
        if as_of >= bond.maturity {
            return Ok(0.0);
        }

        let bump_bp = 1.0;

        let curve_to_bump = match &bond.credit_curve_id {
            Some(id) => id,
            None => return Ok(0.0),
        };

        let curves_up = bump_discount_curve_parallel(&context.curves, curve_to_bump, bump_bp)?;
        let curves_down = bump_discount_curve_parallel(&context.curves, curve_to_bump, -bump_bp)?;

        let pv_up = bond.value(&curves_up, as_of)?.amount();
        let pv_down = bond.value(&curves_down, as_of)?.amount();

        let cs01 = (pv_up - pv_down) / 2.0;

        Ok(cs01)
    }
}
