//! Unified parsing utilities for converting JavaScript values to Rust types.
//!
//! Provides ergonomic helpers for extracting common types from JsValue,
//! with consistent error handling and normalization.

use super::labels::normalize_label;
use crate::core::error::js_error;
use finstack_core::config::RoundingMode;
use finstack_core::currency::Currency;
use finstack_core::dates::{BusinessDayConvention, DayCount};
use finstack_core::math::interp::{ExtrapolationPolicy, InterpStyle};
use std::str::FromStr;
use wasm_bindgen::JsValue;

/// Parse a currency from a JavaScript value (string code expected).
#[allow(dead_code)]
pub(crate) fn parse_currency(value: &JsValue) -> Result<Currency, JsValue> {
    if let Some(code) = value.as_string() {
        Currency::from_str(&code)
            .map_err(|_| crate::core::error::unknown_currency(&code))
    } else {
        Err(js_error("Expected currency code string (e.g., 'USD')"))
    }
}

/// Parse a business day convention from a string or return a default.
#[allow(dead_code)]
pub(crate) fn parse_business_day_convention(
    value: &JsValue,
    default: BusinessDayConvention,
) -> Result<BusinessDayConvention, JsValue> {
    if value.is_undefined() || value.is_null() {
        return Ok(default);
    }
    
    if let Some(name) = value.as_string() {
        BusinessDayConvention::parse_from_string(&name)
    } else {
        Err(js_error("Expected business day convention string"))
    }
}

/// Parse a day count convention from a string label.
#[allow(dead_code)]
pub(crate) fn parse_day_count(label: &str) -> Result<DayCount, JsValue> {
    DayCount::parse_from_string(label)
}

/// Parse a rounding mode from a string label.
pub(crate) fn parse_rounding_mode(name: &str) -> Result<RoundingMode, JsValue> {
    RoundingMode::parse_from_string(name)
}

/// Parse an interpolation style from a string label.
#[allow(dead_code)]
pub(crate) fn parse_interp_style(
    name: &str,
    default: InterpStyle,
) -> Result<InterpStyle, JsValue> {
    if name.is_empty() {
        return Ok(default);
    }
    
    InterpStyle::parse_from_string(name)
}

/// Parse an extrapolation policy from a string label.
#[allow(dead_code)]
pub(crate) fn parse_extrapolation_policy(name: &str) -> Result<ExtrapolationPolicy, JsValue> {
    ExtrapolationPolicy::parse_from_string(name)
}

/// Unified parsing trait for types that can be parsed from strings.
/// 
/// This trait provides a consistent interface for parsing various
/// finstack types from string labels with proper error handling.
pub(crate) trait ParseFromString: Sized {
    type Error;
    
    /// Parse from a normalized string label.
    fn parse_from_normalized(label: &str) -> Result<Self, Self::Error>;
    
    /// Parse from a raw string, handling normalization automatically.
    fn parse_from_string(label: &str) -> Result<Self, JsValue> {
        let normalized = normalize_label(label);
        Self::parse_from_normalized(&normalized)
            .map_err(|_| js_error(format!("Unknown {}: {}", Self::type_name(), label)))
    }
    
    /// Return the type name for error messages.
    fn type_name() -> &'static str;
}

/// Macro to implement ParseFromString for enum-like types.
macro_rules! impl_parse_from_string {
    ($type:ty, $type_name:literal, {
        $( $variant:ident => [ $( $label:literal ),* $(,)? ] ),* $(,)?
    }) => {
        impl ParseFromString for $type {
            type Error = ();
            
            fn parse_from_normalized(label: &str) -> Result<Self, Self::Error> {
                match label {
                    $( $( $label => Ok(Self::$variant), )* )*
                    _ => Err(()),
                }
            }
            
            fn type_name() -> &'static str {
                $type_name
            }
        }
    };
}

impl_parse_from_string!(DayCount, "day-count convention", {
    Act360 => ["act_360", "actual_360"],
    Act365F => ["act_365f", "actual_365f"],
    Act365L => ["act_365l", "actual_365l", "act_365afb"],
    Thirty360 => ["30_360", "thirty_360", "30u_360"],
    ThirtyE360 => ["30e_360", "30_360e"],
    ActAct => ["act_act", "actual_actual", "act_act_isda"],
    ActActIsma => ["act_act_isma", "icma"],
    Bus252 => ["bus_252", "business_252"],
});

impl_parse_from_string!(BusinessDayConvention, "business day convention", {
    Unadjusted => ["unadjusted"],
    Following => ["following"],
    ModifiedFollowing => ["modified_following"],
    Preceding => ["preceding"],
    ModifiedPreceding => ["modified_preceding"],
});

impl_parse_from_string!(RoundingMode, "rounding mode", {
    Bankers => ["bankers", "banker"],
    AwayFromZero => ["away_from_zero", "awayfromzero"],
    TowardZero => ["toward_zero", "towards_zero"],
    Floor => ["floor"],
    Ceil => ["ceil", "ceiling"],
});

impl_parse_from_string!(InterpStyle, "interpolation style", {
    Linear => ["linear"],
    LogLinear => ["log_linear"],
    MonotoneConvex => ["monotone_convex"],
    CubicHermite => ["cubic_hermite"],
    FlatFwd => ["flat_fwd"],
});

impl_parse_from_string!(ExtrapolationPolicy, "extrapolation policy", {
    FlatZero => ["flat_zero"],
    FlatForward => ["flat_forward"],
});

