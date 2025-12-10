//! GenUI-facing wire types for schema generation.
//!
//! These structs intentionally mirror serde-friendly shapes suitable for
//! validation in the UI. When the `ts_export` feature is enabled, `ts-rs`
//! emits TypeScript definitions into the finstack-ui package for Zod wrapping.

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
        export_to = "../packages/finstack-ui/src/schemas/generated/CurvePointWire.ts"
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
        export_to = "../packages/finstack-ui/src/schemas/generated/DiscountCurveWire.ts"
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
        export_to = "../packages/finstack-ui/src/schemas/generated/MarketContextWire.ts"
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
    ts(
        export,
        export_to = "../packages/finstack-ui/src/schemas/generated/BondSpec.ts"
    )
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
        export_to = "../packages/finstack-ui/src/schemas/generated/ValuationResultWire.ts"
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

