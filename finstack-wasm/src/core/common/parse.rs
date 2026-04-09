//! Unified parsing utilities for converting JavaScript values to Rust types.
//!
//! Delegates to `FromStr` implementations provided by `finstack_core` for all
//! supported types. The `ParseFromString` trait provides a uniform error-mapping
//! layer that converts core parse errors into `JsValue` for WASM consumers.

use crate::core::error::js_error;
use finstack_core::cashflow::CFKind;
use finstack_core::config::RoundingMode;
use finstack_core::dates::{BusinessDayConvention, DayCount, StubKind, Tenor};
use finstack_core::math::interp::{ExtrapolationPolicy, InterpStyle};
use std::str::FromStr;
use wasm_bindgen::JsValue;

/// Unified parsing trait for types that can be parsed from strings.
///
/// Delegates to the core crate's `FromStr` implementations with error
/// mapping to `JsValue`.
pub(crate) trait ParseFromString: Sized {
    /// Parse from a raw string, handling normalization automatically.
    fn parse_from_string(label: &str) -> Result<Self, JsValue>;
}

/// Blanket-style macro for types whose `FromStr::Err` implements `ToString`.
macro_rules! impl_parse_from_string {
    ($ty:ty) => {
        impl ParseFromString for $ty {
            fn parse_from_string(label: &str) -> Result<Self, JsValue> {
                Self::from_str(label).map_err(|e| js_error(e.to_string()))
            }
        }
    };
}

impl_parse_from_string!(DayCount);
impl_parse_from_string!(BusinessDayConvention);
impl_parse_from_string!(RoundingMode);
impl_parse_from_string!(InterpStyle);
impl_parse_from_string!(ExtrapolationPolicy);
impl_parse_from_string!(StubKind);
impl_parse_from_string!(CFKind);
impl_parse_from_string!(Tenor);

/// Parse a rounding mode from a string label.
pub(crate) fn parse_rounding_mode(name: &str) -> Result<RoundingMode, JsValue> {
    RoundingMode::parse_from_string(name)
}
