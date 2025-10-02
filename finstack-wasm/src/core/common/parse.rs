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
        Currency::from_str(&code).map_err(|_| crate::core::error::unknown_currency(&code))
    } else {
        Err(js_error("Expected currency code string (e.g., 'USD')"))
    }
}

/// Unified parsing trait for types that can be parsed from strings.
///
/// This trait provides a consistent interface for parsing various
/// finstack types from string labels with proper error handling.
pub(crate) trait ParseFromString: Sized {
    /// Parse from a raw string, handling normalization automatically.
    fn parse_from_string(label: &str) -> Result<Self, JsValue>;
}

// DayCount parsing
impl ParseFromString for DayCount {
    /// Parse a day count convention from a string label.
    ///
    /// Accepts various formats (case-insensitive, '-' and '/' converted to '_').
    fn parse_from_string(label: &str) -> Result<Self, JsValue> {
        let normalized = normalize_label(label);
        match normalized.as_str() {
            "act_360" | "actual_360" => Ok(DayCount::Act360),
            "act_365f" | "actual_365f" => Ok(DayCount::Act365F),
            "act_365l" | "actual_365l" | "act_365afb" => Ok(DayCount::Act365L),
            "30_360" | "thirty_360" | "30u_360" => Ok(DayCount::Thirty360),
            "30e_360" | "30_360e" => Ok(DayCount::ThirtyE360),
            "act_act" | "actual_actual" | "act_act_isda" => Ok(DayCount::ActAct),
            "act_act_isma" | "icma" => Ok(DayCount::ActActIsma),
            "bus_252" | "business_252" => Ok(DayCount::Bus252),
            _ => Err(js_error(format!("Unknown day-count convention: {}", label))),
        }
    }
}

// BusinessDayConvention parsing
impl ParseFromString for BusinessDayConvention {
    /// Parse a business day convention from a string label.
    fn parse_from_string(label: &str) -> Result<Self, JsValue> {
        let normalized = normalize_label(label);
        match normalized.as_str() {
            "unadjusted" => Ok(BusinessDayConvention::Unadjusted),
            "following" => Ok(BusinessDayConvention::Following),
            "modified_following" => Ok(BusinessDayConvention::ModifiedFollowing),
            "preceding" => Ok(BusinessDayConvention::Preceding),
            "modified_preceding" => Ok(BusinessDayConvention::ModifiedPreceding),
            _ => Err(js_error(format!(
                "Unknown business day convention: {}",
                label
            ))),
        }
    }
}

// RoundingMode parsing
impl ParseFromString for RoundingMode {
    /// Parse a rounding mode from a string label.
    fn parse_from_string(label: &str) -> Result<Self, JsValue> {
        let normalized = normalize_label(label);
        match normalized.as_str() {
            "bankers" | "banker" => Ok(RoundingMode::Bankers),
            "away_from_zero" | "awayfromzero" => Ok(RoundingMode::AwayFromZero),
            "toward_zero" | "towards_zero" => Ok(RoundingMode::TowardZero),
            "floor" => Ok(RoundingMode::Floor),
            "ceil" | "ceiling" => Ok(RoundingMode::Ceil),
            _ => Err(js_error(format!("Unknown rounding mode: {}", label))),
        }
    }
}

// InterpStyle parsing
impl ParseFromString for InterpStyle {
    /// Parse an interpolation style from a string label.
    fn parse_from_string(label: &str) -> Result<Self, JsValue> {
        let normalized = normalize_label(label);
        match normalized.as_str() {
            "linear" => Ok(InterpStyle::Linear),
            "log_linear" => Ok(InterpStyle::LogLinear),
            "monotone_convex" => Ok(InterpStyle::MonotoneConvex),
            "cubic_hermite" => Ok(InterpStyle::CubicHermite),
            "flat_fwd" => Ok(InterpStyle::FlatFwd),
            _ => Err(js_error(format!("Unknown interpolation style: {}", label))),
        }
    }
}

// ExtrapolationPolicy parsing
impl ParseFromString for ExtrapolationPolicy {
    /// Parse an extrapolation policy from a string label.
    fn parse_from_string(label: &str) -> Result<Self, JsValue> {
        let normalized = normalize_label(label);
        match normalized.as_str() {
            "flat_zero" => Ok(ExtrapolationPolicy::FlatZero),
            "flat_forward" => Ok(ExtrapolationPolicy::FlatForward),
            _ => Err(js_error(format!("Unknown extrapolation policy: {}", label))),
        }
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
pub(crate) fn parse_interp_style(name: &str, default: InterpStyle) -> Result<InterpStyle, JsValue> {
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
