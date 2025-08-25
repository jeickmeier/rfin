//! Minimal expression engine: AST, scalar evaluation, and Polars lowering.
//!
//! Supported functions:
//! - lag(expr, n)
//! - lead(expr, n)
//! - diff(expr, n)
//! - pct_change(expr, n)
//! - cumsum/ cumprod/ cummin/ cummax
//! - rolling_mean/ rolling_sum (row windows)
//! - ewm_mean(expr, alpha, adjust)

mod ast;
mod context;
mod eval;

pub use ast::{Expr, Function};
pub use context::{ExpressionContext, SimpleContext};
pub use eval::CompiledExpr;
