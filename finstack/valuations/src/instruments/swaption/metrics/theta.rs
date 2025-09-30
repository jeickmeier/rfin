//! Theta calculator for swaptions (daily bump-in-time).
//!
//! Computes daily theta via bump-and-reprice at `as_of + 1 business day`,
//! holding market curves and vol surface fixed, aligning with cap/floor theta.

use crate::instruments::swaption::Swaption;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::dates::calendar::registry::CalendarRegistry;
use finstack_core::dates::DateExt;
use finstack_core::prelude::Result;

/// Theta calculator for swaptions (daily)
pub struct ThetaCalculator;

impl MetricCalculator for ThetaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &Swaption = context.instrument_as()?;

        // Base PV from context
        let base_pv = context.base_value.amount();

        // Advance one business day: use calendar if instrument attributes carry one; otherwise weekday roll
        let as_of_plus_1bd = if let Some(cal_id) = option
            .attributes
            .get_meta("calendar")
            .or_else(|| option.attributes.get_meta("calendar_id"))
        {
            if let Some(cal) = CalendarRegistry::global().resolve_str(cal_id) {
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

        // Reprice at t+1bd with same market context and chosen model path
        let disc = context.curves.get_discount_ref(option.disc_id.as_ref())?;
        let bumped = if option.sabr_params.is_some() {
            option.price_sabr(disc, as_of_plus_1bd)?
        } else {
            // Hold vol surface value constant by re-fetching at bumped as_of; methodology choice: use same vol surface with bumped time
            let t = option.year_fraction(as_of_plus_1bd, option.expiry, option.day_count)?;
            if t <= 0.0 {
                return Ok(0.0);
            }
            let vol = if let Some(impl_vol) = option.pricing_overrides.implied_volatility {
                impl_vol
            } else {
                context
                    .curves
                    .surface_ref(option.vol_id)?
                    .value_clamped(t, option.strike_rate)
            };
            option.price_black(disc, vol, as_of_plus_1bd)?
        };

        Ok(bumped.amount() - base_pv)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}
