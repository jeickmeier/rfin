//! Formula evaluation logic.

use crate::error::{Error, Result};
use crate::evaluator::context::StatementContext;
use finstack_core::expr::{Expr, Function};

/// Compiled expression wrapper.
#[derive(Debug, Clone)]
pub(crate) struct CompiledExpr {
    pub expr: Expr,
}

impl CompiledExpr {
    pub fn new(expr: Expr) -> Self {
        Self { expr }
    }
}

/// Evaluate a compiled expression.
pub(crate) fn evaluate_formula(
    compiled: &CompiledExpr,
    context: &StatementContext,
) -> Result<f64> {
    evaluate_expr(&compiled.expr, context)
}

/// Recursively evaluate an expression.
pub(crate) fn evaluate_expr(expr: &Expr, context: &StatementContext) -> Result<f64> {
    use finstack_core::expr::ExprNode;
    
    match &expr.node {
        ExprNode::Literal(val) => Ok(*val),
        ExprNode::Column(name) => context.get_value(name),
        ExprNode::Call(func, args) => evaluate_function(func, args, context),
    }
}

/// Evaluate a function call.
fn evaluate_function(
    func: &Function,
    args: &[Expr],
    context: &StatementContext,
) -> Result<f64> {
    use finstack_core::expr::ExprNode;
    
    // Check for synthetic operations (encoded as CumSum with special marker)
    // The compiler encodes synthetic ops as: CumSum([Column("__stmt_fn::<name>"), ...args])
    if matches!(func, Function::CumSum) && !args.is_empty() {
        if let ExprNode::Column(ref marker) = args[0].node {
            if let Some(op_name) = marker.strip_prefix("__stmt_fn::") {
                return evaluate_synthetic_op(op_name, &args[1..], context);
            }
        }
    }

    // Handle real functions from finstack-core
    match func {
        Function::Lag => {
            if args.len() != 2 {
                return Err(Error::eval("lag() requires 2 arguments"));
            }
            // TODO: Implement lag logic with historical context
            Err(Error::eval("lag() not yet implemented"))
        }
        Function::Lead => {
            if args.len() != 2 {
                return Err(Error::eval("lead() requires 2 arguments"));
            }
            // TODO: Implement lead logic
            Err(Error::eval("lead() not yet implemented"))
        }
        _ => Err(Error::eval(format!("Function {:?} not supported", func))),
    }
}

/// Evaluate synthetic operations (arithmetic, comparison, logical).
fn evaluate_synthetic_op(
    op_name: &str,
    args: &[Expr],
    context: &StatementContext,
) -> Result<f64> {
    match op_name {
        // Arithmetic operations
        "add" => eval_arithmetic_binary(args, context, |a, b| a + b),
        "sub" => eval_arithmetic_binary(args, context, |a, b| a - b),
        "mul" => eval_arithmetic_binary(args, context, |a, b| a * b),
        "div" => eval_arithmetic_binary(args, context, |a, b| {
            if b == 0.0 {
                f64::NAN
            } else {
                a / b
            }
        }),
        "mod" => eval_arithmetic_binary(args, context, |a, b| a % b),

        // Comparison operations
        "eq" => eval_comparison_binary(args, context, |a, b| a == b),
        "ne" => eval_comparison_binary(args, context, |a, b| a != b),
        "lt" => eval_comparison_binary(args, context, |a, b| a < b),
        "le" => eval_comparison_binary(args, context, |a, b| a <= b),
        "gt" => eval_comparison_binary(args, context, |a, b| a > b),
        "ge" => eval_comparison_binary(args, context, |a, b| a >= b),

        // Logical operations
        "and" => eval_logical_binary(args, context, |a, b| a && b),
        "or" => eval_logical_binary(args, context, |a, b| a || b),
        "not" => {
            if args.len() != 1 {
                return Err(Error::eval("not operator requires 1 argument"));
            }
            let val = evaluate_expr(&args[0], context)?;
            Ok(if val == 0.0 { 1.0 } else { 0.0 })
        }

        // Conditional (if-then-else)
        "if" => {
            if args.len() != 3 {
                return Err(Error::eval("if requires 3 arguments (condition, then_expr, else_expr)"));
            }
            let condition = evaluate_expr(&args[0], context)?;
            if condition != 0.0 {
                evaluate_expr(&args[1], context)
            } else {
                evaluate_expr(&args[2], context)
            }
        }

        _ => Err(Error::eval(format!("Unknown synthetic operation: {}", op_name))),
    }
}

/// Helper for binary arithmetic operations.
fn eval_arithmetic_binary<F>(
    args: &[Expr],
    context: &StatementContext,
    op: F,
) -> Result<f64>
where
    F: Fn(f64, f64) -> f64,
{
    if args.len() != 2 {
        return Err(Error::eval("Binary operation requires 2 arguments"));
    }
    let left = evaluate_expr(&args[0], context)?;
    let right = evaluate_expr(&args[1], context)?;
    Ok(op(left, right))
}

/// Helper for binary comparison operations.
fn eval_comparison_binary<F>(
    args: &[Expr],
    context: &StatementContext,
    op: F,
) -> Result<f64>
where
    F: Fn(f64, f64) -> bool,
{
    if args.len() != 2 {
        return Err(Error::eval("Binary comparison requires 2 arguments"));
    }
    let left = evaluate_expr(&args[0], context)?;
    let right = evaluate_expr(&args[1], context)?;
    Ok(if op(left, right) { 1.0 } else { 0.0 })
}

/// Helper for binary logical operations.
fn eval_logical_binary<F>(
    args: &[Expr],
    context: &StatementContext,
    op: F,
) -> Result<f64>
where
    F: Fn(bool, bool) -> bool,
{
    if args.len() != 2 {
        return Err(Error::eval("Binary logical operation requires 2 arguments"));
    }
    let left = evaluate_expr(&args[0], context)?;
    let right = evaluate_expr(&args[1], context)?;
    Ok(if op(left != 0.0, right != 0.0) {
        1.0
    } else {
        0.0
    })
}

