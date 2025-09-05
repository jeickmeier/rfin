//! Expression engine with DAG planning, caching, and advanced features.
//!
//! Supported functions:
//! - lag(expr, n) / lead(expr, n)
//! - diff(expr, n) / pct_change(expr, n)
//! - cumsum / cumprod / cummin / cummax
//! - rolling_mean / rolling_sum (row windows)
//! - rolling_std / rolling_var / rolling_median
//! - ewm_mean(expr, alpha, adjust)
//! - std / var / median
//!
//! Features:
//! - DAG planning with shared sub-expression detection
//! - Intelligent caching for intermediate results
//! - Pushdown boundary detection for Polars optimization
//! - Determinism toggles and metadata stamping
//!
//! # Polars vs Scalar Execution
//!
//! The expression engine automatically determines whether functions can be executed
//! via Polars vectorization or must fall back to scalar evaluation. This table
//! shows the execution strategy for each function:
//!
//! | Function | Polars Lowering | Scalar Fallback | Notes |
//! |----------|------------------|-----------------|-------|
//! | `lag(expr, n)` | ✅ `col.shift(n)` | ✅ | Row-based shift |
//! | `lead(expr, n)` | ✅ `col.shift(-n)` | ✅ | Row-based shift |
//! | `diff(expr, n)` | ✅ `x - x.shift(n)` | ✅ | First difference |
//! | `pct_change(expr, n)` | ✅ `x / x.shift(n) - 1` | ✅ | Percentage change |
//! | `rolling_mean(expr, n)` | ✅ shifted-sum / n | ✅ | Row window |
//! | `rolling_sum(expr, n)` | ✅ shifted-sum | ✅ | Row window |
//! | `std(expr)` | ✅ `col.std(ddof=1)` | ✅ | Sample std |
//! | `var(expr)` | ✅ `col.var(ddof=1)` | ✅ | Sample var |
//! | `median(expr)` | ✅ `col.median()` | ✅ | |
//! | `shift(expr, n)` | ✅ `col.shift(n)` | ✅ | Positive=down |
//! 
//!
//! Note: Functions like `mean`, `sum`, `min`, `max`, `count`, and time-based
//! dynamic windows are not part of this engine's function set; any aggregation
//! semantics should be expressed via higher-level APIs.
//!
//! Implementation note: Literal lowering is disabled under the `decimal128`
//! feature; in that mode pure-literal expressions won’t lower to Polars.
//!
//! Keep this table in sync with `CompiledExpr::to_polars_expr`.
//!
//! # Execution Strategy
//!
//! 1. **Polars Lowering**: Functions marked with ✅ are automatically lowered to
//!    Polars expressions for vectorized execution when possible.
//! 2. **Scalar Fallback**: All functions have scalar implementations that are
//!    used when Polars lowering is not possible or when the expression context
//!    doesn't support vectorization.
//! 3. **Mixed Execution**: Complex expressions may use both strategies, with
//!    Polars for supported sub-expressions and scalar for unsupported parts.
//! 4. **Determinism**: Both execution paths produce identical results, ensuring
//!    consistent behavior regardless of the execution strategy used.

mod ast;
mod cache;
mod context;
mod dag;
mod eval;

pub use ast::{EvaluationResult, Expr, ExprNode, Function};
pub use cache::{CacheManager, CachedResult};
pub use context::{ExpressionContext, SimpleContext};
pub use dag::{DagBuilder, ExecutionPlan, PushdownAnalyzer, PushdownBoundaries};
pub use eval::{CompiledExpr, EvalOpts};

// Polars Series no longer part of public API surface here
