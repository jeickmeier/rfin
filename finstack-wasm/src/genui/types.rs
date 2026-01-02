//! GenUI-facing wire types for schema generation.
//!
//! These structs intentionally mirror serde-friendly shapes suitable for
//! validation in the UI. When the `ts_export` feature is enabled, `ts-rs`
//! emits TypeScript definitions into the finstack-ui package for Zod wrapping.
//!
//! # Wire Types
//!
//! Wire types are flat, serialization-friendly structures that:
//! - Use primitive types (string, number, array, object)
//! - Avoid complex nested references
//! - Map directly to JSON Schema / Zod schemas
//! - Can be generated to TypeScript via ts-rs

use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[cfg(feature = "ts_export")]
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts_export", derive(TS))]
#[cfg_attr(
    feature = "ts_export",
    ts(
        export,
        export_to = "../finstack-ui/src/schemas/generated/CurvePointWire.ts"
    )
)]
#[cfg_attr(feature = "ts_export", ts(rename_all = "snake_case"))]
pub struct CurvePointWire {
    /// Tenor expressed in fractional years.
    pub tenor_years: f64,
    /// Discount factor at the tenor.
    pub discount_factor: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts_export", derive(TS))]
#[cfg_attr(
    feature = "ts_export",
    ts(
        export,
        export_to = "../finstack-ui/src/schemas/generated/DiscountCurveWire.ts"
    )
)]
#[cfg_attr(feature = "ts_export", ts(rename_all = "snake_case"))]
pub struct DiscountCurveWire {
    pub id: String,
    #[cfg_attr(feature = "ts_export", ts(type = "string"))]
    pub base_date: Date,
    pub points: Vec<CurvePointWire>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts_export", derive(TS))]
#[cfg_attr(
    feature = "ts_export",
    ts(
        export,
        export_to = "../finstack-ui/src/schemas/generated/MarketContextWire.ts"
    )
)]
#[cfg_attr(feature = "ts_export", ts(rename_all = "snake_case"))]
pub struct MarketContextWire {
    #[cfg_attr(feature = "ts_export", ts(type = "string"))]
    pub as_of: Date,
    pub discount_curves: Vec<DiscountCurveWire>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts_export", derive(TS))]
#[cfg_attr(
    feature = "ts_export",
    ts(export, export_to = "../finstack-ui/src/schemas/generated/BondSpec.ts")
)]
#[cfg_attr(feature = "ts_export", ts(rename_all = "snake_case"))]
pub struct BondSpec {
    /// Instrument identifier, typically ISIN or internal code.
    pub id: String,
    /// Currency code (ISO 4217).
    #[cfg_attr(feature = "ts_export", ts(type = "string"))]
    pub currency: Currency,
    /// Principal amount.
    pub notional: f64,
    /// Annual coupon rate as decimal (e.g., 0.05 for 5%).
    pub coupon_rate: f64,
    /// Issue date in ISO-8601 format.
    #[cfg_attr(feature = "ts_export", ts(type = "string"))]
    pub issue: Date,
    /// Maturity date in ISO-8601 format.
    #[cfg_attr(feature = "ts_export", ts(type = "string"))]
    pub maturity: Date,
    /// Discount curve identifier used for pricing.
    pub discount_curve_id: String,
    /// Optional credit curve identifier.
    pub credit_curve_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts_export", derive(TS))]
#[cfg_attr(
    feature = "ts_export",
    ts(
        export,
        export_to = "../finstack-ui/src/schemas/generated/ValuationResultWire.ts"
    )
)]
#[cfg_attr(feature = "ts_export", ts(rename_all = "snake_case"))]
pub struct ValuationResultWire {
    /// Identifier of the instrument valued.
    pub instrument_id: String,
    /// Present value in quote currency.
    pub present_value: f64,
    /// Currency code (ISO 4217).
    #[cfg_attr(feature = "ts_export", ts(type = "string"))]
    pub currency: Currency,
    /// Valuation date as ISO-8601 string.
    #[cfg_attr(feature = "ts_export", ts(type = "string"))]
    pub as_of: Date,
    /// Optional named metrics (DV01, PV01, etc.).
    #[serde(default)]
    pub metrics: BTreeMap<String, f64>,
}

// ============================================================================
// Statement Wire Types
// ============================================================================

