//! Serializable scenario specifications (serde-stable).
//!
//! This module defines the data contract that callers use to describe market,
//! instrument, statement, and time-roll shocks. Types here are intentionally
//! lightweight and focus on configuration rather than behaviour so they can be
//! shared across binaries, configuration files, and JSON/YAML APIs.
//!
//! Start with [`ScenarioSpec`] and [`OperationSpec`]. Use [`RateBindingSpec`]
//! when statement nodes should follow curve-driven rates, and use
//! [`HierarchyTarget`] together with [`ResolutionMode`](finstack_core::market_data::hierarchy::ResolutionMode)
//! for hierarchy-expanded shocks.
//!
//! # Conventions
//!
//! - Percent shocks use percentage-point inputs (`5.0 = +5%`).
//! - Curve and spread shocks quoted in `bp` use additive basis points.
//! - Time-roll operations distinguish business-day-aware, calendar-day, and
//!   approximate day-count semantics via [`TimeRollMode`].
//! - Hierarchy-targeted operations expand to direct market-data identifiers at
//!   execution time; [`ScenarioSpec::resolution_mode`] determines whether
//!   overlapping hierarchy matches accumulate or collapse to the most specific
//!   match.

use finstack_core::market_data::hierarchy::ResolutionMode;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Re-export [`HierarchyTarget`] for hierarchy-targeted operations.
pub use finstack_core::market_data::hierarchy::HierarchyTarget;

/// Re-export [`InstrumentType`] for
/// type-safe instrument shock operations.
pub use finstack_valuations::pricer::InstrumentType;

/// Re-export [`NodeId`] for statement node identification.
pub use finstack_statements::NodeId;

/// A complete scenario specification with metadata and ordered operations.
///
/// # Fields
/// - `id`: Stable identifier used for persistence and reporting.
/// - `name`: Optional display name for UI or logs.
/// - `description`: Optional text describing the intent of the scenario.
/// - `operations`: Ordered list of [`OperationSpec`] values to execute.
/// - `priority`: Used by [`ScenarioEngine::compose`](crate::engine::ScenarioEngine::compose)
///   to determine merge ordering (lower numbers run first).
/// - `resolution_mode`: Controls how hierarchy-targeted shocks at multiple tree
///   levels combine for a single curve. Defaults to [`ResolutionMode::MostSpecificWins`].
///
/// # Examples
///
/// ```rust
/// use finstack_scenarios::{ScenarioSpec, OperationSpec, CurveKind};
/// use finstack_core::market_data::hierarchy::ResolutionMode;
///
/// let scenario = ScenarioSpec {
///     id: "stress_test".into(),
///     name: Some("Q1 Stress".into()),
///     description: Some("Rate shock scenario".into()),
///     operations: vec![
///         OperationSpec::CurveParallelBp {
///             curve_kind: CurveKind::Discount,
///             curve_id: "USD_SOFR".into(),
///             discount_curve_id: None,
///             bp: 50.0,
///         },
///     ],
///     priority: 0,
///     resolution_mode: ResolutionMode::default(),
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

    /// Resolution mode for hierarchy-targeted operations.
    ///
    /// Only relevant when operations use hierarchy targeting.
    /// Default: [`ResolutionMode::MostSpecificWins`].
    #[serde(default)]
    pub resolution_mode: ResolutionMode,
}

