use crate::core::error::js_error;
use finstack_core::math::interp::{ExtrapolationPolicy, InterpStyle};
use std::str::FromStr;
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

    #[wasm_bindgen(js_name = FlatForward)]
    pub fn flat_fwd() -> JsInterpStyle {
        Self::new(InterpStyle::LogLinear)
    }

    #[wasm_bindgen(js_name = PiecewiseQuadraticForward)]
    pub fn piecewise_quadratic_forward() -> JsInterpStyle {
        Self::new(InterpStyle::PiecewiseQuadraticForward)
    }

    #[wasm_bindgen(js_name = fromName)]
    pub fn from_name(name: &str) -> Result<JsInterpStyle, JsValue> {
        InterpStyle::from_str(name)
            .map(Self::new)
            .map_err(|e| js_error(e.to_string()))
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
        ExtrapolationPolicy::from_str(name)
            .map(Self::new)
            .map_err(|e| js_error(e.to_string()))
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
