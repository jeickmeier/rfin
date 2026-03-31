//! Scenario capability bindings for WASM.
//!
//! This module provides JavaScript/TypeScript bindings for the finstack-scenarios crate,
//! enabling deterministic scenario analysis with stress testing and what-if capabilities.

pub mod engine;
pub mod enums;
pub mod reports;
pub mod spec;
pub mod templates;
pub mod utils;

// Re-export all public types for convenient access
pub use engine::{JsExecutionContext, JsScenarioEngine};
pub use enums::{JsCompounding, JsCurveKind, JsTenorMatchMode, JsTimeRollMode, JsVolSurfaceKind};
pub use reports::{JsApplicationReport, JsRollForwardReport};
pub use spec::{JsOperationSpec, JsRateBindingSpec, JsScenarioSpec};
pub use templates::{
    JsAssetClass, JsScenarioSpecBuilder, JsSeverity, JsTemplateMetadata, JsTemplateRegistry,
};
pub use utils::{
    calculate_interpolation_weights, calculate_interpolation_weights_with_info,
    parse_period_to_days, parse_tenor_to_years,
};
