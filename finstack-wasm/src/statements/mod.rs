//! Financial statement modeling bindings for WASM.
//!
//! This module provides JavaScript/TypeScript bindings for the finstack-statements crate,
//! enabling financial statement modeling with deterministic evaluation, currency-safe
//! arithmetic, and support for forecasting methods, extensions, and dynamic metric registries.

pub mod adjustments;
pub mod analysis;
pub mod builder;
pub mod evaluator;
pub mod extensions;
pub mod registry;
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
pub use evaluator::{JsEvaluator, JsStatementResult, JsStatementResultMeta};
pub use extensions::{
    JsCorkscrewExtension, JsCreditScorecardExtension, JsExtensionMetadata, JsExtensionRegistry,
    JsExtensionResult, JsExtensionStatus,
};
pub use registry::{JsMetricDefinition, JsMetricRegistry, JsRegistry, JsUnitType};
pub use types::{
    JsAmountOrScalar, JsCapitalStructureSpec, JsDebtInstrumentSpec, JsFinancialModelSpec,
    JsForecastMethod, JsForecastSpec, JsNodeSpec, JsNodeType, JsSeasonalMode,
};
