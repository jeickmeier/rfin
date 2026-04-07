//! Financial statement modeling bindings for WASM.
//!
//! This module provides JavaScript/TypeScript bindings for the finstack-statements crate,
//! enabling financial statement modeling with deterministic evaluation, currency-safe
//! arithmetic, and support for forecasting methods, extensions, and dynamic metric registries.

pub mod adjustments;
pub mod analysis;
pub mod builder;
pub mod capital_structure;
pub mod dsl;
pub mod evaluator;
pub mod extensions;
pub mod forecast;
pub mod registry;
pub mod templates;
pub mod types;

// Re-export all public types for convenient access
pub use adjustments::{
    JsAdjustment, JsAppliedAdjustment, JsNormalizationConfig, JsNormalizationEngine,
    JsNormalizationResult,
};
pub use analysis::{
    all_dependencies, dependency_tree, dependents, direct_dependencies,
    render_dependency_tree_ascii, JsDependencyTree,
};
pub use builder::JsModelBuilder;
pub use capital_structure::{
    aggregate_instrument_cashflows, JsCapitalStructureCashflows, JsCashflowBreakdown,
    JsEcfSweepSpec, JsPikToggleSpec, JsWaterfallSpec,
};
pub use dsl::{compile_formula, parse_and_compile, parse_formula, JsStmtExpr};
pub use evaluator::{
    node_to_dated_schedule, JsEvaluator, JsMonteCarloConfig, JsMonteCarloResults,
    JsPercentileSeries, JsPeriodDateConvention, JsStatementResult, JsStatementResultMeta,
};
pub use extensions::{
    JsCorkscrewExtension, JsCreditScorecardExtension, JsExtensionMetadata, JsExtensionRegistry,
    JsExtensionResult, JsExtensionStatus,
};
pub use forecast::{
    apply_forecast, apply_override, curve_pct, forward_fill, growth_pct, lognormal_forecast,
    normal_forecast, seasonal_forecast, timeseries_forecast,
};
pub use registry::{JsMetricDefinition, JsMetricRegistry, JsRegistry, JsUnitType};
pub use templates::{
    JsFreeRentWindowSpec, JsLeaseGrowthConvention, JsLeaseSpec, JsManagementFeeBase,
    JsManagementFeeSpec, JsPropertyTemplateNodes, JsRenewalSpec, JsRentRollOutputNodes,
    JsRentStepSpec, JsSimpleLeaseSpec,
};
pub use types::{
    JsAmountOrScalar, JsCapitalStructureSpec, JsDebtInstrumentSpec, JsFinancialModelSpec,
    JsForecastMethod, JsForecastSpec, JsNodeSpec, JsNodeType, JsSeasonalMode,
};
