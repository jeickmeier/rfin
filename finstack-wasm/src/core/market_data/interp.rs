use crate::core::utils::js_error;
use finstack_core::math::interp::{ExtrapolationPolicy, InterpStyle};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

#[wasm_bindgen(js_name = InterpStyle)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JsInterpStyle {
    Linear,
    LogLinear,
    MonotoneConvex,
    CubicHermite,
    FlatFwd,
}

impl From<JsInterpStyle> for InterpStyle {
    fn from(value: JsInterpStyle) -> Self {
        match value {
            JsInterpStyle::Linear => InterpStyle::Linear,
            JsInterpStyle::LogLinear => InterpStyle::LogLinear,
            JsInterpStyle::MonotoneConvex => InterpStyle::MonotoneConvex,
            JsInterpStyle::CubicHermite => InterpStyle::CubicHermite,
            JsInterpStyle::FlatFwd => InterpStyle::FlatFwd,
        }
    }
}

impl From<InterpStyle> for JsInterpStyle {
    fn from(value: InterpStyle) -> Self {
        match value {
            InterpStyle::Linear => JsInterpStyle::Linear,
            InterpStyle::LogLinear => JsInterpStyle::LogLinear,
            InterpStyle::MonotoneConvex => JsInterpStyle::MonotoneConvex,
            InterpStyle::CubicHermite => JsInterpStyle::CubicHermite,
            InterpStyle::FlatFwd => JsInterpStyle::FlatFwd,
            _ => JsInterpStyle::Linear,
        }
    }
}

#[wasm_bindgen(js_class = InterpStyle)]
impl JsInterpStyle {
    #[wasm_bindgen(js_name = fromName)]
    pub fn from_name(name: &str) -> Result<JsInterpStyle, JsValue> {
        match name.to_ascii_lowercase().as_str() {
            "linear" => Ok(JsInterpStyle::Linear),
            "log_linear" | "loglinear" => Ok(JsInterpStyle::LogLinear),
            "monotone_convex" | "monotoneconvex" => Ok(JsInterpStyle::MonotoneConvex),
            "cubic_hermite" | "cubichermite" => Ok(JsInterpStyle::CubicHermite),
            "flat_fwd" | "flat_forward" | "flatforward" => Ok(JsInterpStyle::FlatFwd),
            other => Err(js_error(format!("Unknown interpolation style: {other}"))),
        }
    }
}

#[wasm_bindgen(js_name = ExtrapolationPolicy)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JsExtrapolationPolicy {
    FlatZero,
    FlatForward,
}

impl From<JsExtrapolationPolicy> for ExtrapolationPolicy {
    fn from(value: JsExtrapolationPolicy) -> Self {
        match value {
            JsExtrapolationPolicy::FlatZero => ExtrapolationPolicy::FlatZero,
            JsExtrapolationPolicy::FlatForward => ExtrapolationPolicy::FlatForward,
        }
    }
}

impl From<ExtrapolationPolicy> for JsExtrapolationPolicy {
    fn from(value: ExtrapolationPolicy) -> Self {
        match value {
            ExtrapolationPolicy::FlatZero => JsExtrapolationPolicy::FlatZero,
            ExtrapolationPolicy::FlatForward => JsExtrapolationPolicy::FlatForward,
            _ => JsExtrapolationPolicy::FlatZero,
        }
    }
}

#[wasm_bindgen(js_class = ExtrapolationPolicy)]
impl JsExtrapolationPolicy {
    #[wasm_bindgen(js_name = fromName)]
    pub fn from_name(name: &str) -> Result<JsExtrapolationPolicy, JsValue> {
        match name.to_ascii_lowercase().as_str() {
            "flat_zero" | "flatzero" => Ok(JsExtrapolationPolicy::FlatZero),
            "flat_forward" | "flatforward" | "flat_fwd" => Ok(JsExtrapolationPolicy::FlatForward),
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
