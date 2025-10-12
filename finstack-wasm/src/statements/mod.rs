//! Financial statement modeling bindings for WASM.
//!
//! This module provides JavaScript/TypeScript bindings for the finstack-statements crate,
//! enabling financial statement modeling with deterministic evaluation, currency-safe
//! arithmetic, and support for forecasting methods, extensions, and dynamic metric registries.

pub mod builder;
pub mod evaluator;
pub mod extensions;
pub mod registry;
pub mod types;

// Re-export all public types for convenient access
pub use builder::JsModelBuilder;
pub use evaluator::{JsEvaluator, JsResults, JsResultsMeta};
pub use extensions::{
    JsCorkscrewExtension, JsCreditScorecardExtension, JsExtensionMetadata, JsExtensionRegistry,
    JsExtensionResult, JsExtensionStatus,
};
pub use registry::{JsMetricDefinition, JsMetricRegistry, JsRegistry, JsUnitType};
pub use types::{
    JsAmountOrScalar, JsCapitalStructureSpec, JsDebtInstrumentSpec, JsFinancialModelSpec,
    JsForecastMethod, JsForecastSpec, JsNodeSpec, JsNodeType, JsSeasonalMode,
};
