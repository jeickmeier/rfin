//! Domain-Specific Language (DSL) for financial statement formulas.
//!
//! The DSL engine provides:
//! - **Parser**: Convert formula text to AST
//! - **AST**: Structured representation of formulas
//! - **Compiler**: Transform AST to core's `Expr` for evaluation
//!
//! ## Supported Operations
//!
//! ### Arithmetic
//! - `+`, `-`, `*`, `/`, `%`
//!
//! ### Comparison
//! - `==`, `!=`, `<`, `<=`, `>`, `>=`
//!
//! ### Logical
//! - `and`, `or`
//!
//! ### Time-Series Functions
//! - `lag(expr, n)` - Previous n periods
//! - `diff(expr, n)` - First difference
//! - `pct_change(expr, n)` - Percentage change
//!
//! ### Rolling Window Functions
//! - `rolling_mean(expr, window)` - Rolling average
//! - `rolling_sum(expr, window)` - Rolling sum
//! - `rolling_std(expr, window)` - Rolling standard deviation
//! - `rolling_min(expr, window)` - Rolling minimum
//! - `rolling_max(expr, window)` - Rolling maximum
//!
//! ### Statistical Functions
//! - `std(expr)` - Standard deviation
//! - `var(expr)` - Variance
//! - `median(expr)` - Median value
//!
//! ### Custom Functions (Phase 2.6)
//! - `sum(...)` - Sum multiple values
//! - `mean(...)` - Average of values
//! - `annualize(expr, periods)` - Annualize a value
//! - `ttm(expr)` - Trailing twelve months
//! - `coalesce(expr, default)` - Null coalescing
//!
//! ### Conditional
//! - `if(condition, then_expr, else_expr)` - Conditional expression
//!
//! ## Example
//!
//! ```rust
//! use finstack_statements::dsl::{parse_formula, compile};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Parse a formula
//! let ast = parse_formula("(revenue - cogs) / revenue")?;
//!
//! // Compile to core Expr
//! let expr = compile(&ast)?;
//! # Ok(())
//! # }
//! ```

pub mod ast;
pub mod compiler;
pub mod parser;

pub use ast::{BinOp, StmtExpr, UnaryOp};
pub use compiler::compile;
pub use parser::parse_formula;

/// Parse and compile a formula in one step.
///
/// This is a convenience function that combines parsing and compilation.
///
/// # Example
///
/// ```rust
/// use finstack_statements::dsl::parse_and_compile;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let expr = parse_and_compile("revenue * 1.05")?;
/// # Ok(())
/// # }
/// ```
pub fn parse_and_compile(formula: &str) -> crate::error::Result<finstack_core::expr::Expr> {
    let ast = parse_formula(formula)?;
    compile(&ast)
}
