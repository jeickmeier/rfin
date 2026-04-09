//! Parsing utilities for valuations-specific types.
//!
//! Delegates to core `FromStr` implementations for common types (DayCount,
//! Tenor, etc.) and provides parsing for instrument-specific types.

use crate::core::common::labels::normalize_label;
use crate::core::error::js_error;
use finstack_core::dates::{BusinessDayConvention, DayCount, StubKind, Tenor};
use finstack_core::math::stats::RealizedVarMethod;
use finstack_valuations::instruments::credit_derivatives::cds_tranche::TrancheSide;
use finstack_valuations::instruments::fixed_income::inflation_linked_bond::{
    DeflationProtection, IndexationMethod,
};
use finstack_valuations::instruments::rates::ir_future::Position;
use finstack_valuations::instruments::rates::repo::RepoType;
use finstack_valuations::instruments::rates::swaption::{SwaptionExercise, SwaptionSettlement};
use finstack_valuations::instruments::OptionType;
use finstack_valuations::instruments::PayReceive;
use std::str::FromStr;
use wasm_bindgen::JsValue;

/// Trait for parsing JavaScript string labels into strongly-typed Rust enums.
///
/// For common types like DayCount, Tenor, etc., delegates to core `FromStr`.
/// For valuations-specific types, provides local match blocks.
pub(crate) trait FromJsLabel: Sized {
    /// Parse a string label into the target type.
    fn from_label(label: &str) -> Result<Self, JsValue>;
}

// ============================================================================
// Date/Schedule Types - Delegate to Core FromStr
// ============================================================================

impl FromJsLabel for Tenor {
    fn from_label(label: &str) -> Result<Self, JsValue> {
        Tenor::from_str(label).map_err(|e| js_error(e.to_string()))
    }
}

impl FromJsLabel for DayCount {
    fn from_label(label: &str) -> Result<Self, JsValue> {
        DayCount::from_str(label).map_err(|e| js_error(e.to_string()))
    }
}

impl FromJsLabel for StubKind {
    fn from_label(label: &str) -> Result<Self, JsValue> {
        StubKind::from_str(label).map_err(|e| js_error(e.to_string()))
    }
}

impl FromJsLabel for BusinessDayConvention {
    fn from_label(label: &str) -> Result<Self, JsValue> {
        BusinessDayConvention::from_str(label).map_err(|e| js_error(e.to_string()))
    }
}

// ============================================================================
// Swap & Rate Instruments (PayReceive covers IRS, CDS, and Inflation Swaps)
// ============================================================================

impl FromJsLabel for PayReceive {
    fn from_label(label: &str) -> Result<Self, JsValue> {
        let normalized = normalize_label(label);
        match normalized.as_str() {
            "pay" | "payer" | "short" | "pay_protection" | "payprotection" | "pay_fixed"
            | "payfixed" => Ok(PayReceive::Pay),
            "receive" | "receiver" | "long" | "receive_protection" | "receiveprotection"
            | "receive_fixed" | "receivefixed" => Ok(PayReceive::Receive),
            _ => normalized
                .parse()
                .map_err(|e: String| js_error(format!("Invalid pay/receive side: {}", e))),
        }
    }
}

impl FromJsLabel for TrancheSide {
    fn from_label(label: &str) -> Result<Self, JsValue> {
        let normalized = normalize_label(label);
        match normalized.as_str() {
            "buy_protection" | "buyprotection" => Ok(TrancheSide::BuyProtection),
            "sell_protection" | "sellprotection" => Ok(TrancheSide::SellProtection),
            _ => normalized
                .parse()
                .map_err(|e: String| js_error(format!("Invalid tranche side: {}", e))),
        }
    }
}

// ============================================================================
// Options
// ============================================================================

impl FromJsLabel for OptionType {
    fn from_label(label: &str) -> Result<Self, JsValue> {
        let normalized = normalize_label(label);
        match normalized.as_str() {
            "call" => Ok(OptionType::Call),
            "put" => Ok(OptionType::Put),
            _ => normalized
                .parse()
                .map_err(|e: String| js_error(format!("Invalid option type: {}", e))),
        }
    }
}

impl FromJsLabel for SwaptionExercise {
    fn from_label(label: &str) -> Result<Self, JsValue> {
        let normalized = normalize_label(label);
        match normalized.as_str() {
            "european" => Ok(SwaptionExercise::European),
            _ => normalized
                .parse()
                .map_err(|e: String| js_error(format!("Invalid exercise style: {}", e))),
        }
    }
}

impl FromJsLabel for SwaptionSettlement {
    fn from_label(label: &str) -> Result<Self, JsValue> {
        let normalized = normalize_label(label);
        match normalized.as_str() {
            "physical" => Ok(SwaptionSettlement::Physical),
            _ => normalized
                .parse()
                .map_err(|e: String| js_error(format!("Invalid settlement type: {}", e))),
        }
    }
}

// ============================================================================
// Futures & Repo
// ============================================================================

impl FromJsLabel for Position {
    fn from_label(label: &str) -> Result<Self, JsValue> {
        let normalized = normalize_label(label);
        match normalized.as_str() {
            "long" => Ok(Position::Long),
            "short" => Ok(Position::Short),
            _ => normalized
                .parse()
                .map_err(|e: String| js_error(format!("Invalid position: {}", e))),
        }
    }
}

impl FromJsLabel for RepoType {
    fn from_label(label: &str) -> Result<Self, JsValue> {
        let normalized = normalize_label(label);
        match normalized.as_str() {
            "term" => Ok(RepoType::Term),
            "open" => Ok(RepoType::Open),
            _ => normalized
                .parse()
                .map_err(|e: String| js_error(format!("Invalid repo type: {}", e))),
        }
    }
}

// ============================================================================
// Inflation-Linked Bonds
// ============================================================================

impl FromJsLabel for IndexationMethod {
    fn from_label(label: &str) -> Result<Self, JsValue> {
        let normalized = normalize_label(label);
        match normalized.as_str() {
            "tips" => Ok(IndexationMethod::TIPS),
            _ => normalized
                .parse()
                .map_err(|e: String| js_error(format!("Invalid indexation method: {}", e))),
        }
    }
}

impl FromJsLabel for DeflationProtection {
    fn from_label(label: &str) -> Result<Self, JsValue> {
        let normalized = normalize_label(label);
        match normalized.as_str() {
            "maturity_only" | "maturityonly" => Ok(DeflationProtection::MaturityOnly),
            _ => normalized
                .parse()
                .map_err(|e: String| js_error(format!("Invalid deflation protection: {}", e))),
        }
    }
}

// ============================================================================
// Variance Swaps - Delegate to Core FromStr
// ============================================================================

impl FromJsLabel for RealizedVarMethod {
    fn from_label(label: &str) -> Result<Self, JsValue> {
        RealizedVarMethod::from_str(label).map_err(|e| js_error(e.to_string()))
    }
}

// ============================================================================
// Helper Functions for Optional Values
// ============================================================================

/// Parses an optional label string, returning a default value if None.
///
/// Note: This uses FromJsLabel trait for valuations-specific types.
/// For core types, prefer crate::core::common::parse::parse_optional_with_default.
pub(crate) fn parse_optional_with_default<T: FromJsLabel>(
    label: Option<String>,
    default: T,
) -> Result<T, JsValue> {
    match label {
        Some(s) => T::from_label(&s),
        None => Ok(default),
    }
}
