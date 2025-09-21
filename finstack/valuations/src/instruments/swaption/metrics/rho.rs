//! Rho calculator for swaptions (per 1%).
//!
//! Computes sensitivity to a parallel rate shift on the discount curve. Uses
//! the configured model (SABR or Black) consistently with instrument pricing.

use crate::instruments::swaption::Swaption;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::prelude::Result;
use finstack_core::F;

/// Rho calculator for swaptions (per 1%)
pub struct RhoCalculator;

impl MetricCalculator for RhoCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let option: &Swaption = context.instrument_as()?;
        let disc = context
            .curves
            .get::<finstack_core::market_data::term_structures::discount_curve::DiscountCurve>(
            option.disc_id,
        )?;

        // Base price from context
        let base_price = context.base_value.amount();

        // Get volatility from surface using as_of (vol held constant during bump)
        let pricer = crate::instruments::swaption::pricing::SwaptionPricer;
        let time_to_expiry =
            pricer.year_fraction(context.as_of, option.expiry, option.day_count)?;
        let vol = if let Some(impl_vol) = option.pricing_overrides.implied_volatility {
            impl_vol
        } else {
            let vol_surface = context.curves.surface_ref(option.vol_id)?;
            vol_surface.value_clamped(time_to_expiry, option.strike_rate)
        };

        // Create bumped discount curve (+1bp) by modifying knots directly
        let bumped_disc = disc.with_parallel_bump(super::config::DISC_BUMP_BP);

        // Reprice with bumped curve using same model path as instrument pricing
        let bumped_price = if option.sabr_params.is_some() {
            pricer
                .price_sabr(option, &bumped_disc, context.as_of)?
                .amount()
        } else {
            pricer
                .price_black(option, &bumped_disc, vol, context.as_of)?
                .amount()
        };

        // Rho per 1% = (PV_bumped_1bp - PV_base) * 100
        // This gives sensitivity to a 100bp (1%) parallel shift
        let rho_1bp = bumped_price - base_price;
        Ok(rho_1bp * super::config::RHO_PCT_PER_BP)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}