/// Individual operation within a scenario.
///
/// Each variant represents a specific type of shock or adjustment that can be
/// applied to market data, instruments, statements, or the valuation horizon.
/// Units are encoded in the variant name and field docs:
/// - `Pct` fields use percentage points (`5.0 = +5%`)
/// - `Bp` fields use additive basis points
/// - `Pts` fields use absolute correlation or volatility points in decimal form
///
/// Hierarchy-targeted variants are resolved into direct identifiers during
/// [`crate::engine::ScenarioEngine::apply`] using the market hierarchy attached
/// to the execution context.
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
        /// Base currency in the FX pair.
        base: finstack_core::currency::Currency,
        /// Quote currency in the FX pair.
        quote: finstack_core::currency::Currency,
        /// Percentage change (positive means base strengthens).
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
    EquityPricePct {
        /// Equity identifiers to shock.
        ids: Vec<String>,
        /// Percentage price change.
        pct: f64,
    },

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
    ///
    /// Matching semantics:
    /// - Only instrument metadata key/value pairs are considered (the `meta` map on
    ///   `Attributes`); tag sets are ignored.
    /// - Attribute keys/values compared case-insensitively
    /// - AND semantics (all provided pairs must match metadata)
    /// - An empty attribute map matches every instrument (wildcard)
    InstrumentPricePctByAttr {
        /// Attributes to match. Matching uses AND semantics with
        /// case-insensitive key/value comparison.
        attrs: IndexMap<String, String>,
        /// Percentage price change.
        pct: f64,
    },

    /// Parallel shift to a curve (additive in basis points).
    ///
    /// For rate curves (Discount, Forward, Hazard), `bp` uses the standard
    /// convention where 1 bp = 0.0001 in fractional rate space. For
    /// [`CurveKind::VolIndex`], `bp` is scaled differently: 100 bp = 1.0 index
    /// point (i.e., `bp / 100`). See the vol-index branch in the curve adapter
    /// for details.
    ///
    /// # Example
    /// ```rust
    /// use finstack_scenarios::{OperationSpec, CurveKind};
    ///
    /// let op = OperationSpec::CurveParallelBp {
    ///     curve_kind: CurveKind::Discount,
    ///     curve_id: "USD_SOFR".into(),
    ///     discount_curve_id: None,
    ///     bp: 50.0, // +50bp parallel shift
    /// };
    /// ```
    CurveParallelBp {
        /// Type of curve (Discount, Forward, Hazard, etc.).
        curve_kind: CurveKind,
        /// Curve identifier.
        curve_id: String,
        /// Optional explicit discount curve identifier when an operation must pair
        /// `curve_id` with a discounting curve (some forward/hazard bumps need one).
        /// If `None`, adapters apply engine default resolution; set this when multiple
        /// curves could match or you need deterministic selection.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        discount_curve_id: Option<String>,
        /// Basis point shift (additive).
        bp: f64,
    },

    /// Node-specific basis point shifts for curve shaping.
    ///
    /// For rate curves (Discount, Forward, Hazard), `bp` values use the standard
    /// convention where 1 bp = 0.0001 in fractional rate space. For
    /// [`CurveKind::VolIndex`], `bp` is scaled differently: 100 bp = 1.0 index
    /// point (i.e., `bp / 100`). See the vol-index branch in the curve adapter
    /// for details.
    ///
    /// # Example
    /// ```rust
    /// use finstack_scenarios::{OperationSpec, CurveKind, TenorMatchMode};
    ///
    /// let op = OperationSpec::CurveNodeBp {
    ///     curve_kind: CurveKind::Discount,
    ///     curve_id: "USD_SOFR".into(),
    ///     discount_curve_id: None,
    ///     nodes: vec![
    ///         ("2Y".into(), 25.0),
    ///         ("10Y".into(), -10.0),
    ///     ],
    ///     match_mode: TenorMatchMode::Interpolate,
    /// };
    /// ```
    CurveNodeBp {
        /// Type of curve (Discount, Forward, Hazard, etc.).
        curve_kind: CurveKind,
        /// Curve identifier.
        curve_id: String,
        /// Optional explicit discount curve identifier when an operation must pair
        /// `curve_id` with a discounting curve (some forward/hazard bumps need one).
        /// If `None`, adapters apply engine default resolution; set this when multiple
        /// curves could match or you need deterministic selection.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        discount_curve_id: Option<String>,
        /// Vector of (tenor, basis_point_shift) pairs.
        nodes: Vec<(String, f64)>,
        /// How to handle tenors not in the curve.
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
    BaseCorrParallelPts {
        /// Surface identifier.
        surface_id: String,
        /// Absolute shift in correlation points.
        points: f64,
    },

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
        /// Surface identifier.
        surface_id: String,
        /// Optional detachment points in basis points (e.g., 300 for 3%).
        detachment_bps: Option<Vec<i32>>,
        /// Optional maturity strings (e.g., "5Y").
        maturities: Option<Vec<String>>,
        /// Absolute shift in correlation points.
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
        /// Type of volatility surface (Equity, FX, IR, etc.).
        surface_kind: VolSurfaceKind,
        /// Surface identifier.
        surface_id: String,
        /// Percentage change in volatility.
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
        /// Type of volatility surface (Equity, FX, IR, etc.).
        surface_kind: VolSurfaceKind,
        /// Surface identifier.
        surface_id: String,
        /// Optional tenor strings (e.g., "1M", "3M").
        tenors: Option<Vec<String>>,
        /// Optional strike levels.
        strikes: Option<Vec<f64>>,
        /// Percentage change in volatility.
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
    StmtForecastPercent {
        /// Statement node identifier.
        node_id: NodeId,
        /// Percentage change to apply.
        pct: f64,
    },

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
    StmtForecastAssign {
        /// Statement node identifier.
        node_id: NodeId,
        /// Absolute value to assign.
        value: f64,
    },

    /// Bind a statement rate node to a market curve for this scenario.
    ///
    /// The rate at the specified tenor is extracted from the curve and applied
    /// to the node, overriding any static value. The binding is also registered
    /// in the execution context so it persists across subsequent time rolls.
    ///
    /// # Example
    /// ```rust
    /// use finstack_scenarios::{OperationSpec, RateBindingSpec};
    /// use finstack_scenarios::spec::Compounding;
    ///
    /// let op = OperationSpec::RateBinding {
    ///     binding: RateBindingSpec {
    ///         node_id: "InterestExpense".into(),
    ///         curve_id: "USD-OIS".into(),
    ///         tenor: "1Y".into(),
    ///         compounding: Compounding::Annual,
    ///         day_count: None,
    ///     },
    /// };
    /// ```
    RateBinding {
        /// The rate binding specification to apply.
        binding: RateBindingSpec,
    },

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
    ///
    /// Matching semantics:
    /// - Only instrument metadata key/value pairs are considered (the `meta` map on
    ///   `Attributes`); tag sets are ignored.
    /// - Attribute keys/values compared case-insensitively
    /// - AND semantics (all provided pairs must match metadata)
    /// - An empty attribute map matches every instrument (wildcard)
    InstrumentSpreadBpByAttr {
        /// Attributes to match. Matching uses AND semantics with
        /// case-insensitive key/value comparison.
        attrs: IndexMap<String, String>,
        /// Basis point shift to apply.
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
        /// Instrument types to shock.
        instrument_types: Vec<InstrumentType>,
        /// Percentage price change.
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
        /// Instrument types to shock.
        instrument_types: Vec<InstrumentType>,
        /// Basis point shift to apply.
        bp: f64,
    },

    // ========================================================================
    // Structured Credit Correlation Operations
    // ========================================================================
    /// Shock asset correlation for structured credit instruments.
    ///
    /// Applies a parallel shift to asset correlation parameters. For flat
    /// correlation structures, this bumps the single asset correlation. For
    /// sectored structures, it bumps both intra-sector and inter-sector
    /// correlations (inter-sector at half the rate).
    ///
    /// # Arguments
    /// - `delta_pts`: Additive shock in correlation points (e.g., 0.05 for +5%)
    ///
    /// # Clamping
    /// Asset correlation is clamped to [0, 0.99] after the shock.
    ///
    /// # Example
    /// ```rust
    /// use finstack_scenarios::OperationSpec;
    ///
    /// let op = OperationSpec::AssetCorrelationPts {
    ///     delta_pts: 0.05, // +5 percentage points
    /// };
    /// ```
    AssetCorrelationPts {
        /// Additive shock in correlation points.
        delta_pts: f64,
    },

    /// Shock prepay-default correlation for structured credit instruments.
    ///
    /// Applies a parallel shift to the correlation between prepayment and
    /// default rates. This is typically negative (higher prepayment = lower
    /// defaults due to selection effects).
    ///
    /// # Arguments
    /// - `delta_pts`: Additive shock in correlation points
    ///
    /// # Clamping
    /// Prepay-default correlation is clamped to [-0.99, 0.99].
    ///
    /// # Example
    /// ```rust
    /// use finstack_scenarios::OperationSpec;
    ///
    /// let op = OperationSpec::PrepayDefaultCorrelationPts {
    ///     delta_pts: 0.10, // +10 percentage points (less negative)
    /// };
    /// ```
    PrepayDefaultCorrelationPts {
        /// Additive shock in correlation points.
        delta_pts: f64,
    },

    /// Shock recovery-default correlation for structured credit instruments.
    ///
    /// Applies a shock to the correlation between recovery rates and defaults.
    /// Negative correlation implies recoveries decline when defaults rise
    /// (common in economic stress scenarios).
    ///
    /// # Arguments
    /// - `delta_pts`: Additive shock in correlation points
    ///
    /// # Example
    /// ```rust
    /// use finstack_scenarios::OperationSpec;
    ///
    /// let op = OperationSpec::RecoveryCorrelationPts {
    ///     delta_pts: -0.10, // -10 percentage points (more negative)
    /// };
    /// ```
    RecoveryCorrelationPts {
        /// Additive shock in correlation points.
        delta_pts: f64,
    },

    /// Shock prepayment factor loading (sensitivity to systematic factors).
    ///
    /// Bumps the factor loading that determines how much prepayment rates
    /// move with systematic factors. Higher factor loading means prepayment
    /// is more correlated across the pool.
    ///
    /// # Arguments
    /// - `delta_pts`: Additive shock to factor loading
    ///
    /// # Clamping
    /// Factor loading is clamped to [0, 1.0].
    ///
    /// # Example
    /// ```rust
    /// use finstack_scenarios::OperationSpec;
    ///
    /// let op = OperationSpec::PrepayFactorLoadingPts {
    ///     delta_pts: 0.05, // +5 percentage points
    /// };
    /// ```
    PrepayFactorLoadingPts {
        /// Additive shock to factor loading.
        delta_pts: f64,
    },

    // ========================================================================
    // Hierarchy-targeted operations (expanded to direct ops at execution time)
    // ========================================================================
    /// Hierarchy-targeted parallel curve shift.
    ///
    /// During [`crate::engine::ScenarioEngine::apply`], the engine resolves
    /// `target` against the market-data hierarchy and expands this into one
    /// [`OperationSpec::CurveParallelBp`] per matched curve. When multiple
    /// hierarchy operations reach the same curve, [`ScenarioSpec::resolution_mode`]
    /// determines whether they accumulate or only the most specific match survives.
    HierarchyCurveParallelBp {
        /// Type of curve (Discount, Forward, Hazard, etc.).
        curve_kind: CurveKind,
        /// Hierarchy target to resolve to curves.
        target: HierarchyTarget,
        /// Basis point shift (additive).
        bp: f64,
    },

    /// Hierarchy-targeted vol-surface parallel shift.
    ///
    /// Resolution follows the same hierarchy-expansion rules as
    /// [`OperationSpec::HierarchyCurveParallelBp`], but targets volatility
    /// surfaces of the requested [`VolSurfaceKind`].
    HierarchyVolSurfaceParallelPct {
        /// Type of volatility surface.
        surface_kind: VolSurfaceKind,
        /// Hierarchy target to resolve to surfaces.
        target: HierarchyTarget,
        /// Percentage change in volatility.
        pct: f64,
    },

    /// Hierarchy-targeted equity price shift.
    ///
    /// Resolution expands the hierarchy target into one direct equity-price
    /// shock per matched identifier. Percent inputs use percentage points.
    HierarchyEquityPricePct {
        /// Hierarchy target to resolve to equity IDs.
        target: HierarchyTarget,
        /// Percentage price change.
        pct: f64,
    },

    /// Hierarchy-targeted base-correlation parallel shift.
    ///
    /// Resolution expands the hierarchy target into one direct
    /// [`OperationSpec::BaseCorrParallelPts`] per matched surface.
    HierarchyBaseCorrParallelPts {
        /// Hierarchy target to resolve to surfaces.
        target: HierarchyTarget,
        /// Absolute shift in correlation points.
        points: f64,
    },

    /// Roll the valuation horizon forward before any later scenario operations run.
    ///
    /// `period` uses tenor-style strings such as `"1D"`, `"1W"`, `"1M"`, and
    /// `"1Y"`. [`TimeRollMode`] controls whether the roll uses business-day-aware
    /// date arithmetic, pure calendar arithmetic, or fixed approximate day counts.
    ///
    /// This operation is handled in phase 0 of
    /// [`crate::engine::ScenarioEngine::apply`], before market or statement
    /// shocks. If `apply_shocks` is `false`, the engine stops after the time roll
    /// and returns an [`crate::engine::ApplicationReport`] without applying the
    /// remaining operations in the scenario.
    ///
    /// # Example
    /// ```rust
    /// use finstack_scenarios::{OperationSpec, TimeRollMode};
    ///
    /// let op = OperationSpec::TimeRollForward {
    ///     period: "1M".into(),
    ///     apply_shocks: true,
    ///     roll_mode: TimeRollMode::BusinessDays,
    /// };
    /// ```
    TimeRollForward {
        /// Period to roll forward (e.g., "1D", "1W", "1M", "1Y")
        period: String,
        /// Whether to continue applying the remaining scenario operations after the roll.
        ///
        /// When `false`, [`crate::engine::ScenarioEngine::apply`] returns
        /// immediately after the roll-forward step.
        #[serde(default = "default_true")]
        apply_shocks: bool,
        /// Roll mode controlling calendar vs business-day behaviour.
        ///
        /// Defaults to `BusinessDays`, which respects calendars when provided.
        #[serde(default)]
        roll_mode: TimeRollMode,
    },
}

