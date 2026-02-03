use crate::metrics::{MetricCalculator, MetricContext};

/// Generic discount factor at effective start/value date.
///
/// Uses `Instrument::market_dependencies()` to find the primary discount curve
/// and `Instrument::effective_start_date()` to determine the start date.
pub struct GenericDfStartCalculator;

impl MetricCalculator for GenericDfStartCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let deps = context.instrument.market_dependencies();
        let discount_id = deps
            .curves
            .discount_curves
            .first()
            .cloned()
            .ok_or_else(|| {
                finstack_core::Error::Validation(format!(
                    "Instrument {} has no discount curve dependencies for DfStart",
                    context.instrument.id()
                ))
            })?;
        let start_date = context.instrument.effective_start_date().ok_or_else(|| {
            finstack_core::Error::Validation(format!(
                "Instrument {} has no value date for DfStart",
                context.instrument.id()
            ))
        })?;

        let disc = context.curves.get_discount(&discount_id)?;
        disc.df_on_date_curve(start_date)
    }
}
