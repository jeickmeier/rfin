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
//!
//! ## Implementation Status
//!
//! ### Phase 1: Foundation ✅
//! - ✅ Core wire types (`NodeSpec`, `NodeType`, `AmountOrScalar`)
//! - ✅ Type-state builder pattern (`ModelBuilder<NeedPeriods>`, `ModelBuilder<Ready>`)
//! - ✅ Period integration using `finstack-core::dates::build_periods`
//! - ✅ Value nodes with explicit period values
//! - ✅ Calculated nodes with formulas
//!
//! ### Phase 2: DSL Engine ✅
//! - ✅ Parser for formula text (arithmetic, functions, operators)
//! - ✅ AST representation (`StmtExpr`)
//! - ✅ Compiler to core `Expr`
//! - ✅ Time-series operators (lag, lead, diff, pct_change) - fully implemented
//! - ✅ Rolling window functions (all variants: mean, sum, std, var, median, min, max)
//! - ✅ Statistical functions (std, var, median, cumsum, cumprod, cummin, cummax)
//! - ✅ Custom functions (sum, mean, ttm, annualize, coalesce)
//!
//! ### Phase 3: Evaluator ✅
//! - ✅ Evaluation context (`StatementContext`)
//! - ✅ Basic evaluator with period-by-period evaluation
//! - ✅ DAG construction and topological sort
//! - ✅ Precedence resolution (Value > Forecast > Formula)
//! - ✅ Where clause masking (conditional node evaluation)
//! - ✅ Circular dependency detection
//!
//! ### Phase 4: Forecast Methods ✅
//! - ✅ Forward fill
//! - ✅ Growth percentage (compound growth)
//! - ✅ Curve percentage (period-specific rates)
//! - ✅ Normal distribution (deterministic with seed)
//! - ✅ Log-normal distribution (always positive)
//! - ✅ Override (sparse period values)
//! - ✅ TimeSeries (external data reference)
//! - ✅ Seasonal (patterns with optional growth)
//!
//! ### Phase 5: Dynamic Registry ✅
//! - ✅ JSON schema for metrics
//! - ✅ Registry loader
//! - ✅ Built-in metrics (fin.* namespace)
//! - ✅ Inter-metric dependencies
//! - ✅ Namespace management
//!
//! ### Phase 6: Capital Structure ✅
//! - ✅ Bond and swap instruments
//! - ✅ Generic instrument support (automatic deserialization)
//! - ✅ Cashflow computation with market context
//! - ✅ `cs.*` namespace in DSL
//! - ✅ Integration with finstack-valuations
//!
//! ### Phase 7: Extensions ✅
//! - ✅ Extensions framework
//! - ✅ Corkscrew extension (balance sheet roll-forward validation)
//! - ✅ Credit scorecard extension (rating assignment)
//! - ✅ Results export to Polars DataFrames
//!
//! ### Phase 8: Adjustments Module ✅
//! - ✅ Normalization engine that reads evaluated nodes and produces adjusted metrics
//! - ✅ Add-backs tracker with detailed audit trail (raw/capped amounts per period)
//! - ✅ Capping policies tied to base nodes (e.g., synergies capped at % of EBITDA)
//! - ✅ Merge helpers for stamping Adjusted EBITDA (or other normalized nodes) back into results
//! - ✅ Serializable configs for adjustments to enable registry-driven workflows

/// Normalization engine and add-back tracking for adjusted metrics.
pub mod adjustments;
/// Analysis helpers and post-processing utilities.
pub mod analysis;
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
/// Templates for common models and schemas.
pub mod templates;
/// Core statement model types.
pub mod types;
pub(crate) mod utils;

// Re-export core types at crate root for ergonomic imports
pub use error::{Error, Result};
pub use evaluator::NumericMode;
pub use types::{
    AmountOrScalar, CapitalStructureSpec, DebtInstrumentSpec, FinancialModelSpec, ForecastMethod,
    ForecastSpec, NodeSpec, NodeType, NodeValueType, SeasonalMode,
};
