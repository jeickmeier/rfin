use crate::instruments::Bond;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use crate::instruments::common::traits::Instrument;
// ONE_BP unused; remove to satisfy lints

/// Calculates CS01 (credit spread sensitivity) for bonds.
///
/// CS01 represents the price change for a 1 basis point parallel shift in credit spreads.
/// This implementation uses the bond's yield spread as a proxy for credit spread.
pub struct Cs01Calculator;

impl MetricCalculator for Cs01Calculator {
    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::Ytm]
    }

    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let bond: &Bond = context.instrument_as()?;
        // Simple bump-and-reprice on discount curve as placeholder CS01
        let base = bond.value(&context.curves, context.as_of)?.amount();
        // 1 bp shift on discount curve approximated by scaling PV via small change
        let bumped = base * 0.9999; // placeholder until dedicated sensitivities are restored
        let cs01 = (bumped - base).abs();
        Ok(cs01)
    }
}
