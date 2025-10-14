//! Serializable scenario specifications (serde-stable).
//!
//! This module defines the surface area that callers use to describe shocks.
//! Types here are intentionally lightweight and focus on data rather than
//! behaviour so they can be shared across binaries, configuration files, and
//! JSON/YAML APIs.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Re-export [`InstrumentType`](finstack_valuations::pricer::InstrumentType) for
/// type-safe instrument shock operations.
pub use finstack_valuations::pricer::InstrumentType;

/// A complete scenario specification with metadata and ordered operations.
///
/// # Fields
/// - `id`: Stable identifier used for persistence and reporting.
/// - `name`: Optional display name for UI or logs.
/// - `description`: Optional text describing the intent of the scenario.
/// - `operations`: Ordered list of [`OperationSpec`] values to execute.
/// - `priority`: Used by [`ScenarioEngine::compose`](crate::engine::ScenarioEngine::compose)
///   to determine merge ordering (lower numbers run first).
///
/// # Examples
///
/// ```rust
/// use finstack_scenarios::{ScenarioSpec, OperationSpec, CurveKind};
///
/// let scenario = ScenarioSpec {
///     id: "stress_test".into(),
///     name: Some("Q1 Stress".into()),
///     description: Some("Rate shock scenario".into()),
///     operations: vec![
///         OperationSpec::CurveParallelBp {
///             curve_kind: CurveKind::Discount,
///             curve_id: "USD_SOFR".into(),
///             bp: 50.0,
///         },
///     ],
///     priority: 0,
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioSpec {
    /// Unique identifier for this scenario.
    pub id: String,

    /// Optional human-readable name.
    #[serde(default)]
    pub name: Option<String>,

    /// Optional description.
    #[serde(default)]
    pub description: Option<String>,

    /// List of operations to apply.
    pub operations: Vec<OperationSpec>,

    /// Priority for composition (lower value = higher priority).
    #[serde(default)]
    pub priority: i32,
}

