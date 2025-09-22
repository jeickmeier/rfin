use crate::instruments::Bond;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
// F not used here; keep imports minimal

/// Calculates modified duration for bonds.
pub struct ModifiedDurationCalculator;

impl MetricCalculator for ModifiedDurationCalculator {
    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::DurationMac]
    }

    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let bond: &Bond = context.instrument_as()?;
        let ytm = context
            .computed
            .get(&MetricId::Ytm)
            .copied()
            .unwrap_or(0.0);
        let d_mac = context
            .computed
            .get(&MetricId::DurationMac)
            .copied()
            .unwrap_or(0.0);

        // Modified duration depends on compounding; default to Street (periodic with bond freq)
        let m = crate::instruments::bond::pricing::helpers::periods_per_year(bond.schedule.freq)
            .unwrap_or(1.0)
            .max(1.0);
        Ok(d_mac / (1.0 + ytm / m))
    }
}
