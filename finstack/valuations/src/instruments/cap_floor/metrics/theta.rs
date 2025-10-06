//! Theta calculator for interest rate options.
//!
//! Computes daily theta via a bump-and-reprice approach: reprice the instrument
//! at `as_of + 1 business day` holding market curves and vol surface fixed.

use crate::instruments::cap_floor::InterestRateOption;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::dates::calendar::registry::CalendarRegistry;
use finstack_core::dates::DateExt;
use finstack_core::Result;

/// Theta calculator (daily bump-and-reprice)
pub struct ThetaCalculator;

impl MetricCalculator for ThetaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &InterestRateOption = context.instrument_as()?;

        // Base PV from context
        let base_pv = context.base_value.amount();

        // Advance one business day using calendar if available, else weekday roll
        let as_of_plus_1bd = if let Some(ref cal_id) = option.calendar_id {
            if let Some(cal) = CalendarRegistry::global().resolve_str(cal_id.as_str()) {
                // Manual next business day search with bounded loop
                let mut d = context.as_of.add_weekdays(1);
                let mut searched = 0;
                while !cal.is_business_day(d) && searched < 100 {
                    d = d.add_weekdays(1);
                    searched += 1;
                }
                d
            } else {
                context.as_of.add_weekdays(1)
            }
        } else {
            context.as_of.add_weekdays(1)
        };

        // Reprice at t+1bd with same market context
        let bumped = option.npv(&context.curves, as_of_plus_1bd)?;

        Ok(bumped.amount() - base_pv)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}
