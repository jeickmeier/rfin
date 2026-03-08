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
pub struct RhoCalculator;

impl MetricCalculator for RhoCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let cds_option: &CDSOption = context.instrument_as()?;
        let as_of = context.as_of;

        // Check expiry
        let t = cds_option.day_count.year_fraction(
            as_of,
            cds_option.expiry,
            finstack_core::dates::DayCountCtx::default(),
        )?;
        if t <= 0.0 {
            return Ok(0.0);
        }

        // Base PV
        let base_pv = context.base_value.amount();

        // Get the discount curve and bump it
        let disc = context.curves.get_discount(&cds_option.discount_curve_id)?;

        // Bump discount curve by 1bp (parallel shift)
        let bumped_disc = disc.with_parallel_bump(RHO_BUMP_BP)?;

        // Create bumped market context
        let bumped_curves = context.curves.as_ref().clone().insert(bumped_disc);

        // Reprice with bumped curve
        let pv_bumped = cds_option.value(&bumped_curves, as_of)?.amount();

        // Rho = (PV_bumped - PV_base) / bump_size
        // Report per 1bp change
        let rho = (pv_bumped - base_pv) / RHO_BUMP_BP;

        Ok(rho)
    }
}
