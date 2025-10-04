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

#![deny(unsafe_code)]
#![warn(missing_docs)]

pub mod builder;
pub mod capital_structure;
pub mod dsl;
pub mod error;
pub mod evaluator;
pub mod extensions;
pub mod forecast;
pub mod registry;
pub mod results;
pub mod types;

// Re-export core types at crate root for ergonomic imports
pub use error::{Error, Result};
pub use types::{
    AmountOrScalar, CapitalStructureSpec, DebtInstrumentSpec, FinancialModelSpec, ForecastMethod,
    ForecastSpec, NodeSpec, NodeType, SeasonalMode,
};

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
    pub use crate::evaluator::{Evaluator, Results};
    pub use crate::extensions::{
        CorkscrewExtension, CreditScorecardExtension, Extension, ExtensionContext,
        ExtensionMetadata, ExtensionRegistry, ExtensionResult, ExtensionStatus,
    };
    pub use crate::registry::Registry;
    pub use crate::types::{
        AmountOrScalar, FinancialModelSpec, ForecastMethod, ForecastSpec, NodeSpec, NodeType,
        SeasonalMode,
    };

    // Re-export commonly used types from finstack-core
    pub use finstack_core::currency::Currency;
    pub use finstack_core::dates::{build_periods, Period, PeriodId, PeriodKind};
    pub use finstack_core::money::Money;
}
