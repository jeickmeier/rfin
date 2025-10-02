//! Parsing utilities for valuations-specific types.
//!
//! Delegates to core parsing module for common types (DayCount, Frequency, etc.)
//! and provides parsing for instrument-specific types.

use crate::core::common::parse::ParseFromString;
use crate::core::common::labels::normalize_label;
use crate::core::error::js_error;
use finstack_core::dates::{BusinessDayConvention, DayCount, Frequency, StubKind};
use finstack_core::math::stats::RealizedVarMethod;
use finstack_valuations::instruments::cds::PayReceive as CdsPayReceive;
use finstack_valuations::instruments::cds_tranche::TrancheSide;
use finstack_valuations::instruments::common::parameters::OptionType;
use finstack_valuations::instruments::inflation_linked_bond::{
    DeflationProtection, IndexationMethod,
};
use finstack_valuations::instruments::inflation_swap::PayReceiveInflation;
use finstack_valuations::instruments::ir_future::Position;
use finstack_valuations::instruments::repo::RepoType;
use finstack_valuations::instruments::swaption::{SwaptionExercise, SwaptionSettlement};
use finstack_valuations::instruments::variance_swap::PayReceive as VarSwapPayReceive;
use wasm_bindgen::JsValue;

/// Trait for parsing JavaScript string labels into strongly-typed Rust enums.
///
/// For common types like DayCount, Frequency, etc., use ParseFromString from
/// crate::core::common::parse instead. This trait is for valuations-specific types.
pub(crate) trait FromJsLabel: Sized {
    /// Parse a string label into the target type.
    fn from_label(label: &str) -> Result<Self, JsValue>;
}

// ============================================================================
// Date/Schedule Types - Delegate to Core
// ============================================================================

impl FromJsLabel for Frequency {
    fn from_label(label: &str) -> Result<Self, JsValue> {
        Frequency::parse_from_string(label)
    }
}

impl FromJsLabel for DayCount {
    fn from_label(label: &str) -> Result<Self, JsValue> {
        DayCount::parse_from_string(label)
    }
}

impl FromJsLabel for StubKind {
    fn from_label(label: &str) -> Result<Self, JsValue> {
        StubKind::parse_from_string(label)
    }
}

impl FromJsLabel for BusinessDayConvention {
    fn from_label(label: &str) -> Result<Self, JsValue> {
        BusinessDayConvention::parse_from_string(label)
    }
}

// ============================================================================
// Swap & Rate Instruments
// ============================================================================

impl FromJsLabel for PayReceiveInflation {
    fn from_label(label: &str) -> Result<Self, JsValue> {
        let normalized = normalize_label(label);
        match normalized.as_str() {
            "pay_fixed" | "payfixed" => Ok(PayReceiveInflation::PayFixed),
            "receive_fixed" | "receivefixed" => Ok(PayReceiveInflation::ReceiveFixed),
            _ => normalized
                .parse()
                .map_err(|e: String| js_error(format!("Invalid inflation swap side: {}", e))),
        }
    }
}

// ============================================================================
// Credit Instruments
// ============================================================================

impl FromJsLabel for CdsPayReceive {
    fn from_label(label: &str) -> Result<Self, JsValue> {
        let normalized = normalize_label(label);
        match normalized.as_str() {
            "pay_protection" | "payprotection" => Ok(CdsPayReceive::PayProtection),
            "receive_protection" | "receiveprotection" => Ok(CdsPayReceive::ReceiveProtection),
            _ => normalized
                .parse()
                .map_err(|e: String| js_error(format!("Invalid CDS pay/receive side: {}", e))),
        }
    }
}

impl FromJsLabel for VarSwapPayReceive {
    fn from_label(label: &str) -> Result<Self, JsValue> {
        let normalized = normalize_label(label);
        match normalized.as_str() {
            "receive" => Ok(VarSwapPayReceive::Receive),
            "pay" => Ok(VarSwapPayReceive::Pay),
            _ => Err(js_error(format!("Invalid variance swap side: {}", label))),
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
            _ => normalized.parse().map_err(|e: String| {
                js_error(format!("Invalid deflation protection: {}", e))
            }),
        }
    }
}

// ============================================================================
// Variance Swaps
// ============================================================================

impl FromJsLabel for RealizedVarMethod {
    fn from_label(label: &str) -> Result<Self, JsValue> {
        let normalized = normalize_label(label);
        match normalized.as_str() {
            "close_to_close" | "closetoclose" => Ok(RealizedVarMethod::CloseToClose),
            "parkinson" => Ok(RealizedVarMethod::Parkinson),
            "garman_klass" | "garmanklass" => Ok(RealizedVarMethod::GarmanKlass),
            "rogers_satchell" | "rogerssatchell" => Ok(RealizedVarMethod::RogersSatchell),
            "yang_zhang" | "yangzhang" => Ok(RealizedVarMethod::YangZhang),
            _ => Err(js_error(format!(
                "Unknown realized variance method: {}",
                label
            ))),
        }
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

