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
//! - Time-based windows with every="30d" syntax
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
//! |----------|----------------|-----------------|-------|
//! | `lag(expr, n)` | ✅ `col.shift(n)` | ✅ | Row-based shifting |
//! | `lead(expr, n)` | ✅ `col.shift(-n)` | ✅ | Row-based shifting |
//! | `diff(expr, n)` | ✅ `col.diff(n)` | ✅ | First difference |
//! | `pct_change(expr, n)` | ✅ `col.pct_change(n)` | ✅ | Percentage change |
//! | `cumsum(expr)` | ✅ `col.cumsum()` | ✅ | Cumulative sum |
//! | `cumprod(expr)` | ✅ `col.cumprod()` | ✅ | Cumulative product |
//! | `cummin(expr)` | ✅ `col.cummin()` | ✅ | Cumulative minimum |
//! | `cummax(expr)` | ✅ `col.cummax()` | ✅ | Cumulative maximum |
//! | `rolling_sum(expr, window)` | ✅ `col.rolling_sum(window)` | ✅ | Rolling sum |
//! | `rolling_mean(expr, window)` | ✅ `col.rolling_mean(window)` | ✅ | Rolling mean |
//! | `rolling_std(expr, window)` | ✅ `col.rolling_std(window)` | ✅ | Rolling std dev |
//! | `rolling_var(expr, window)` | ✅ `col.rolling_var(window)` | ✅ | Rolling variance |
//! | `rolling_median(expr, window)` | ✅ `col.rolling_median(window)` | ✅ | Rolling median |
//! | `rolling_min(expr, window)` | ✅ `col.rolling_min(window)` | ✅ | Rolling minimum |
//! | `rolling_max(expr, window)` | ✅ `col.rolling_max(window)` | ✅ | Rolling maximum |
//! | `rolling_time_mean(expr, window, time_col)` | ✅ `group_by_dynamic(time_col, every=window).agg([col.mean()])` | ✅ | Time-based windows |
//! | `ewm_mean(expr, alpha, adjust)` | ✅ `col.ewm_mean(alpha, adjust)` | ✅ | Exponential weighted mean |
//! | `std(expr)` | ✅ `col.std()` | ✅ | Standard deviation |
//! | `var(expr)` | ✅ `col.var()` | ✅ | Variance |
//! | `median(expr)` | ✅ `col.median()` | ✅ | Median |
//! | `mean(expr)` | ✅ `col.mean()` | ✅ | Mean |
//! | `sum(expr)` | ✅ `col.sum()` | ✅ | Sum |
//! | `min(expr)` | ✅ `col.min()` | ✅ | Minimum |
//! | `max(expr)` | ✅ `col.max()` | ✅ | Maximum |
//! | `count(expr)` | ✅ `col.count()` | ✅ | Count |
//! | Custom functions | ❌ | ✅ | User-defined functions |
//! | Complex expressions | ❌ | ✅ | Multi-step expressions |
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

pub use ast::{
    DurationSpec, EvaluationResult, Expr, ExprNode, Function, TimeWindow, WindowSpec,
};
pub use cache::{CacheManager, CachedResult};
pub use context::{ExpressionContext, SimpleContext};
pub use dag::{DagBuilder, ExecutionPlan, PushdownAnalyzer, PushdownBoundaries};
pub use eval::CompiledExpr;

// Re-export Polars Series type since it's part of CachedResult's public API
pub use polars::prelude::Series;
