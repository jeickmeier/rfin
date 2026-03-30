//! Unified parsing utilities for converting JavaScript values to Rust types.
//!
//! This is the single source of truth for all parsing logic in the WASM bindings.
//! All label normalization and parsing should go through this module.

use super::labels::normalize_label;
use crate::core::error::js_error;
use finstack_core::cashflow::CFKind;
use finstack_core::config::RoundingMode;
use finstack_core::dates::{BusinessDayConvention, DayCount, StubKind, Tenor};
use finstack_core::math::interp::{ExtrapolationPolicy, InterpStyle};
use wasm_bindgen::JsValue;

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
            "flat_fwd" => Ok(InterpStyle::LogLinear),
            "piecewise_quadratic_forward" | "piecewise_quadratic" | "pqf" => {
                Ok(InterpStyle::PiecewiseQuadraticForward)
            }
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
/// Parse a rounding mode from a string label.
pub(crate) fn parse_rounding_mode(name: &str) -> Result<RoundingMode, JsValue> {
    RoundingMode::parse_from_string(name)
}
// StubKind parsing
impl ParseFromString for StubKind {
    /// Parse a stub kind from a string label.
    fn parse_from_string(label: &str) -> Result<Self, JsValue> {
        let normalized = normalize_label(label);
        match normalized.as_str() {
            "none" => Ok(StubKind::None),
            "short_front" => Ok(StubKind::ShortFront),
            "short_back" => Ok(StubKind::ShortBack),
            "long_front" => Ok(StubKind::LongFront),
            "long_back" => Ok(StubKind::LongBack),
            _ => Err(js_error(format!("Unknown stub kind: {}", label))),
        }
    }
}

// CFKind parsing
impl ParseFromString for CFKind {
    /// Parse a cashflow kind from a string label.
    fn parse_from_string(label: &str) -> Result<Self, JsValue> {
        let normalized = normalize_label(label);
        match normalized.as_str() {
            "fixed" => Ok(CFKind::Fixed),
            "float_reset" => Ok(CFKind::FloatReset),
            "notional" => Ok(CFKind::Notional),
            "pik" => Ok(CFKind::PIK),
            "amortization" | "amort" => Ok(CFKind::Amortization),
            "fee" => Ok(CFKind::Fee),
            "stub" => Ok(CFKind::Stub),
            _ => Err(js_error(format!("Unknown cashflow kind: {}", label))),
        }
    }
}

// Tenor parsing
impl ParseFromString for Tenor {
    /// Parse a frequency from a string label.
    fn parse_from_string(label: &str) -> Result<Self, JsValue> {
        let normalized = normalize_label(label);
        match normalized.as_str() {
            "annual" | "yearly" => Ok(Tenor::annual()),
            "semiannual" | "semi_annual" => Ok(Tenor::semi_annual()),
            "quarterly" => Ok(Tenor::quarterly()),
            "monthly" => Ok(Tenor::monthly()),
            "bimonthly" | "bi_monthly" => Ok(Tenor::bimonthly()),
            "biweekly" | "bi_weekly" => Ok(Tenor::biweekly()),
            "weekly" => Ok(Tenor::weekly()),
            "daily" => Ok(Tenor::daily()),
            _ => Err(js_error(format!("Unknown frequency: {}", label))),
        }
    }
}
