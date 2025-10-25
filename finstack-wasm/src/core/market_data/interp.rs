use crate::core::error::js_error;
use finstack_core::math::interp::{ExtrapolationPolicy, InterpStyle};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

#[wasm_bindgen(js_name = InterpStyle)]
#[derive(Clone, Copy, Debug)]
pub struct JsInterpStyle {
    inner: InterpStyle,
}

impl JsInterpStyle {
    fn new(inner: InterpStyle) -> Self {
        Self { inner }
    }
}

impl From<JsInterpStyle> for InterpStyle {
    fn from(value: JsInterpStyle) -> Self {
        value.inner
    }
}

impl From<InterpStyle> for JsInterpStyle {
    fn from(value: InterpStyle) -> Self {
        Self::new(value)
    }
}

#[wasm_bindgen(js_class = InterpStyle)]
impl JsInterpStyle {
    #[wasm_bindgen(js_name = Linear)]
    pub fn linear() -> JsInterpStyle {
        Self::new(InterpStyle::Linear)
    }

    #[wasm_bindgen(js_name = LogLinear)]
    pub fn log_linear() -> JsInterpStyle {
        Self::new(InterpStyle::LogLinear)
    }

    #[wasm_bindgen(js_name = MonotoneConvex)]
    pub fn monotone_convex() -> JsInterpStyle {
        Self::new(InterpStyle::MonotoneConvex)
    }

    #[wasm_bindgen(js_name = CubicHermite)]
    pub fn cubic_hermite() -> JsInterpStyle {
        Self::new(InterpStyle::CubicHermite)
    }

    #[wasm_bindgen(js_name = FlatFwd)]
    pub fn flat_fwd() -> JsInterpStyle {
        Self::new(InterpStyle::FlatFwd)
    }

    #[wasm_bindgen(js_name = fromName)]
    pub fn from_name(name: &str) -> Result<JsInterpStyle, JsValue> {
        match name.to_ascii_lowercase().as_str() {
            "linear" => Ok(Self::linear()),
            "log_linear" | "loglinear" => Ok(Self::log_linear()),
            "monotone_convex" | "monotoneconvex" => Ok(Self::monotone_convex()),
            "cubic_hermite" | "cubichermite" => Ok(Self::cubic_hermite()),
            "flat_fwd" | "flat_forward" | "flatforward" => Ok(Self::flat_fwd()),
            other => Err(js_error(format!("Unknown interpolation style: {other}"))),
        }
    }
}

#[wasm_bindgen(js_name = ExtrapolationPolicy)]
#[derive(Clone, Copy, Debug)]
pub struct JsExtrapolationPolicy {
    inner: ExtrapolationPolicy,
}

impl JsExtrapolationPolicy {
    fn new(inner: ExtrapolationPolicy) -> Self {
        Self { inner }
    }
}

impl From<JsExtrapolationPolicy> for ExtrapolationPolicy {
    fn from(value: JsExtrapolationPolicy) -> Self {
        value.inner
    }
}

impl From<ExtrapolationPolicy> for JsExtrapolationPolicy {
    fn from(value: ExtrapolationPolicy) -> Self {
        Self::new(value)
    }
}

#[wasm_bindgen(js_class = ExtrapolationPolicy)]
impl JsExtrapolationPolicy {
    #[wasm_bindgen(js_name = FlatZero)]
    pub fn flat_zero() -> JsExtrapolationPolicy {
        Self::new(ExtrapolationPolicy::FlatZero)
    }

    #[wasm_bindgen(js_name = FlatForward)]
    pub fn flat_forward() -> JsExtrapolationPolicy {
        Self::new(ExtrapolationPolicy::FlatForward)
    }

    #[wasm_bindgen(js_name = fromName)]
    pub fn from_name(name: &str) -> Result<JsExtrapolationPolicy, JsValue> {
        match name.to_ascii_lowercase().as_str() {
            "flat_zero" | "flatzero" => Ok(Self::flat_zero()),
            "flat_forward" | "flatforward" | "flat_fwd" => Ok(Self::flat_forward()),
            other => Err(js_error(format!("Unknown extrapolation policy: {other}"))),
        }
    }
}

pub(crate) fn parse_interp_style(
    value: Option<&str>,
    default: InterpStyle,
) -> Result<InterpStyle, JsValue> {
    match value {
        None => Ok(default),
        Some(name) => JsInterpStyle::from_name(name).map(Into::into),
    }
}

pub(crate) fn parse_extrapolation(value: Option<&str>) -> Result<ExtrapolationPolicy, JsValue> {
    match value {
        None => Ok(ExtrapolationPolicy::FlatZero),
        Some(name) => JsExtrapolationPolicy::from_name(name).map(Into::into),
    }
}
