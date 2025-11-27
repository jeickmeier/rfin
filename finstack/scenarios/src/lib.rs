#![deny(unsafe_code)]
#![warn(missing_docs)]
#![deny(clippy::unwrap_used)]

//! Finstack Scenarios — Lightweight deterministic scenario capability.
//!
//! This crate provides a minimal, deterministic API for applying shocks to market data
//! and financial statement forecasts. It enables what-if analysis and stress testing
//! without requiring a full DSL parser.
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use finstack_scenarios::{ScenarioSpec, OperationSpec, CurveKind, ScenarioEngine, ExecutionContext};
//! use finstack_core::market_data::MarketContext;
//! use finstack_statements::FinancialModelSpec;
//! use time::macros::date;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mut market = MarketContext::new();
//! let mut model = FinancialModelSpec::new("test", vec![]);
//! let as_of = date!(2025-01-01);
//!
//! let scenario = ScenarioSpec {
//!     id: "stress_test".into(),
//!     name: Some("Q1 Stress Test".into()),
//!     description: None,
//!     operations: vec![
//!         OperationSpec::CurveParallelBp {
//!             curve_kind: CurveKind::Discount,
//!             curve_id: "USD_SOFR".into(),
//!             bp: 50.0,
//!         },
//!     ],
//!     priority: 0,
//! };
//!
//! let engine = ScenarioEngine::default();
//! let mut ctx = ExecutionContext {
//!     market: &mut market,
//!     model: &mut model,
//!     instruments: None,
//!     rate_bindings: None,
//!     calendar: None,
//!     as_of,
//! };
//!
//! let report = engine.apply(&scenario, &mut ctx)?;
//! println!("Applied {} operations", report.operations_applied);
//! # Ok(())
//! # }
//! ```

pub mod adapters;
pub mod engine;
pub mod error;
pub mod spec;
pub mod utils;

pub use engine::{ExecutionContext, ScenarioEngine};
pub use error::{Error, Result};
pub use spec::{
    Compounding, CurveKind, InstrumentType, OperationSpec, RateBindingSpec, ScenarioSpec,
    TenorMatchMode, VolSurfaceKind,
};
