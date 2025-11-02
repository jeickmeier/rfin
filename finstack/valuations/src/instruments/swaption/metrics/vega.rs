//! Vega calculator for swaptions.
//!
//! Computes cash vega using Black model vega with forward swap rate and
//! underlying swap annuity. Uses SABR-implied vol if parameters are set,
//! otherwise uses the volatility surface or an override from `PricingOverrides`.

use crate::instruments::swaption::Swaption;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::prelude::Result;

/// Vega calculator for swaptions
pub struct VegaCalculator;

impl MetricCalculator for VegaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &Swaption = context.instrument_as()?;
        let disc = context.curves.get_discount_ref(option.discount_curve_id.as_ref())?;
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
                .surface_ref(option.vol_surface_id.as_str())?
                .value_clamped(t, option.strike_rate)
        };

        // Use centralized Black76 helper for forward-based pricing
        use crate::instruments::common::models::d1_black76;
        let d1 = d1_black76(forward, option.strike_rate, sigma, t);

        let vega =
            forward * finstack_core::math::norm_pdf(d1) * t.sqrt() / super::config::VOL_PCT_SCALE;
        // Scale by notional and annuity for cash vega
        Ok(vega * option.notional.amount() * annuity)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}