fn default_true() -> bool {
    true
}

/// Identifies which family of curve an operation targets.
///
/// These variants map to the market data collections exposed by
/// `finstack_core::market_data::context::MarketContext`.
/// They also determine which quoting and interpolation conventions apply when
/// downstream helpers extract rates or apply node shocks.
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
    #[serde(rename = "forward", alias = "forecast")]
    Forward,
    /// Credit Par CDS curve (bumping spreads)
    #[serde(rename = "par_cds")]
    ParCDS,
    /// Inflation index curve.
    Inflation,
    /// Commodity forward curve.
    Commodity,
    /// Volatility index curve (VIX, VSTOXX, etc.).
    #[serde(rename = "vol_index")]
    VolIndex,
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
/// [`TenorMatchMode::Exact`] requires the requested tenor to coincide with an
/// existing pillar. [`TenorMatchMode::Interpolate`] instead distributes the
/// bump across adjacent knots using the interpolation policy implemented by the
/// adapter.
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

/// Controls how time roll-forward periods are interpreted.
///
/// Use [`TimeRollMode::BusinessDays`] when the scenario should respect holiday
/// calendars and business-day adjustment rules. Use
/// [`TimeRollMode::CalendarDays`] when the tenor should be added without a
/// business-day adjustment. [`TimeRollMode::Approximate`] uses fixed day-count
/// approximations aligned with [`crate::utils::parse_period_to_days`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TimeRollMode {
    /// Calendar-aware roll that adjusts to business days when a calendar is supplied.
    #[default]
    BusinessDays,
    /// Pure calendar-day addition (no business-day adjustment even if a calendar exists).
    CalendarDays,
    /// Explicit approximate mode using fixed day counts (legacy 30/365 semantics).
    Approximate,
}

