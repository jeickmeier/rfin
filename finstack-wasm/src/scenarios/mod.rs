//! Scenario capability bindings for WASM.
//!
//! This module provides JavaScript/TypeScript bindings for the finstack-scenarios crate,
//! enabling deterministic scenario analysis with stress testing and what-if capabilities.

pub mod engine;
pub mod enums;
pub mod reports;
pub mod spec;

// Re-export all public types for convenient access
pub use engine::{JsExecutionContext, JsScenarioEngine};
pub use enums::{JsCurveKind, JsTenorMatchMode, JsVolSurfaceKind};
pub use reports::{JsApplicationReport, JsRollForwardReport};
pub use spec::{JsOperationSpec, JsScenarioSpec};
