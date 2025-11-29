//! Delta calculator for swaptions.
//!
//! Computes cash delta using Black or Normal model greeks with forward swap rate and
//! underlying swap annuity. Uses SABR-implied vol if parameters are set,
//! otherwise uses the volatility surface or an override from `PricingOverrides`.

use crate::instruments::common::parameters::OptionType;
use crate::instruments::swaption::{Swaption, VolatilityModel};
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::prelude::Result;

/// Delta calculator for swaptions
pub struct DeltaCalculator;

impl MetricCalculator for DeltaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &Swaption = context.instrument_as()?;

        // Use consolidated helper to get pre-computed inputs
        let inputs = match option.greek_inputs(&context.curves, context.as_of)? {
            Some(inputs) => inputs,
            None => return Ok(0.0), // Option expired
        };

        let delta = match option.vol_model {
            VolatilityModel::Black => {
                if inputs.forward <= 0.0 {
                    // Black model undefined for negative rates
                    return Ok(0.0);
                }
                use crate::instruments::common::models::d1_black76;
                let d1 = d1_black76(inputs.forward, option.strike_rate, inputs.sigma, inputs.time_to_expiry);
                match option.option_type {
                    OptionType::Call => finstack_core::math::norm_cdf(d1),
                    OptionType::Put => -finstack_core::math::norm_cdf(-d1),
                }
            }
            VolatilityModel::Normal => {
                use crate::instruments::common::models::volatility::normal::d_bachelier;
                let d = d_bachelier(inputs.forward, option.strike_rate, inputs.sigma, inputs.time_to_expiry);
                match option.option_type {
                    OptionType::Call => finstack_core::math::norm_cdf(d),
                    OptionType::Put => -finstack_core::math::norm_cdf(-d),
                }
            }
        };

        // Scale by notional and annuity for cash delta
        Ok(delta * option.notional.amount() * inputs.annuity)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}
