#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(clippy::new_without_default)]
#![warn(clippy::float_cmp)]
#![cfg_attr(test, allow(clippy::float_cmp))]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
// Allow expect() in doc tests (they are test code)
#![doc(test(attr(allow(clippy::expect_used))))]

//! # Finstack Statements — Financial Statement Modeling Engine
//!
//! The `finstack-statements` crate enables users to build financial statement models
//! as directed graphs of metrics evaluated over discrete periods (monthly, quarterly, annually).
//!
//! ## Key Features
//!
//! - **Declarative modeling** with a rich DSL for formulas
//! - **Time-series forecasting** with deterministic and statistical methods
//! - **Capital structure integration** for debt/equity tracking
//! - **Dynamic metric registry** (no recompilation needed)
//! - **Currency-safe arithmetic** with explicit FX handling
//! - **Deterministic evaluation**
//! - **EBITDA normalization & adjustments** with audited add-backs and cap policies
//!
//! ## Quick Start
//!
//! ```rust
//! use finstack_statements::prelude::*;
//!
//! # fn main() -> Result<()> {
//! // Build a simple P&L model
//! let model = ModelBuilder::new("Acme Corp")
//!     .periods("2025Q1..Q4", Some("2025Q2"))?
//!     .value("revenue", &[
//!         (PeriodId::quarter(2025, 1), AmountOrScalar::scalar(10_000_000.0)),
//!         (PeriodId::quarter(2025, 2), AmountOrScalar::scalar(11_000_000.0)),
//!     ])
//!     .compute("cogs", "revenue * 0.6")?
//!     .compute("gross_profit", "revenue - cogs")?
//!     .build()?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Architecture
//!
//! The crate is organized into:
//!
//! - **types**: Wire types for serialization (`NodeSpec`, `FinancialModelSpec`)
//! - **builder**: Type-safe builder API with compile-time state enforcement
//! - **error**: Comprehensive error types with context
//! - **adjustments**: Normalization engine plus add-back specs/caps for adjusted EBITDA
//! - **dsl**: Formula DSL (parser, AST, compiler) for arithmetic, time-series, rolling and statistical functions
//! - **evaluator**: DAG-based evaluation with topological sort, precedence resolution, and capital structure integration
//! - **forecast**: Deterministic and statistical forecast methods (growth, seasonal, Monte Carlo)
//! - **registry**: Dynamic metric registry with namespace management and built-in `fin.*` metrics
//! - **extensions**: Corkscrew, credit scorecard, and custom plugin support

/// Normalization engine and add-back tracking for adjusted metrics.
pub mod adjustments;
/// Type-safe model builder API.
pub mod builder;
/// Debt and equity structure modeling.
pub mod capital_structure;
/// Formula DSL parsing and compilation.
pub mod dsl;
/// Error types for statement modeling.
pub mod error;
/// Evaluation engine for metric graphs.
pub mod evaluator;
/// Extension framework for custom logic.
pub mod extensions;
/// Forecast methods and time-series drivers.
pub mod forecast;
/// Convenient re-exports for common statement types.
pub mod prelude;
/// Metric registry and namespace management.
pub mod registry;
/// Core statement model types.
pub mod types;
/// Internal utilities (constants, formula helpers, graph traversal).
pub mod utils;

// Re-export core types at crate root for ergonomic imports
pub use error::{Error, Result};
pub use evaluator::NumericMode;
pub use types::{
    AmountOrScalar, CapitalStructureSpec, DebtInstrumentSpec, FinancialModelSpec, ForecastMethod,
    ForecastSpec, NodeId, NodeSpec, NodeType, NodeValueType, SeasonalMode,
};
