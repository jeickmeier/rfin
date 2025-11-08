//! Swaption DV01 metric calculator.
//!
//! Provides DV01 calculation for Swaption instruments using rate sensitivity.
//!
//! # Market Standard Formula
//!
//! For swaptions, DV01 represents sensitivity to parallel shifts in interest rates.
//! This is captured by the Rho greek (sensitivity per 1% rate change).
//!
//! DV01 = Rho / 100
//!
//! The calculation uses bump-and-reprice methodology:
//! 1. Price swaption at current discount curve
//! 2. Bump discount curve by 1bp
//! 3. Reprice swaption with bumped curve
//! 4. DV01 = (bumped_price - base_price) × 100 / 100
//!
//! This approach properly accounts for the option's sensitivity through both:
//! - The underlying swap's annuity (PV01 of the swap)
//! - The option's delta (probability of exercise)
//!
//! # Note
//!
//! For swaptions, vega (volatility risk) is typically larger than DV01 (rate risk).

use crate::instruments::swaption::Swaption;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// DV01 calculator for Swaption instruments using bump-and-reprice methodology.
pub struct SwaptionDv01Calculator;

impl MetricCalculator for SwaptionDv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let swaption: &Swaption = context.instrument_as()?;
        let as_of = context.as_of;

        if as_of >= swaption.expiry {
            return Ok(0.0);
        }

        // Market standard: Use bump-and-reprice with 1bp parallel shift
        let disc = context.curves.get_discount(&swaption.discount_curve_id)?;
        let base_price = context.base_value.amount();

        // Get volatility (held constant during rate bump)
        let time_to_expiry = swaption.year_fraction(as_of, swaption.expiry, swaption.day_count)?;
        let vol = if let Some(impl_vol) = swaption.pricing_overrides.implied_volatility {
            impl_vol
        } else {
            let vol_surface = context
                .curves
                .surface_ref(swaption.vol_surface_id.as_str())?;
            vol_surface.value_clamped(time_to_expiry, swaption.strike_rate)
        };

        // Bump discount curve by 1bp
        let bumped_disc = disc.with_parallel_bump(1.0); // 1bp bump

        // Reprice with bumped curve
        let bumped_price = if swaption.sabr_params.is_some() {
            swaption.price_sabr(&bumped_disc, as_of)?.amount()
        } else {
            swaption.price_black(&bumped_disc, vol, as_of)?.amount()
        };

        // DV01 = change in price for 1bp rate increase
        let dv01 = bumped_price - base_price;

        Ok(dv01)
    }
}
