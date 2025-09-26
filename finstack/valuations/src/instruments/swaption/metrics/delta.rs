//! Delta calculator for swaptions.
//!
//! Computes cash delta using Black model greeks with forward swap rate and
//! underlying swap annuity. Uses SABR-implied vol if parameters are set,
//! otherwise uses the volatility surface or an override from `PricingOverrides`.

use crate::instruments::common::parameters::OptionType;
use crate::instruments::swaption::Swaption;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::prelude::Result;
use finstack_core::F;

/// Delta calculator for swaptions
pub struct DeltaCalculator;

impl MetricCalculator for DeltaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let option: &Swaption = context.instrument_as()?;
        let disc = context
            .curves
            .get_discount_ref(
                option.disc_id.as_ref(),
            )?;
        let pricer = crate::instruments::swaption::pricing::SwaptionPricer;
        let t = pricer.year_fraction(context.as_of, option.expiry, option.day_count)?;

        if t <= 0.0 {
            return Ok(0.0);
        }

        let forward = pricer.forward_swap_rate(option, disc, context.as_of)?;
        let annuity = pricer.swap_annuity(option, disc, context.as_of)?;

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

        let variance = sigma * sigma * t;
        let d1 = if variance > 0.0 {
            ((forward / option.strike_rate).ln() + 0.5 * variance) / variance.sqrt()
        } else {
            0.0
        };

        let delta = match option.option_type {
            OptionType::Call => finstack_core::math::norm_cdf(d1),
            OptionType::Put => -finstack_core::math::norm_cdf(-d1),
        };

        // Scale by notional and annuity for cash delta
        Ok(delta * option.notional.amount() * annuity)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}