/// Configuration for rate binding between curves and statement nodes.
///
/// Specifies how to extract a rate from a market curve and link it to a
/// statement forecast node. This enables automatic propagation of market
/// rate shocks to financial statement projections.
///
/// The extracted rate is written into the statement model as a decimal scalar
/// (for example `0.0525` for 5.25%). `compounding` controls the output quote
/// convention, while `day_count` optionally overrides the curve's native day
/// count when converting `tenor` into a year fraction.
///
/// Supported `day_count` override strings are the aliases accepted by
/// [`crate::utils::parse_day_count_override`], including `act/360`,
/// `act/365f`, `act/act`, `30/360`, and `30e/360`.
///
/// # Examples
/// ```rust
/// use finstack_scenarios::spec::RateBindingSpec;
/// use finstack_scenarios::spec::Compounding;
///
/// let binding = RateBindingSpec {
///     node_id: "InterestRate".into(),
///     curve_id: "USD_SOFR".into(),
///     tenor: "1Y".into(),
///     compounding: Compounding::Continuous,
///     day_count: None, // Use curve's day count
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RateBindingSpec {
    /// Statement node ID to receive the rate.
    pub node_id: NodeId,

    /// Curve ID to extract rate from.
    pub curve_id: String,

    /// Tenor for rate extraction (e.g., "3M", "1Y").
    pub tenor: String,

    /// Compounding convention for rate conversion.
    #[serde(default)]
    pub compounding: Compounding,

    /// Day count convention override. If None, uses curve's convention.
    #[serde(default)]
    pub day_count: Option<String>,
}

