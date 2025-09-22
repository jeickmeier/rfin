use crate::instruments::Bond;
use crate::metrics::{MetricCalculator, MetricContext};

/// Calculates accrued interest for bonds.
///
/// Computes the accrued interest since the last coupon payment up to the
/// valuation date. This is essential for determining the dirty price and
/// other bond metrics that depend on accrued interest.
///
/// See unit tests and `examples/` for usage.
pub struct AccruedInterestCalculator;

impl MetricCalculator for AccruedInterestCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let bond: &Bond = context.instrument_as()?;
        let accrued = crate::instruments::bond::pricing::helpers::compute_accrued_interest_with_context(
            bond,
            &context.curves,
            context.as_of,
        )?;
        Ok(accrued)
    }
}
