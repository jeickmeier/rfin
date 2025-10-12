//! Compiler from Statements DSL AST to core Expr.

use crate::dsl::ast::{BinOp as StmtBinOp, StmtExpr, UnaryOp as StmtUnaryOp};
use crate::error::Result;
use finstack_core::expr::{BinOp as CoreBinOp, Expr, Function, UnaryOp as CoreUnaryOp};

/// Compile a `StmtExpr` AST to core's `Expr`.
///
/// This converts the statements DSL syntax into the core expression engine's
/// representation for evaluation.
///
/// # Example
///
/// ```rust
/// use finstack_statements::dsl::{parse_formula, compile};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let ast = parse_formula("revenue - cogs")?;
/// let expr = compile(&ast)?;
/// # Ok(())
/// # }
/// ```
pub fn compile(ast: &StmtExpr) -> Result<Expr> {
    match ast {
        StmtExpr::Literal(val) => Ok(Expr::literal(*val)),

        StmtExpr::NodeRef(name) => Ok(Expr::column(name.clone())),

        // Capital structure references are encoded as special column names
        // Format: __cs__component__instrument_or_total
        StmtExpr::CSRef {
            component,
            instrument_or_total,
        } => {
            let encoded = format!("__cs__{}__{}", component, instrument_or_total);
            Ok(Expr::column(encoded))
        }

        StmtExpr::BinOp { op, left, right } => compile_bin_op(*op, left, right),

        StmtExpr::UnaryOp { op, operand } => compile_unary_op(*op, operand),

        StmtExpr::Call { func, args } => compile_function_call(func, args),

        StmtExpr::IfThenElse {
            condition,
            then_expr,
            else_expr,
        } => compile_if_then_else(condition, then_expr, else_expr),
    }
}

/// Compile binary operations.
fn compile_bin_op(op: StmtBinOp, left: &StmtExpr, right: &StmtExpr) -> Result<Expr> {
    let left_expr = compile(left)?;
    let right_expr = compile(right)?;

    // Map statement BinOp to core BinOp
    let core_op = match op {
        StmtBinOp::Add => CoreBinOp::Add,
        StmtBinOp::Sub => CoreBinOp::Sub,
        StmtBinOp::Mul => CoreBinOp::Mul,
        StmtBinOp::Div => CoreBinOp::Div,
        StmtBinOp::Mod => CoreBinOp::Mod,
        StmtBinOp::Eq => CoreBinOp::Eq,
        StmtBinOp::Ne => CoreBinOp::Ne,
        StmtBinOp::Lt => CoreBinOp::Lt,
        StmtBinOp::Le => CoreBinOp::Le,
        StmtBinOp::Gt => CoreBinOp::Gt,
        StmtBinOp::Ge => CoreBinOp::Ge,
        StmtBinOp::And => CoreBinOp::And,
        StmtBinOp::Or => CoreBinOp::Or,
    };

    Ok(Expr::bin_op(core_op, left_expr, right_expr))
}

/// Compile unary operations.
fn compile_unary_op(op: StmtUnaryOp, operand: &StmtExpr) -> Result<Expr> {
    let operand_expr = compile(operand)?;

    let core_op = match op {
        StmtUnaryOp::Neg => CoreUnaryOp::Neg,
        StmtUnaryOp::Not => CoreUnaryOp::Not,
    };

    Ok(Expr::unary_op(core_op, operand_expr))
}

/// Compile function calls.
fn compile_function_call(func_name: &str, args: &[StmtExpr]) -> Result<Expr> {
    let compiled_args: Result<Vec<_>> = args.iter().map(compile).collect();
    let compiled_args = compiled_args?;

    // Handle min/max specially by transforming to nested conditionals
    if func_name == "min" {
        return compile_min_function(&compiled_args);
    }
    if func_name == "max" {
        return compile_max_function(&compiled_args);
    }

    // Map DSL function names to core Function enum
    let func = match func_name {
        "lag" => Some(Function::Lag),
        // "lead" is intentionally not supported (no forward-looking in financial models)
        "diff" => Some(Function::Diff),
        "pct_change" => Some(Function::PctChange),
        "cumsum" => Some(Function::CumSum),
        "cumprod" => Some(Function::CumProd),
        "cummin" => Some(Function::CumMin),
        "cummax" => Some(Function::CumMax),
        "rolling_mean" => Some(Function::RollingMean),
        "rolling_sum" => Some(Function::RollingSum),
        "rolling_std" => Some(Function::RollingStd),
        "rolling_var" => Some(Function::RollingVar),
        "rolling_median" => Some(Function::RollingMedian),
        "rolling_min" => Some(Function::RollingMin),
        "rolling_max" => Some(Function::RollingMax),
        "rolling_count" => Some(Function::RollingCount),
        "ewm_mean" => Some(Function::EwmMean),
        "ewm_std" => Some(Function::EwmStd),
        "ewm_var" => Some(Function::EwmVar),
        "std" => Some(Function::Std),
        "var" => Some(Function::Var),
        "median" => Some(Function::Median),
        "shift" => Some(Function::Shift),
        "rank" => Some(Function::Rank),
        "quantile" => Some(Function::Quantile),
        "sum" => Some(Function::Sum),
        "mean" => Some(Function::Mean),
        "ttm" => Some(Function::Ttm),
        "annualize" => Some(Function::Annualize),
        "coalesce" => Some(Function::Coalesce),
        _ => None,
    };

    if let Some(f) = func {
        // Validate argument counts for custom functions
        match f {
            Function::Sum | Function::Mean => {
                if compiled_args.is_empty() {
                    return Err(crate::error::Error::eval(format!(
                        "{:?} requires at least one argument",
                        f
                    )));
                }
            }
            Function::Ttm => {
                if compiled_args.len() != 1 {
                    return Err(crate::error::Error::eval(
                        "ttm() requires exactly 1 argument",
                    ));
                }
            }
            Function::Annualize => {
                if compiled_args.len() != 2 {
                    return Err(crate::error::Error::eval(
                        "annualize() requires 2 arguments (value, periods_per_year)",
                    ));
                }
            }
            Function::Coalesce => {
                if compiled_args.len() < 2 {
                    return Err(crate::error::Error::eval(
                        "coalesce() requires at least 2 arguments",
                    ));
                }
            }
            _ => {}
        }
        Ok(Expr::call(f, compiled_args))
    } else {
        Err(crate::error::Error::eval(format!(
            "Function '{}' is not supported. \
             Supported functions include: lag, diff, pct_change, rolling_*, ewm_*, std, var, median, \
             sum, mean, min, max, ttm, annualize, coalesce",
            func_name
        )))
    }
}