impl RateBindingSpec {
    /// Build a binding from the legacy `(node_id, curve_id)` map.
    ///
    /// Uses a 1Y tenor with continuous compounding and no day-count override.
    ///
    /// # Arguments
    ///
    /// - `node_id`: Statement node that should receive the extracted rate.
    /// - `curve_id`: Curve identifier to query during scenario application.
    ///
    /// # Returns
    ///
    /// A [`RateBindingSpec`] using the legacy defaults:
    /// - tenor = `1Y`
    /// - compounding = [`Compounding::Continuous`]
    /// - day count override = `None`
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_scenarios::{Compounding, RateBindingSpec};
    ///
    /// let binding = RateBindingSpec::from_legacy("InterestRate", "USD_SOFR");
    /// assert_eq!(binding.tenor, "1Y");
    /// assert_eq!(binding.compounding, Compounding::Continuous);
    /// assert!(binding.day_count.is_none());
    /// ```
    pub fn from_legacy(node_id: impl Into<NodeId>, curve_id: impl Into<String>) -> Self {
        Self {
            node_id: node_id.into(),
            curve_id: curve_id.into(),
            tenor: "1Y".to_string(),
            compounding: Compounding::Continuous,
            day_count: None,
        }
    }

    /// Convert a legacy `(node_id, curve_id)` map into detailed binding specs.
    ///
    /// # Arguments
    ///
    /// - `legacy`: Mapping from statement node identifiers to curve identifiers.
    ///
    /// # Returns
    ///
    /// An [`IndexMap`] containing one [`RateBindingSpec`] per legacy entry, in
    /// insertion order.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use indexmap::IndexMap;
    /// use finstack_scenarios::RateBindingSpec;
    ///
    /// let mut legacy = IndexMap::new();
    /// legacy.insert("InterestRate".to_string(), "USD_SOFR".to_string());
    ///
    /// let bindings = RateBindingSpec::map_from_legacy(legacy);
    /// assert_eq!(bindings["InterestRate"].curve_id, "USD_SOFR");
    /// ```
    pub fn map_from_legacy(legacy: IndexMap<String, String>) -> IndexMap<NodeId, RateBindingSpec> {
        legacy
            .into_iter()
            .map(|(node_id, curve_id)| {
                let node_id = NodeId::from(node_id);
                let spec = RateBindingSpec::from_legacy(node_id.clone(), curve_id);
                (node_id, spec)
            })
            .collect()
    }
}

