//! Rho metric for `CDSOption`.
//!
//! Rho measures the sensitivity of the CDS option value to changes in interest rates.
//! This implementation uses finite differences with a bumped discount curve for accuracy.
//!
//! # Market Standard
//!
//! For CDS options, rho captures rate sensitivity through:
//! - Discount factor to expiry (direct Black formula term)
//! - Risky annuity discounting (indirect through RPV01)
//!
//! The finite-difference approach captures both effects correctly.

use crate::instruments::common_impl::traits::Instrument;
use crate::instruments::credit_derivatives::cds_option::CDSOption;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Standard rate bump: 1 basis point.
const RHO_BUMP_BP: f64 = 1.0;

/// Rho calculator for credit options on CDS spreads using finite differences.
///
/// Computes rho by bumping the discount curve by 1bp and repricing.
/// Reports the dollar value change per 1bp change in rates.
pub(crate) struct RhoCalculator;

impl MetricCalculator for RhoCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let cds_option: &CDSOption = context.instrument_as()?;
        let as_of = context.as_of;

        // Check expiry
        let t = cds_option.time_to_expiry(as_of)?;
        if t <= 0.0 {
            return Ok(0.0);
        }

        let disc = context.curves.get_discount(&cds_option.discount_curve_id)?;

        let bumped_up = disc.with_parallel_bump(RHO_BUMP_BP)?;
        let bumped_down = disc.with_parallel_bump(-RHO_BUMP_BP)?;

        let curves_up = context.curves.as_ref().clone().insert(bumped_up);
        let curves_down = context.curves.as_ref().clone().insert(bumped_down);

        let pv_up = cds_option.value(&curves_up, as_of)?.amount();
        let pv_down = cds_option.value(&curves_down, as_of)?.amount();

        Ok((pv_up - pv_down) / (2.0 * RHO_BUMP_BP))
    }
}