/// Compile min function by transforming to nested if-then-else.
///
/// min(a, b) → if(a < b, a, b)
/// min(a, b, c) → if(a < b, if(a < c, a, c), if(b < c, b, c))
fn compile_min_function(args: &[Expr]) -> Result<Expr> {
    if args.is_empty() {
        return Err(crate::error::Error::eval(
            "min() requires at least 1 argument",
        ));
    }

    if args.len() == 1 {
        return Ok(args[0].clone());
    }

    // Recursively build nested conditionals
    let mut result = args[0].clone();
    for arg in &args[1..] {
        // result = if(result < arg, result, arg)
        let condition = Expr::bin_op(finstack_core::expr::BinOp::Lt, result.clone(), arg.clone());
        result = Expr::if_then_else(condition, result, arg.clone());
    }

    Ok(result)
}

/// Compile max function by transforming to nested if-then-else.
///
/// max(a, b) → if(a > b, a, b)
/// max(a, b, c) → if(a > b, if(a > c, a, c), if(b > c, b, c))
fn compile_max_function(args: &[Expr]) -> Result<Expr> {
    if args.is_empty() {
        return Err(crate::error::Error::eval(
            "max() requires at least 1 argument",
        ));
    }

    if args.len() == 1 {
        return Ok(args[0].clone());
    }

    // Recursively build nested conditionals
    let mut result = args[0].clone();
    for arg in &args[1..] {
        // result = if(result > arg, result, arg)
        let condition = Expr::bin_op(finstack_core::expr::BinOp::Gt, result.clone(), arg.clone());
        result = Expr::if_then_else(condition, result, arg.clone());
    }

    Ok(result)
}

/// Compile if-then-else expressions.
fn compile_if_then_else(
    condition: &StmtExpr,
    then_expr: &StmtExpr,
    else_expr: &StmtExpr,
) -> Result<Expr> {
    let cond = compile(condition)?;
    let then_branch = compile(then_expr)?;
    let else_branch = compile(else_expr)?;

    Ok(Expr::if_then_else(cond, then_branch, else_branch))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsl::parse_formula;
    use finstack_core::expr::ExprNode;

    #[test]
    fn test_compile_literal() {
        let ast = StmtExpr::literal(42.0);
        let expr = compile(&ast).unwrap();

        match expr.node {
            ExprNode::Literal(v) => assert_eq!(v, 42.0),
            _ => panic!("Expected Literal"),
        }
    }

    #[test]
    fn test_compile_node_ref() {
        let ast = StmtExpr::node_ref("revenue");
        let expr = compile(&ast).unwrap();

        match expr.node {
            ExprNode::Column(ref name) => assert_eq!(name, "revenue"),
            _ => panic!("Expected Column"),
        }
    }

    #[test]
    fn test_compile_addition() {
        let ast = StmtExpr::bin_op(
            StmtBinOp::Add,
            StmtExpr::literal(1.0),
            StmtExpr::literal(2.0),
        );

        let expr = compile(&ast).unwrap();

        // Should compile to a BinOp expression
        match expr.node {
            ExprNode::BinOp { .. } => {}
            _ => panic!("Expected BinOp for arithmetic"),
        }
    }

    #[test]
    fn test_compile_function_lag() {
        let ast = StmtExpr::call(
            "lag",
            vec![StmtExpr::node_ref("revenue"), StmtExpr::literal(1.0)],
        );

        let expr = compile(&ast).unwrap();

        match expr.node {
            ExprNode::Call(Function::Lag, args) => {
                assert_eq!(args.len(), 2);
            }
            _ => panic!("Expected Lag function call"),
        }
    }

    #[test]
    fn test_compile_from_parse() {
        let ast = parse_formula("revenue - cogs").unwrap();
        let expr = compile(&ast).unwrap();

        // Should compile successfully to a BinOp
        match expr.node {
            ExprNode::BinOp { .. } => {}
            _ => panic!("Expected BinOp for subtraction"),
        }
    }

    #[test]
    fn test_compile_complex_expression() {
        let ast = parse_formula("(revenue - cogs) / revenue").unwrap();
        let expr = compile(&ast);

        assert!(expr.is_ok());
    }
}
