use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::dates::DayCountCtx;

/// Generic year fraction between value date and expiry.
///
/// Uses the primary discount curve day count as the convention for computing
/// the year fraction. This avoids instrument-specific downcasts and provides
/// a consistent convention aligned with the discounting curve.
#[allow(dead_code)]
pub struct GenericYearFractionCalculator;

impl MetricCalculator for GenericYearFractionCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let deps = context.instrument.market_dependencies();
        let discount_id = deps
            .curves
            .discount_curves
            .first()
            .cloned()
            .ok_or_else(|| {
                finstack_core::Error::Validation(format!(
                    "Instrument {} has no discount curve dependencies for YearFraction",
                    context.instrument.id()
                ))
            })?;
        let start_date = context.instrument.effective_start_date().ok_or_else(|| {
            finstack_core::Error::Validation(format!(
                "Instrument {} has no value date for YearFraction",
                context.instrument.id()
            ))
        })?;
        let end_date = context.instrument.expiry().ok_or_else(|| {
            finstack_core::Error::Validation(format!(
                "Instrument {} has no expiry date for YearFraction",
                context.instrument.id()
            ))
        })?;

        if end_date <= start_date {
            return Err(finstack_core::Error::Validation(format!(
                "YearFraction: end date ({}) must be after start date ({})",
                end_date, start_date
            )));
        }

        let disc = context.curves.get_discount(&discount_id)?;
        disc.day_count()
            .year_fraction(start_date, end_date, DayCountCtx::default())
    }
}
