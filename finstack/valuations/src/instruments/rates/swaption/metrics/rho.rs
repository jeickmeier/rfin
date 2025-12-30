//! Rho calculator for swaptions (per 1bp).
//!
//! Computes sensitivity to a parallel rate shift on the discount curve. Uses
//! the configured model (SABR or Black) consistently with instrument pricing.

use crate::instruments::pricing_overrides::VolSurfaceExtrapolation;
use crate::instruments::swaption::Swaption;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::Result;

/// Rho calculator for swaptions (per 1bp)
pub struct RhoCalculator;

impl MetricCalculator for RhoCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &Swaption = context.instrument_as()?;
        let disc = context.curves.get_discount(&option.discount_curve_id)?;

        // Base price from context
        let base_price = context.base_value.amount();

        // Get volatility from surface using as_of (vol held constant during bump)
        let time_to_expiry =
            option.year_fraction(context.as_of, option.expiry, option.day_count)?;
        let vol = if let Some(impl_vol) = option.pricing_overrides.implied_volatility {
            impl_vol
        } else {
            let vol_surface = context.curves.surface(option.vol_surface_id.as_str())?;
            match option.pricing_overrides.vol_surface_extrapolation {
                VolSurfaceExtrapolation::Clamp | VolSurfaceExtrapolation::LinearInVariance => {
                    // LinearInVariance falls back to Clamp until surface impl is ready
                    vol_surface.value_clamped(time_to_expiry, option.strike_rate)
                }
                VolSurfaceExtrapolation::Error => {
                    vol_surface.value_checked(time_to_expiry, option.strike_rate)?
                }
            }
        };

        // Create bumped discount curve (+1bp) by modifying knots directly
        let bump_bp = option
            .pricing_overrides
            .rho_bump_decimal
            .unwrap_or(super::config::DISC_BUMP_BP);
        let bumped_disc = disc.try_with_parallel_bump(bump_bp)?;

        // Reprice with bumped curve using same model path as instrument pricing
        let bumped_price = if option.sabr_params.is_some() {
            option.price_sabr(&bumped_disc, context.as_of)?.amount()
        } else {
            option
                .price_black(&bumped_disc, vol, context.as_of)?
                .amount()
        };

        // Rho per 1bp = PV(rate + 1bp) − PV(base)
        let rho_1bp = bumped_price - base_price;
        Ok(rho_1bp)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}
