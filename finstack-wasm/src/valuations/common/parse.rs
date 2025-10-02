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

/// Normalizes a label string for consistent parsing (lowercase, underscore normalization)
pub(crate) fn normalize_label(label: &str) -> String {
    label.to_lowercase().replace('-', "_")
}

/// Trait for parsing JavaScript string labels into strongly-typed Rust enums
///
/// This trait standardizes parsing across all instrument bindings, replacing
/// scattered parse_X() helper functions with a unified approach.
pub(crate) trait FromJsLabel: Sized {
    /// Parse a string label into the target type
    ///
    /// # Arguments
    /// * `label` - The string label to parse (case-insensitive)
    ///
    /// # Returns
    /// * `Ok(Self)` if parsing succeeded
    /// * `Err(JsValue)` with a user-friendly error message if parsing failed
    fn from_label(label: &str) -> Result<Self, JsValue>;
}

// ============================================================================
// Date/Schedule Types
// ============================================================================

impl FromJsLabel for Frequency {
    fn from_label(label: &str) -> Result<Self, JsValue> {
        let normalized = normalize_label(label);
        match normalized.as_str() {
            "monthly" => Ok(Frequency::monthly()),
            "quarterly" => Ok(Frequency::quarterly()),
            "semi_annual" | "semiannual" => Ok(Frequency::semi_annual()),
            "annual" => Ok(Frequency::annual()),
            _ => Err(js_error(format!("Unknown frequency: {}", label))),
        }
    }
}

impl FromJsLabel for DayCount {
    fn from_label(label: &str) -> Result<Self, JsValue> {
        let normalized = normalize_label(label);
        match normalized.as_str() {
            "act_360" | "act360" => Ok(DayCount::Act360),
            "act_365f" | "act365f" => Ok(DayCount::Act365F),
            "act_act" | "actact" => Ok(DayCount::ActAct),
            "thirty_360" | "30_360" | "30360" => Ok(DayCount::Thirty360),
            _ => Err(js_error(format!("Unknown day count: {}", label))),
        }
    }
}

impl FromJsLabel for StubKind {
    fn from_label(label: &str) -> Result<Self, JsValue> {
        let normalized = normalize_label(label);
        match normalized.as_str() {
            "none" => Ok(StubKind::None),
            _ => normalized
                .parse()
                .map_err(|e: String| js_error(format!("Invalid stub kind: {}", e))),
        }
    }
}

impl FromJsLabel for BusinessDayConvention {
    fn from_label(label: &str) -> Result<Self, JsValue> {
        let normalized = normalize_label(label);
        match normalized.as_str() {
            "following" => Ok(BusinessDayConvention::Following),
            "modified_following" | "modifiedfollowing" => {
                Ok(BusinessDayConvention::ModifiedFollowing)
            }
            "preceding" => Ok(BusinessDayConvention::Preceding),
            _ => Err(js_error(format!(
                "Invalid business day convention: {}",
                label
            ))),
        }
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

/// Parses an optional label string, returning a default value if None
pub(crate) fn parse_optional_with_default<T: FromJsLabel>(
    label: Option<String>,
    default: T,
) -> Result<T, JsValue> {
    match label {
        Some(s) => T::from_label(&s),
        None => Ok(default),
    }
}

