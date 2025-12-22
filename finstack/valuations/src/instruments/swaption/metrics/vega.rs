//! Vega calculator for swaptions.
//!
//! Computes cash vega using Black or Normal model vega with forward swap rate and
//! underlying swap annuity. Uses SABR-implied vol if parameters are set,
//! otherwise uses the volatility surface or an override from `PricingOverrides`.

use crate::instruments::swaption::{Swaption, VolatilityModel};
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::Result;

/// Vega calculator for swaptions
pub struct VegaCalculator;

impl MetricCalculator for VegaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &Swaption = context.instrument_as()?;

        // Use consolidated helper to get pre-computed inputs
        let inputs = match option.greek_inputs(&context.curves, context.as_of)? {
            Some(inputs) => inputs,
            None => return Ok(0.0), // Option expired
        };

        let vega_raw = match option.vol_model {
            VolatilityModel::Black => {
                if inputs.forward <= 0.0 {
                    0.0
                } else {
                    use crate::instruments::common::models::d1_black76;
                    let d1 = d1_black76(
                        inputs.forward,
                        option.strike_rate,
                        inputs.sigma,
                        inputs.time_to_expiry,
                    );
                    inputs.forward
                        * finstack_core::math::norm_pdf(d1)
                        * inputs.time_to_expiry.sqrt()
                }
            }
            VolatilityModel::Normal => {
                use crate::instruments::common::models::volatility::normal::d_bachelier;
                let d = d_bachelier(
                    inputs.forward,
                    option.strike_rate,
                    inputs.sigma,
                    inputs.time_to_expiry,
                );
                finstack_core::math::norm_pdf(d) * inputs.time_to_expiry.sqrt()
            }
        };

        let vega = vega_raw / super::config::VOL_PCT_SCALE;
        // Scale by notional and annuity for cash vega
        Ok(vega * option.notional.amount() * inputs.annuity)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}
