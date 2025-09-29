//! Gamma calculator for swaptions.
//!
//! Computes cash gamma using Black model gamma with forward swap rate and
//! underlying swap annuity. Uses SABR-implied vol if parameters are set,
//! otherwise uses the volatility surface or an override from `PricingOverrides`.

use crate::instruments::swaption::Swaption;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::prelude::Result;


/// Gamma calculator for swaptions
pub struct GammaCalculator;

impl MetricCalculator for GammaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &Swaption = context.instrument_as()?;
        let disc = context.curves.get_discount_ref(option.disc_id.as_ref())?;
        let t = option.year_fraction(context.as_of, option.expiry, option.day_count)?;

        if t <= 0.0 {
            return Ok(0.0);
        }

        let forward = option.forward_swap_rate(disc, context.as_of)?;
        let annuity = option.swap_annuity(disc, context.as_of)?;

        let sigma = if let Some(sabr) = &option.sabr_params {
            let model = crate::instruments::common::models::SABRModel::new(sabr.clone());
            model.implied_volatility(forward, option.strike_rate, t)?
        } else if let Some(impl_vol) = option.pricing_overrides.implied_volatility {
            impl_vol
        } else {
            context
                .curves
                .surface_ref(option.vol_id)?
                .value_clamped(t, option.strike_rate)
        };

        if sigma <= 0.0 || forward <= 0.0 {
            return Ok(0.0);
        }

        let variance = sigma * sigma * t;
        let d1 = ((forward / option.strike_rate).ln() + 0.5 * variance) / variance.sqrt();
        let gamma = finstack_core::math::norm_pdf(d1) / (forward * sigma * t.sqrt());

        // Scale by notional and annuity for cash gamma
        Ok(gamma * option.notional.amount() * annuity)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}
