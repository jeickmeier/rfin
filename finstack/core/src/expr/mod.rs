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

mod ast;
mod cache;
mod context;
mod dag;
mod eval;
mod time_windows;

pub use ast::{
    EvaluationResult, ExecMeta, Expr, ExprNode, Function, ResultMetadata, TimeWindow, WindowSpec,
};
pub use cache::{CacheManager, CachedResult};
pub use context::{ExpressionContext, SimpleContext};
pub use dag::{DagBuilder, ExecutionPlan, PushdownAnalyzer, PushdownBoundaries};
pub use eval::CompiledExpr;
pub use time_windows::parse_duration;

// Re-export Polars Series type since it's part of CachedResult's public API
pub use polars::prelude::Series;
