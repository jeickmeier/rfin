//! # Finstack Statements — Financial Statement Modeling Engine
//!
//! The `finstack-statements` crate enables users to build financial statement models
//! as directed graphs of metrics evaluated over discrete periods (monthly, quarterly, annually).
//!
//! ## Key Features
//!
//! - **Declarative modeling** with a rich DSL for formulas
//! - **Time-series forecasting** with deterministic and statistical methods
//! - **Capital structure integration** for debt/equity tracking (feature-gated)
//! - **Dynamic metric registry** (no recompilation needed)
//! - **Currency-safe arithmetic** with explicit FX handling
//! - **Deterministic evaluation** (serial ≡ parallel)
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
//!
//! ## Implementation Status
//!
//! ### Phase 1: Foundation ✅
//! - ✅ Core wire types (`NodeSpec`, `NodeType`, `AmountOrScalar`)
//! - ✅ Type-state builder pattern (`ModelBuilder<NeedPeriods>`, `ModelBuilder<Ready>`)
//! - ✅ Period integration using `finstack-core::dates::build_periods`
//! - ✅ Value nodes with explicit period values
//! - ✅ Basic calculated nodes with formulas (no evaluation yet)
//!
//! ### Phase 2: DSL Engine ✅
//! - ✅ Parser for formula text (arithmetic, functions, operators)
//! - ✅ AST representation (`StmtExpr`)
//! - ✅ Compiler to core `Expr`
//! - ✅ Time-series operators (lag, lead, diff, pct_change)
//! - ✅ Rolling window functions (rolling_mean, rolling_sum, rolling_std)
//! - ✅ Statistical functions (std, var, median)
//!
//! ### Phase 3: Evaluator ✅
//! - ✅ Evaluation context (`StatementContext`)
//! - ✅ Basic evaluator with period-by-period evaluation
//! - ✅ DAG construction and topological sort
//! - ✅ Precedence resolution (Value > Forecast > Formula)
//! - ✅ Where clause masking
//! - ✅ Circular dependency detection
//!
//! Future phases will add:
//! - Forecast methods (forward fill, growth, statistical)
//! - Dynamic metric registry
//! - Capital structure integration
//! - DataFrame export (Polars)

#![deny(unsafe_code)]
#![warn(missing_docs)]

pub mod error;
pub mod types;
pub mod builder;
pub mod dsl;
pub mod evaluator;

/// Commonly used types and traits.
///
/// Import this module to get quick access to the most common types:
///
/// ```rust
/// use finstack_statements::prelude::*;
/// ```
pub mod prelude {
    pub use crate::builder::{ModelBuilder, NeedPeriods, Ready};
    pub use crate::error::{Error, Result};
    pub use crate::types::{AmountOrScalar, FinancialModelSpec, NodeSpec, NodeType};
    pub use crate::evaluator::{Evaluator, Results};

    // Re-export commonly used types from finstack-core
    pub use finstack_core::dates::{build_periods, Period, PeriodId};
    pub use finstack_core::money::Money;
    pub use finstack_core::currency::Currency;
}
