//! Compiler from Statements DSL AST to core Expr.

use crate::dsl::ast::{BinOp, StmtExpr, UnaryOp};
use crate::error::Result;
use finstack_core::expr::{Expr, Function};

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
///
/// Note: Core's Function enum doesn't have arithmetic operators, so we
/// implement them as synthetic function calls for now.
fn compile_bin_op(op: BinOp, left: &StmtExpr, right: &StmtExpr) -> Result<Expr> {
    let left_expr = compile(left)?;
    let right_expr = compile(right)?;

    // For Phase 2, we'll represent arithmetic operations as pseudo-functions
    // These will need custom handling in the evaluator
    let func_name = match op {
        BinOp::Add => "add",
        BinOp::Sub => "sub",
        BinOp::Mul => "mul",
        BinOp::Div => "div",
        BinOp::Mod => "mod",
        BinOp::Eq => "eq",
        BinOp::Ne => "ne",
        BinOp::Lt => "lt",
        BinOp::Le => "le",
        BinOp::Gt => "gt",
        BinOp::Ge => "ge",
        BinOp::And => "and",
        BinOp::Or => "or",
    };

    // Create a synthetic function call node
    // This will need special handling in the evaluator since these aren't real Function variants
    Ok(create_synthetic_call(
        func_name,
        vec![left_expr, right_expr],
    ))
}

/// Compile unary operations.
fn compile_unary_op(op: UnaryOp, operand: &StmtExpr) -> Result<Expr> {
    let operand_expr = compile(operand)?;

    match op {
        UnaryOp::Neg => {
            // Represent as: 0 - operand
            Ok(create_synthetic_call(
                "sub",
                vec![Expr::literal(0.0), operand_expr],
            ))
        }
        UnaryOp::Not => Ok(create_synthetic_call("not", vec![operand_expr])),
    }
}

/// Compile function calls.
fn compile_function_call(func_name: &str, args: &[StmtExpr]) -> Result<Expr> {
    let compiled_args: Result<Vec<_>> = args.iter().map(compile).collect();
    let compiled_args = compiled_args?;

    // Map DSL function names to core Function enum
    let func = match func_name {
        "lag" => Some(Function::Lag),
        "lead" => Some(Function::Lead),
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
        _ => None,
    };

    if let Some(f) = func {
        Ok(Expr::call(f, compiled_args))
    } else {
        // For custom functions (sum, mean, annualize, ttm, etc.), create synthetic calls
        Ok(create_synthetic_call(func_name, compiled_args))
    }
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

    Ok(create_synthetic_call(
        "if",
        vec![cond, then_branch, else_branch],
    ))
}

/// Create a synthetic function call for operations not in core's Function enum.
///
/// This is a temporary solution for Phase 2. These will need custom evaluation
/// logic in the statements evaluator.
fn create_synthetic_call(name: &str, args: Vec<Expr>) -> Expr {
    // We store the function name in the expression by creating a special column reference
    // Format: "__stmt_fn::<name>" as a marker
    // The actual args are stored as a Call to a placeholder function

    // For now, we'll use a hack: encode the function name in the column reference
    // and wrap it with the first core function as a placeholder
    // This is NOT ideal but works for Phase 2 implementation
    //
    // A better solution would be to extend core's Function enum or create a wrapper

    // Store function name as a literal string by encoding it
    // This will require custom evaluator logic
    let marker = Expr::column(format!("__stmt_fn::{}", name));

    // Return a call structure with the marker and args
    // We'll use CumSum as a placeholder since it's harmless
    let mut all_args = vec![marker];
    all_args.extend(args);

    Expr::call(Function::CumSum, all_args)
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
        let ast = StmtExpr::bin_op(BinOp::Add, StmtExpr::literal(1.0), StmtExpr::literal(2.0));

        let expr = compile(&ast).unwrap();

        // Should compile to a synthetic function call
        match expr.node {
            ExprNode::Call(..) => {}
            _ => panic!("Expected Call for arithmetic"),
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

        // Should compile successfully
        match expr.node {
            ExprNode::Call(..) => {}
            _ => panic!("Expected Call for subtraction"),
        }
    }

    #[test]
    fn test_compile_complex_expression() {
        let ast = parse_formula("(revenue - cogs) / revenue").unwrap();
        let expr = compile(&ast);

        assert!(expr.is_ok());
    }
}