/// Compounding convention for rate conversions.
///
/// Used when extracting rates from curves to convert between
/// different quoting conventions (for example from continuous zeros to annual
/// or simple statement rates). The output remains a decimal annualized rate;
/// only the compounding basis changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Compounding {
    /// Simple interest (no compounding).
    Simple,
    /// Continuous compounding (default).
    #[default]
    Continuous,
    /// Annual compounding (1 per year).
    Annual,
    /// Semi-annual compounding (2 per year).
    SemiAnnual,
    /// Quarterly compounding (4 per year).
    Quarterly,
    /// Monthly compounding (12 per year).
    Monthly,
}

impl ScenarioSpec {
    /// Validate the scenario specification for consistency.
    ///
    /// Checks for:
    /// - Non-empty ID
    /// - Valid operations (recursively)
    ///
    /// # Returns
    ///
    /// `Ok(())` when the scenario has a non-empty identifier, contains at most
    /// one [`OperationSpec::TimeRollForward`], and every contained operation
    /// passes [`OperationSpec::validate`].
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::Error::Validation`] if the scenario ID is empty,
    /// more than one time-roll operation is present, or any contained operation
    /// is invalid.
    pub fn validate(&self) -> crate::error::Result<()> {
        if self.id.trim().is_empty() {
            return Err(crate::error::Error::Validation(
                "Scenario ID cannot be empty".into(),
            ));
        }
        let time_roll_count = self
            .operations
            .iter()
            .filter(|op| matches!(op, OperationSpec::TimeRollForward { .. }))
            .count();
        if time_roll_count > 1 {
            return Err(crate::error::Error::Validation(format!(
                "Scenario contains {} TimeRollForward operations; at most one is allowed",
                time_roll_count
            )));
        }
        for (i, op) in self.operations.iter().enumerate() {
            op.validate()
                .map_err(|e| crate::error::Error::Validation(format!("Operation {}: {}", i, e)))?;
        }
        Ok(())
    }
}

impl OperationSpec {
    /// Validate the operation specification.
    ///
    /// Checks for:
    /// - Non-NaN numeric values
    /// - Non-empty identifiers
    /// - Logical consistency (e.g. valid currency pairs)
    ///
    /// # Returns
    ///
    /// `Ok(())` when the operation's identifiers, numeric inputs, and variant-
    /// specific invariants are valid.
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::Error::Validation`] if any identifier is empty,
    /// any numeric field is non-finite, a percentage shock is less than or equal
    /// to `-100%`, or the variant violates its own structural rules.
    pub fn validate(&self) -> crate::error::Result<()> {
        match self {
            OperationSpec::MarketFxPct { base, quote, pct } => {
                if base == quote {
                    return Err(crate::error::Error::Validation(
                        "Base and quote currencies must be different".into(),
                    ));
                }
                check_finite(*pct, "pct")?;
            }
            OperationSpec::EquityPricePct { ids, pct } => {
                if ids.is_empty() {
                    return Err(crate::error::Error::Validation(
                        "Equity IDs cannot be empty".into(),
                    ));
                }
                check_finite(*pct, "pct")?;
                check_pct_floor(*pct, "pct")?;
            }
            OperationSpec::InstrumentPricePctByAttr { pct, .. } => {
                check_finite(*pct, "pct")?;
                check_pct_floor(*pct, "pct")?;
            }
            OperationSpec::CurveParallelBp {
                curve_id,
                discount_curve_id,
                bp,
                ..
            } => {
                check_id(curve_id, "curve_id")?;
                if let Some(discount_curve_id) = discount_curve_id {
                    check_id(discount_curve_id, "discount_curve_id")?;
                }
                check_finite(*bp, "bp")?;
            }
            OperationSpec::CurveNodeBp {
                curve_id,
                discount_curve_id,
                nodes,
                ..
            } => {
                check_id(curve_id, "curve_id")?;
                if let Some(discount_curve_id) = discount_curve_id {
                    check_id(discount_curve_id, "discount_curve_id")?;
                }
                if nodes.is_empty() {
                    return Err(crate::error::Error::Validation(
                        "Curve nodes cannot be empty".into(),
                    ));
                }
                for (tenor, bp) in nodes {
                    if tenor.trim().is_empty() {
                        return Err(crate::error::Error::Validation(
                            "Curve node tenor cannot be empty".into(),
                        ));
                    }
                    check_finite(*bp, "bp")?;
                }
            }
            OperationSpec::BaseCorrParallelPts { surface_id, points } => {
                check_id(surface_id, "surface_id")?;
                check_finite(*points, "points")?;
            }
            OperationSpec::BaseCorrBucketPts {
                surface_id, points, ..
            } => {
                check_id(surface_id, "surface_id")?;
                check_finite(*points, "points")?;
            }
            OperationSpec::VolSurfaceParallelPct {
                surface_id, pct, ..
            } => {
                check_id(surface_id, "surface_id")?;
                check_finite(*pct, "pct")?;
                check_pct_floor(*pct, "pct")?;
            }
            OperationSpec::VolSurfaceBucketPct {
                surface_id, pct, ..
            } => {
                check_id(surface_id, "surface_id")?;
                check_finite(*pct, "pct")?;
                check_pct_floor(*pct, "pct")?;
            }
            OperationSpec::StmtForecastPercent { node_id, pct } => {
                check_id(node_id.as_str(), "node_id")?;
                check_finite(*pct, "pct")?;
            }
            OperationSpec::StmtForecastAssign { node_id, value } => {
                check_id(node_id.as_str(), "node_id")?;
                check_finite(*value, "value")?;
            }
            OperationSpec::RateBinding { binding } => {
                check_id(binding.node_id.as_str(), "node_id")?;
                check_id(&binding.curve_id, "curve_id")?;
                if binding.tenor.trim().is_empty() {
                    return Err(crate::error::Error::Validation(
                        "RateBinding tenor cannot be empty".into(),
                    ));
                }
            }
            OperationSpec::InstrumentSpreadBpByAttr { bp, .. } => {
                check_finite(*bp, "bp")?;
            }
            OperationSpec::InstrumentPricePctByType { pct, .. } => {
                check_finite(*pct, "pct")?;
                check_pct_floor(*pct, "pct")?;
            }
            OperationSpec::InstrumentSpreadBpByType { bp, .. } => {
                check_finite(*bp, "bp")?;
            }
            OperationSpec::AssetCorrelationPts { delta_pts } => {
                check_finite(*delta_pts, "delta_pts")?;
            }
            OperationSpec::PrepayDefaultCorrelationPts { delta_pts } => {
                check_finite(*delta_pts, "delta_pts")?;
            }
            OperationSpec::RecoveryCorrelationPts { delta_pts } => {
                check_finite(*delta_pts, "delta_pts")?;
            }
            OperationSpec::PrepayFactorLoadingPts { delta_pts } => {
                check_finite(*delta_pts, "delta_pts")?;
            }
            OperationSpec::HierarchyCurveParallelBp { target, bp, .. } => {
                if target.path.is_empty() {
                    return Err(crate::error::Error::Validation(
                        "Hierarchy target path cannot be empty".into(),
                    ));
                }
                check_finite(*bp, "bp")?;
            }
            OperationSpec::HierarchyVolSurfaceParallelPct { target, pct, .. } => {
                if target.path.is_empty() {
                    return Err(crate::error::Error::Validation(
                        "Hierarchy target path cannot be empty".into(),
                    ));
                }
                check_finite(*pct, "pct")?;
                check_pct_floor(*pct, "pct")?;
            }
            OperationSpec::HierarchyEquityPricePct { target, pct } => {
                if target.path.is_empty() {
                    return Err(crate::error::Error::Validation(
                        "Hierarchy target path cannot be empty".into(),
                    ));
                }
                check_finite(*pct, "pct")?;
                check_pct_floor(*pct, "pct")?;
            }
            OperationSpec::HierarchyBaseCorrParallelPts { target, points } => {
                if target.path.is_empty() {
                    return Err(crate::error::Error::Validation(
                        "Hierarchy target path cannot be empty".into(),
                    ));
                }
                check_finite(*points, "points")?;
            }
            OperationSpec::TimeRollForward { period, .. } => {
                if period.trim().is_empty() {
                    return Err(crate::error::Error::Validation(
                        "Time roll period cannot be empty".into(),
                    ));
                }
            }
        }
        Ok(())
    }
}

fn check_finite(val: f64, name: &str) -> crate::error::Result<()> {
    if !val.is_finite() {
        Err(crate::error::Error::Validation(format!(
            "Value '{}' must be finite",
            name
        )))
    } else {
        Ok(())
    }
}

fn check_id(id: &str, name: &str) -> crate::error::Result<()> {
    if id.trim().is_empty() {
        Err(crate::error::Error::Validation(format!(
            "Identifier '{}' cannot be empty",
            name
        )))
    } else {
        Ok(())
    }
}

fn check_pct_floor(val: f64, name: &str) -> crate::error::Result<()> {
    if val <= -100.0 {
        Err(crate::error::Error::Validation(format!(
            "Percentage '{}' must be greater than -100% (got {:.1}%)",
            name, val
        )))
    } else {
        Ok(())
    }
}
