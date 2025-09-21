use crate::cashflow::traits::CashflowProvider;
use crate::instruments::Bond;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::F;

/// Calculates accrued interest for bonds.
///
/// Computes the accrued interest since the last coupon payment up to the
/// valuation date. This is essential for determining the dirty price and
/// other bond metrics that depend on accrued interest.
///
/// See unit tests and `examples/` for usage.
pub struct AccruedInterestCalculator;

impl MetricCalculator for AccruedInterestCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        // Borrow bond once to compute accrued and optionally cache flows/hints
        let (accrued_amt, disc_id, dc, maybe_flows) = {
            let bond: &Bond = context.instrument_as()?;

            // Use context-aware helper (supports FRNs); falls back to fixed/custom path
            let accrued_amt =
                crate::instruments::bond::pricing::helpers::compute_accrued_interest_with_context(
                    bond,
                    &context.curves,
                    context.as_of,
                )?;

            // Prepare potential flows for caching (build now, assign later)
            let maybe_flows = if context.cashflows.is_none() {
                Some(bond.build_schedule(&context.curves, context.as_of)?)
            } else {
                None
            };

            (accrued_amt, bond.disc_id.clone(), bond.dc, maybe_flows)
        };

        // Cache basic context hints for downstream metrics
        context.discount_curve_id = Some(disc_id);
        context.day_count = Some(dc);
        // Also cache full holder cashflows for downstream risk metrics
        if context.cashflows.is_none() {
            if let Some(flows) = maybe_flows {
                context.cashflows = Some(flows);
            }
        }

        Ok(accrued_amt)
    }
}
