//! WASM bindings for volatility conventions and conversion utilities.

use crate::core::error::js_error;
use finstack_core::volatility::{convert_volatility, VolatilityConvention};
use wasm_bindgen::prelude::*;

/// Volatility quoting convention wrapper.
#[wasm_bindgen(js_name = VolatilityConvention)]
#[derive(Clone, Debug)]
pub struct JsVolatilityConvention {
    pub(crate) inner: VolatilityConvention,
}

#[wasm_bindgen(js_class = VolatilityConvention)]
impl JsVolatilityConvention {
    /// Normal (Bachelier) volatility.
    #[wasm_bindgen(js_name = normal)]
    pub fn normal() -> JsVolatilityConvention {
        JsVolatilityConvention {
            inner: VolatilityConvention::Normal,
        }
    }

    /// Lognormal (Black) volatility.
    #[wasm_bindgen(js_name = lognormal)]
    pub fn lognormal() -> JsVolatilityConvention {
        JsVolatilityConvention {
            inner: VolatilityConvention::Lognormal,
        }
    }

    /// Shifted lognormal volatility with explicit shift.
    #[wasm_bindgen(js_name = shiftedLognormal)]
    pub fn shifted_lognormal(shift: f64) -> JsVolatilityConvention {
        JsVolatilityConvention {
            inner: VolatilityConvention::ShiftedLognormal { shift },
        }
    }
}

/// Convert volatility between conventions by equating option prices.
///
/// @param {number} vol - Input volatility
/// @param {VolatilityConvention} fromConvention - Source convention
/// @param {VolatilityConvention} toConvention - Target convention
/// @param {number} forwardRate - Forward rate
/// @param {number} timeToExpiry - Time to expiry (years)
/// @param {number} zeroThreshold - Threshold for near-zero forwards
#[wasm_bindgen(js_name = convertVolatility)]
pub fn convert_volatility_js(
    vol: f64,
    from_convention: &JsVolatilityConvention,
    to_convention: &JsVolatilityConvention,
    forward_rate: f64,
    time_to_expiry: f64,
    zero_threshold: f64,
) -> Result<f64, JsValue> {
    if !vol.is_finite()
        || !forward_rate.is_finite()
        || !time_to_expiry.is_finite()
        || !zero_threshold.is_finite()
    {
        return Err(js_error("convertVolatility: inputs must be finite numbers"));
    }

    Ok(convert_volatility(
        vol,
        from_convention.inner,
        to_convention.inner,
        forward_rate,
        time_to_expiry,
        zero_threshold,
    ))
}
