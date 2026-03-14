//! Expression engine with DAG planning, caching, and scalar evaluation.
//!
//! Supported functions:
//! - lag(expr, n) / lead(expr, n)
//! - diff(expr, n) / pct_change(expr, n)
//! - cumsum / cumprod / cummin / cummax
//! - rolling_mean / rolling_sum (row windows)
//! - rolling_std / rolling_var / rolling_median
//! - ewm_mean(expr, alpha, adjust)
//! - std / var / median
//! - shift / rank / quantile (reducer over entire series; broadcasts scalar)
//!   - For rolling/windowed quantiles, use `rolling_median` or implement a
//!     domain-specific rolling estimator; `quantile` here is a global reducer.
//! - rolling_min / rolling_max / rolling_count
//! - ewm_std / ewm_var
//!
//! Features:
//! - DAG planning with shared sub-expression detection
//! - Intelligent caching for intermediate results
//! - Optimized scalar implementations
//! - Deterministic execution
//! - Metadata stamping for results
//!
//! # Quick Example
//!
//! ```rust
//! use finstack_core::expr::{Expr, Function, CompiledExpr, SimpleContext, EvalOpts};
//!
//! // Create expression: rolling_mean(x, 3)
//! let expr = Expr::call(
//!     Function::RollingMean,
//!     vec![Expr::column("x"), Expr::literal(3.0)]
//! );
//!
//! // Compile and evaluate
//! let compiled = CompiledExpr::new(expr);
//! let context = SimpleContext::new(["x"]);
//! let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
//! let cols = [data.as_slice()];
//! let result = compiled.eval(&context, &cols, EvalOpts::default()).unwrap();
//! ```
//!
//! # Execution Strategy
//!
//! All functions are implemented using optimized scalar algorithms that:
//! 1. Minimize allocations through buffer reuse
//! 2. Use vectorized patterns where beneficial (e.g., rolling windows)
//! 3. Provide deterministic results across platforms
//! 4. Support WASM compilation without external dependencies

mod ast;
pub(crate) mod cache;
mod context;
mod dag;
mod eval;
mod eval_functions;

// Public API - simplified surface for end users
pub use ast::{BinOp, EvaluationResult, Expr, ExprNode, Function, UnaryOp};
pub use context::SimpleContext;
pub use eval::{CompiledExpr, EvalOpts};

/// DAG planning types are considered internal to the expression engine.
///
/// They remain public for bindings/tests that need them, but are hidden from
/// the primary docs and are not a stable surface.
#[doc(hidden)]
pub use dag::{
    BoundaryType, CacheStrategy, DagBuilder, DagNode, ExecutionPlan, PushdownBoundaries,
    PushdownBoundary,
};

// Polars Series no longer part of public API surface here
