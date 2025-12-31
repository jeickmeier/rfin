//! WASM bindings for volatility conventions and conversion utilities.

use crate::core::error::js_error;
use finstack_core::math::volatility::{convert_atm_volatility, VolatilityConvention};
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

/// Convert ATM volatility between conventions by equating option prices.
///
/// This function performs ATM (at-the-money, strike = forward) volatility conversion.
/// For surface-aware or strike-specific conversions, use a volatility surface.
///
/// @param {number} vol - Input volatility (must be positive and finite)
/// @param {VolatilityConvention} fromConvention - Source convention
/// @param {VolatilityConvention} toConvention - Target convention
/// @param {number} forwardRate - Forward rate
/// @param {number} timeToExpiry - Time to expiry (years, must be non-negative)
/// @returns {number} Converted volatility
/// @throws {Error} If inputs are invalid or conversion fails
#[wasm_bindgen(js_name = convertAtmVolatility)]
pub fn convert_atm_volatility_js(
    vol: f64,
    from_convention: &JsVolatilityConvention,
    to_convention: &JsVolatilityConvention,
    forward_rate: f64,
    time_to_expiry: f64,
) -> Result<f64, JsValue> {
    convert_atm_volatility(
        vol,
        from_convention.inner,
        to_convention.inner,
        forward_rate,
        time_to_expiry,
    )
    .map_err(|e| js_error(e.to_string()))
}