/// Individual operation within a scenario.
///
/// Each variant represents a specific type of shock or adjustment
/// that can be applied to market data or statements.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum OperationSpec {
    /// FX rate percent shift.
    ///
    /// # Example
    /// ```rust
    /// use finstack_scenarios::OperationSpec;
    /// use finstack_core::currency::Currency;
    ///
    /// let op = OperationSpec::MarketFxPct {
    ///     base: Currency::USD,
    ///     quote: Currency::EUR,
    ///     pct: 5.0, // +5% means USD strengthens
    /// };
    /// ```
    MarketFxPct {
        base: finstack_core::currency::Currency,
        quote: finstack_core::currency::Currency,
        pct: f64,
    },

    /// Equity price percent shock.
    ///
    /// # Example
    /// ```rust
    /// use finstack_scenarios::OperationSpec;
    ///
    /// let op = OperationSpec::EquityPricePct {
    ///     ids: vec!["SPY".into(), "QQQ".into()],
    ///     pct: -10.0, // -10% price drop
    /// };
    /// ```
    EquityPricePct { ids: Vec<String>, pct: f64 },

    /// Instrument price shock by exact attribute match.
    ///
    /// # Example
    /// ```rust
    /// use finstack_scenarios::OperationSpec;
    /// use indexmap::IndexMap;
    ///
    /// let mut attrs = IndexMap::new();
    /// attrs.insert("sector".into(), "Energy".into());
    /// attrs.insert("rating".into(), "BBB".into());
    ///
    /// let op = OperationSpec::InstrumentPricePctByAttr {
    ///     attrs,
    ///     pct: -3.0,
    /// };
    /// ```
    InstrumentPricePctByAttr {
        attrs: IndexMap<String, String>,
        pct: f64,
    },

    /// Parallel shift to a curve (additive in basis points).
    ///
    /// # Example
    /// ```rust
    /// use finstack_scenarios::{OperationSpec, CurveKind};
    ///
    /// let op = OperationSpec::CurveParallelBp {
    ///     curve_kind: CurveKind::Discount,
    ///     curve_id: "USD_SOFR".into(),
    ///     bp: 50.0, // +50bp parallel shift
    /// };
    /// ```
    CurveParallelBp {
        curve_kind: CurveKind,
        curve_id: String,
        bp: f64,
    },

    /// Node-specific basis point shifts for curve shaping.
    ///
    /// # Example
    /// ```rust
    /// use finstack_scenarios::{OperationSpec, CurveKind, TenorMatchMode};
    ///
    /// let op = OperationSpec::CurveNodeBp {
    ///     curve_kind: CurveKind::Discount,
    ///     curve_id: "USD_SOFR".into(),
    ///     nodes: vec![
    ///         ("2Y".into(), 25.0),
    ///         ("10Y".into(), -10.0),
    ///     ],
    ///     match_mode: TenorMatchMode::Interpolate,
    /// };
    /// ```
    CurveNodeBp {
        curve_kind: CurveKind,
        curve_id: String,
        nodes: Vec<(String, f64)>,
        #[serde(default)]
        match_mode: TenorMatchMode,
    },

    /// Parallel shift to base correlation surface (absolute points).
    ///
    /// # Example
    /// ```rust
    /// use finstack_scenarios::OperationSpec;
    ///
    /// let op = OperationSpec::BaseCorrParallelPts {
    ///     surface_id: "CDX_IG".into(),
    ///     points: 0.05, // +5 percentage points
    /// };
    /// ```
    BaseCorrParallelPts { surface_id: String, points: f64 },

    /// Bucket-specific base correlation shifts.
    ///
    /// # Example
    /// ```rust
    /// use finstack_scenarios::OperationSpec;
    ///
    /// let op = OperationSpec::BaseCorrBucketPts {
    ///     surface_id: "CDX_IG".into(),
    ///     detachment_bps: Some(vec![300, 700]), // 3% and 7% detachment
    ///     maturities: None, // all maturities
    ///     points: 0.03,
    /// };
    /// ```
    BaseCorrBucketPts {
        surface_id: String,
        detachment_bps: Option<Vec<i32>>,
        maturities: Option<Vec<String>>,
        points: f64,
    },

    /// Parallel percent shift to volatility surface.
    ///
    /// # Example
    /// ```rust
    /// use finstack_scenarios::{OperationSpec, VolSurfaceKind};
    ///
    /// let op = OperationSpec::VolSurfaceParallelPct {
    ///     surface_kind: VolSurfaceKind::Equity,
    ///     surface_id: "SPX".into(),
    ///     pct: 10.0, // +10% volatility increase
    /// };
    /// ```
    VolSurfaceParallelPct {
        surface_kind: VolSurfaceKind,
        surface_id: String,
        pct: f64,
    },

    /// Bucketed volatility surface shock.
    ///
    /// # Example
    /// ```rust
    /// use finstack_scenarios::{OperationSpec, VolSurfaceKind};
    ///
    /// let op = OperationSpec::VolSurfaceBucketPct {
    ///     surface_kind: VolSurfaceKind::Equity,
    ///     surface_id: "SPX".into(),
    ///     tenors: Some(vec!["1M".into(), "3M".into()]),
    ///     strikes: Some(vec![90.0, 95.0, 100.0]),
    ///     pct: 15.0,
    /// };
    /// ```
    VolSurfaceBucketPct {
        surface_kind: VolSurfaceKind,
        surface_id: String,
        tenors: Option<Vec<String>>,
        strikes: Option<Vec<f64>>,
        pct: f64,
    },

    /// Statement forecast percent change.
    ///
    /// # Example
    /// ```rust
    /// use finstack_scenarios::OperationSpec;
    ///
    /// let op = OperationSpec::StmtForecastPercent {
    ///     node_id: "Revenue".into(),
    ///     pct: -5.0, // -5% revenue decrease
    /// };
    /// ```
    StmtForecastPercent { node_id: String, pct: f64 },

    /// Statement forecast value assignment.
    ///
    /// # Example
    /// ```rust
    /// use finstack_scenarios::OperationSpec;
    ///
    /// let op = OperationSpec::StmtForecastAssign {
    ///     node_id: "Revenue".into(),
    ///     value: 1_000_000.0,
    /// };
    /// ```
    StmtForecastAssign { node_id: String, value: f64 },

    /// Instrument spread shock by exact attribute match (FI only).
    ///
    /// # Example
    /// ```rust
    /// use finstack_scenarios::OperationSpec;
    /// use indexmap::IndexMap;
    ///
    /// let mut attrs = IndexMap::new();
    /// attrs.insert("rating".into(), "CCC".into());
    ///
    /// let op = OperationSpec::InstrumentSpreadBpByAttr {
    ///     attrs,
    ///     bp: 50.0,
    /// };
    /// ```
    InstrumentSpreadBpByAttr {
        attrs: IndexMap<String, String>,
        bp: f64,
    },

    /// Instrument price shock by type.
    ///
    /// # Example
    /// ```rust
    /// use finstack_scenarios::{OperationSpec, InstrumentType};
    ///
    /// let op = OperationSpec::InstrumentPricePctByType {
    ///     instrument_types: vec![InstrumentType::Bond, InstrumentType::Loan],
    ///     pct: -5.0,
    /// };
    /// ```
    InstrumentPricePctByType {
        instrument_types: Vec<InstrumentType>,
        pct: f64,
    },

    /// Instrument spread shock by type.
    ///
    /// # Example
    /// ```rust
    /// use finstack_scenarios::{OperationSpec, InstrumentType};
    ///
    /// let op = OperationSpec::InstrumentSpreadBpByType {
    ///     instrument_types: vec![InstrumentType::CDS],
    ///     bp: 100.0,
    /// };
    /// ```
    InstrumentSpreadBpByType {
        instrument_types: Vec<InstrumentType>,
        bp: f64,
    },

    /// Roll forward horizon by a period with carry/theta.
    ///
    /// # Example
    /// ```rust
    /// use finstack_scenarios::OperationSpec;
    ///
    /// let op = OperationSpec::TimeRollForward {
    ///     period: "1M".into(),
    ///     apply_shocks: true,
    /// };
    /// ```
    TimeRollForward {
        /// Period to roll forward (e.g., "1D", "1W", "1M", "1Y")
        period: String,
        /// Whether to apply market shocks after rolling
        #[serde(default = "default_true")]
        apply_shocks: bool,
    },
}

