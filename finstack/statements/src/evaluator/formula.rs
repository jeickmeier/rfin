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
    use finstack_core::expr::{BinOp, ExprNode, UnaryOp};
    
    match &expr.node {
        ExprNode::Literal(val) => Ok(*val),
        ExprNode::Column(name) => context.get_value(name),
        ExprNode::Call(func, args) => evaluate_function(func, args, context),
        ExprNode::BinOp { op, left, right } => {
            let left_val = evaluate_expr(left, context)?;
            let right_val = evaluate_expr(right, context)?;
            
            let result = match op {
                // Arithmetic
                BinOp::Add => left_val + right_val,
                BinOp::Sub => left_val - right_val,
                BinOp::Mul => left_val * right_val,
                BinOp::Div => {
                    if right_val == 0.0 {
                        f64::NAN
                    } else {
                        left_val / right_val
                    }
                }
                BinOp::Mod => left_val % right_val,
                
                // Comparison (return 1.0 for true, 0.0 for false)
                BinOp::Eq => if left_val == right_val { 1.0 } else { 0.0 },
                BinOp::Ne => if left_val != right_val { 1.0 } else { 0.0 },
                BinOp::Lt => if left_val < right_val { 1.0 } else { 0.0 },
                BinOp::Le => if left_val <= right_val { 1.0 } else { 0.0 },
                BinOp::Gt => if left_val > right_val { 1.0 } else { 0.0 },
                BinOp::Ge => if left_val >= right_val { 1.0 } else { 0.0 },
                
                // Logical (treat non-zero as true)
                BinOp::And => if left_val != 0.0 && right_val != 0.0 { 1.0 } else { 0.0 },
                BinOp::Or => if left_val != 0.0 || right_val != 0.0 { 1.0 } else { 0.0 },
            };
            Ok(result)
        }
        ExprNode::UnaryOp { op, operand } => {
            let val = evaluate_expr(operand, context)?;
            let result = match op {
                UnaryOp::Neg => -val,
                UnaryOp::Not => if val == 0.0 { 1.0 } else { 0.0 },
            };
            Ok(result)
        }
        ExprNode::IfThenElse {
            condition,
            then_expr,
            else_expr,
        } => {
            let cond_val = evaluate_expr(condition, context)?;
            if cond_val != 0.0 {
                evaluate_expr(then_expr, context)
            } else {
                evaluate_expr(else_expr, context)
            }
        }
    }
}

/// Evaluate a function call.
fn evaluate_function(
    func: &Function,
    args: &[Expr],
    _context: &StatementContext,
) -> Result<f64> {
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