/// Wire type for a financial model node specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts_export", derive(TS))]
#[cfg_attr(
    feature = "ts_export",
    ts(
        export,
        export_to = "../finstack-ui/src/schemas/generated/NodeSpecWire.ts"
    )
)]
#[cfg_attr(feature = "ts_export", ts(rename_all = "snake_case"))]
pub struct NodeSpecWire {
    /// Unique node identifier.
    pub id: String,
    /// Human-readable label.
    pub label: String,
    /// Node type: "value", "formula", "forecast", "input".
    pub node_type: String,
    /// Formula expression (if node_type is "formula").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub formula: Option<String>,
    /// Forecast method (if node_type is "forecast").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub forecast_method: Option<String>,
    /// Fixed values per period (if node_type is "value").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub values: Option<BTreeMap<String, f64>>,
}

/// Wire type for a financial model specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts_export", derive(TS))]
#[cfg_attr(
    feature = "ts_export",
    ts(
        export,
        export_to = "../finstack-ui/src/schemas/generated/FinancialModelWire.ts"
    )
)]
#[cfg_attr(feature = "ts_export", ts(rename_all = "snake_case"))]
pub struct FinancialModelWire {
    /// Model identifier.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Currency code for model outputs.
    #[cfg_attr(feature = "ts_export", ts(type = "string"))]
    pub currency: Currency,
    /// Period identifiers in order.
    pub periods: Vec<String>,
    /// Node specifications.
    pub nodes: Vec<NodeSpecWire>,
}

/// Wire type for statement evaluation results.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts_export", derive(TS))]
#[cfg_attr(
    feature = "ts_export",
    ts(
        export,
        export_to = "../finstack-ui/src/schemas/generated/StatementResultsWire.ts"
    )
)]
#[cfg_attr(feature = "ts_export", ts(rename_all = "snake_case"))]
pub struct StatementResultsWire {
    /// Node values indexed by node_id -> period_id -> value.
    pub values: BTreeMap<String, BTreeMap<String, f64>>,
    /// Evaluation metadata.
    pub meta: StatementResultsMetaWire,
}

/// Wire type for statement results metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts_export", derive(TS))]
#[cfg_attr(
    feature = "ts_export",
    ts(
        export,
        export_to = "../finstack-ui/src/schemas/generated/StatementResultsMetaWire.ts"
    )
)]
#[cfg_attr(feature = "ts_export", ts(rename_all = "snake_case"))]
pub struct StatementResultsMetaWire {
    /// Number of nodes evaluated.
    pub num_nodes: usize,
    /// Number of periods evaluated.
    pub num_periods: usize,
    /// Evaluation time in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eval_time_ms: Option<u64>,
}

// ============================================================================
// Scenario Wire Types
// ============================================================================

/// Wire type for a scenario specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts_export", derive(TS))]
#[cfg_attr(
    feature = "ts_export",
    ts(
        export,
        export_to = "../finstack-ui/src/schemas/generated/ScenarioSpecWire.ts"
    )
)]
#[cfg_attr(feature = "ts_export", ts(rename_all = "snake_case"))]
pub struct ScenarioSpecWire {
    /// Scenario identifier.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Optional description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Operations to apply.
    pub operations: Vec<ScenarioOperationWire>,
}

/// Wire type for a scenario operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts_export", derive(TS))]
#[cfg_attr(
    feature = "ts_export",
    ts(
        export,
        export_to = "../finstack-ui/src/schemas/generated/ScenarioOperationWire.ts"
    )
)]
#[cfg_attr(feature = "ts_export", ts(rename_all = "snake_case"))]
pub struct ScenarioOperationWire {
    /// Operation type: "parallel_shift", "twist", "scale", "override", etc.
    pub operation_type: String,
    /// Target path (e.g., "discount_curves.USD", "fx_rates.EUR/USD").
    pub target: String,
    /// Operation value (interpretation depends on operation_type).
    pub value: f64,
    /// Optional parameters as key-value pairs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<BTreeMap<String, String>>,
}

// ============================================================================
// Error Wire Type
// ============================================================================

/// Wire type for error responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts_export", derive(TS))]
#[cfg_attr(
    feature = "ts_export",
    ts(
        export,
        export_to = "../finstack-ui/src/schemas/generated/ErrorWire.ts"
    )
)]
#[cfg_attr(feature = "ts_export", ts(rename_all = "snake_case"))]
pub struct ErrorWire {
    /// Error kind (InputError, ValidationError, CalibrationError, etc.).
    pub kind: String,
    /// Error message.
    pub message: String,
    /// Optional additional details.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<BTreeMap<String, String>>,
}