fn default_true() -> bool {
    true
}

/// Identifies which family of curve an operation targets.
///
/// These variants map to the market data collections exposed by
/// `finstack_core::market_data::MarketContext`.
///
/// # Examples
/// ```rust
/// use finstack_scenarios::CurveKind;
///
/// let kind = CurveKind::Discount;
/// assert_eq!(format!("{:?}", kind), "Discount");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CurveKind {
    /// Discount factor curve.
    Discount,
    /// Forward rate curve.
    Forecast,
    /// Credit hazard rate curve.
    Hazard,
    /// Inflation index curve.
    Inflation,
}

/// Identifies which category of volatility surface an operation targets.
///
/// Drives lookups into the relevant vol collections in market data.
///
/// # Examples
/// ```rust
/// use finstack_scenarios::VolSurfaceKind;
///
/// assert!(matches!(VolSurfaceKind::Swaption, VolSurfaceKind::Swaption));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VolSurfaceKind {
    /// Equity volatility surface.
    Equity,
    /// Credit volatility surface.
    Credit,
    /// Swaption volatility surface.
    Swaption,
}

/// Strategy for aligning requested tenor bumps with curve pillars.
///
/// # Examples
/// ```rust
/// use finstack_scenarios::TenorMatchMode;
///
/// let mode = TenorMatchMode::Interpolate;
/// assert_eq!(format!("{:?}", mode), "Interpolate");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TenorMatchMode {
    /// Match exact pillar only (errors if not found).
    Exact,
    /// Use key-rate bump at interpolated time.
    #[default]
    Interpolate,
}
