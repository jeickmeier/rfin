//! Formula evaluation logic.

use crate::error::{Error, Result};
use crate::evaluator::context::EvaluationContext;
use finstack_core::expr::{Expr, ExprNode, Function};

/// Evaluate a compiled expression.
pub(crate) fn evaluate_formula(expr: &Expr, context: &EvaluationContext) -> Result<f64> {
    evaluate_expr(expr, context)
}

/// Recursively evaluate an expression.
pub(crate) fn evaluate_expr(expr: &Expr, context: &EvaluationContext) -> Result<f64> {
    use finstack_core::expr::{BinOp, ExprNode, UnaryOp};

    match &expr.node {
        ExprNode::Literal(val) => Ok(*val),
        ExprNode::Column(name) => {
            // Check if this is a capital structure reference (format: __cs__component__instrument_or_total)
            if name.starts_with("__cs__") {
                let parts: Vec<&str> = name.split("__").collect();
                if parts.len() == 4 && parts[0].is_empty() && parts[1] == "cs" {
                    let component = parts[2];
                    let instrument_or_total = parts[3];
                    return context.get_cs_value(component, instrument_or_total);
                }
            }
            context.get_value(name)
        }
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
                BinOp::Eq => {
                    if left_val == right_val {
                        1.0
                    } else {
                        0.0
                    }
                }
                BinOp::Ne => {
                    if left_val != right_val {
                        1.0
                    } else {
                        0.0
                    }
                }
                BinOp::Lt => {
                    if left_val < right_val {
                        1.0
                    } else {
                        0.0
                    }
                }
                BinOp::Le => {
                    if left_val <= right_val {
                        1.0
                    } else {
                        0.0
                    }
                }
                BinOp::Gt => {
                    if left_val > right_val {
                        1.0
                    } else {
                        0.0
                    }
                }
                BinOp::Ge => {
                    if left_val >= right_val {
                        1.0
                    } else {
                        0.0
                    }
                }

                // Logical (treat non-zero as true)
                BinOp::And => {
                    if left_val != 0.0 && right_val != 0.0 {
                        1.0
                    } else {
                        0.0
                    }
                }
                BinOp::Or => {
                    if left_val != 0.0 || right_val != 0.0 {
                        1.0
                    } else {
                        0.0
                    }
                }
            };
            Ok(result)
        }
        ExprNode::UnaryOp { op, operand } => {
            let val = evaluate_expr(operand, context)?;
            let result = match op {
                UnaryOp::Neg => -val,
                UnaryOp::Not => {
                    if val == 0.0 {
                        1.0
                    } else {
                        0.0
                    }
                }
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
fn evaluate_function(func: &Function, args: &[Expr], context: &EvaluationContext) -> Result<f64> {
    // Handle real functions from finstack-core
    match func {
        Function::Lag => {
            if args.len() != 2 {
                return Err(Error::eval("lag() requires 2 arguments"));
            }
            // Evaluate the expression to get the node name
            let node_value = evaluate_expr(&args[0], context)?;
            let lag_periods = evaluate_expr(&args[1], context)? as i32;
            
            // For now, we'll use a simple approach: get the node name from the first arg
            // In a full implementation, we'd need to handle arbitrary expressions
            if let ExprNode::Column(node_name) = &args[0].node {
                // Find the lagged period
                let current_period = context.period_id;
                // This is simplified - in production we'd need proper period arithmetic
                // For now, return the node value if lag is 0, otherwise look in historical
                if lag_periods == 0 {
                    context.get_value(node_name)
                } else {
                    // Look for the value in historical results
                    // This would need proper period offset calculation
                    context.get_historical_value(node_name, &current_period)
                        .ok_or_else(|| Error::eval(format!("No historical value for {} with lag {}", node_name, lag_periods)))
                }
            } else {
                // If not a simple column reference, evaluate the expression
                Ok(node_value)
            }
        }
        Function::Lead => {
            if args.len() != 2 {
                return Err(Error::eval("lead() requires 2 arguments"));
            }
            // Lead requires looking forward, which is more complex
            // For now, return the current value as a placeholder
            evaluate_expr(&args[0], context)
        }
        Function::Diff => {
            if args.is_empty() || args.len() > 2 {
                return Err(Error::eval("diff() requires 1 or 2 arguments"));
            }
            // Calculate first difference: current - lag(current, periods)
            let current = evaluate_expr(&args[0], context)?;
            let _lag_periods = if args.len() == 2 {
                evaluate_expr(&args[1], context)? as i32
            } else {
                1 // Default to lag of 1
            };
            
            // Get lagged value
            if let ExprNode::Column(node_name) = &args[0].node {
                if let Some(lagged_value) = context.get_historical_value(node_name, &context.period_id) {
                    Ok(current - lagged_value)
                } else {
                    // No historical value, return 0 or NaN
                    Ok(0.0)
                }
            } else {
                // Can't compute diff for complex expressions without history
                Ok(0.0)
            }
        }
        Function::PctChange => {
            if args.is_empty() || args.len() > 2 {
                return Err(Error::eval("pct_change() requires 1 or 2 arguments"));
            }
            // Calculate percentage change: (current - lag) / lag
            let current = evaluate_expr(&args[0], context)?;
            let _lag_periods = if args.len() == 2 {
                evaluate_expr(&args[1], context)? as i32
            } else {
                1 // Default to lag of 1
            };
            
            // Get lagged value
            if let ExprNode::Column(node_name) = &args[0].node {
                if let Some(lagged_value) = context.get_historical_value(node_name, &context.period_id) {
                    if lagged_value != 0.0 {
                        Ok((current - lagged_value) / lagged_value)
                    } else {
                        Ok(f64::NAN) // Division by zero
                    }
                } else {
                    // No historical value
                    Ok(0.0)
                }
            } else {
                // Can't compute pct_change for complex expressions without history
                Ok(0.0)
            }
        }
        // Rolling window functions
        Function::RollingMean | Function::RollingSum | Function::RollingStd | 
        Function::RollingVar | Function::RollingMedian | Function::RollingMin | 
        Function::RollingMax | Function::RollingCount => {
            if args.len() != 2 {
                return Err(Error::eval(format!("{:?} requires 2 arguments (expression, window)", func)));
            }
            
            let window = evaluate_expr(&args[1], context)? as usize;
            if window == 0 {
                return Err(Error::eval("Window size must be greater than 0"));
            }
            
            // Collect values from historical data for the window
            let mut values = Vec::new();
            
            // Add current value
            let current = evaluate_expr(&args[0], context)?;
            values.push(current);
            
            // Add historical values if available
            if let ExprNode::Column(node_name) = &args[0].node {
                // Simplified: just use any available historical values
                // In production, we'd need proper period lookback
                for (_period, period_values) in &context.historical_results {
                    if let Some(value) = period_values.get(node_name) {
                        values.push(*value);
                        if values.len() >= window {
                            break;
                        }
                    }
                }
            }
            
            // If we don't have enough values, use what we have
            let actual_window = values.len().min(window);
            if actual_window == 0 {
                return Ok(0.0);
            }
            
            match func {
                Function::RollingMean => {
                    Ok(values[..actual_window].iter().sum::<f64>() / actual_window as f64)
                }
                Function::RollingSum => {
                    Ok(values[..actual_window].iter().sum())
                }
                Function::RollingStd => {
                    // Calculate standard deviation
                    let mean = values[..actual_window].iter().sum::<f64>() / actual_window as f64;
                    let variance = values[..actual_window].iter()
                        .map(|v| (v - mean).powi(2))
                        .sum::<f64>() / actual_window as f64;
                    Ok(variance.sqrt())
                }
                Function::RollingVar => {
                    // Calculate variance
                    let mean = values[..actual_window].iter().sum::<f64>() / actual_window as f64;
                    Ok(values[..actual_window].iter()
                        .map(|v| (v - mean).powi(2))
                        .sum::<f64>() / actual_window as f64)
                }
                Function::RollingMedian => {
                    // Calculate median
                    let mut sorted = values[..actual_window].to_vec();
                    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
                    if actual_window % 2 == 0 {
                        Ok((sorted[actual_window / 2 - 1] + sorted[actual_window / 2]) / 2.0)
                    } else {
                        Ok(sorted[actual_window / 2])
                    }
                }
                Function::RollingMin => {
                    Ok(values[..actual_window].iter().fold(f64::INFINITY, |a, b| a.min(*b)))
                }
                Function::RollingMax => {
                    Ok(values[..actual_window].iter().fold(f64::NEG_INFINITY, |a, b| a.max(*b)))
                }
                Function::RollingCount => {
                    Ok(actual_window as f64)
                }
                _ => unreachable!(),
            }
        }
        
        // Statistical functions (operate on all historical values)
        Function::Std | Function::Var | Function::Median | Function::CumSum | 
        Function::CumProd | Function::CumMin | Function::CumMax => {
            if args.is_empty() {
                return Err(Error::eval(format!("{:?} requires at least 1 argument", func)));
            }
            
            // Collect all values (current + historical)
            let mut values = Vec::new();
            let current = evaluate_expr(&args[0], context)?;
            values.push(current);
            
            // Add historical values
            if let ExprNode::Column(node_name) = &args[0].node {
                for (_period, period_values) in &context.historical_results {
                    if let Some(value) = period_values.get(node_name) {
                        values.push(*value);
                    }
                }
            }
            
            if values.is_empty() {
                return Ok(0.0);
            }
            
            match func {
                Function::Std => {
                    // Standard deviation
                    let mean = values.iter().sum::<f64>() / values.len() as f64;
                    let variance = values.iter()
                        .map(|v| (v - mean).powi(2))
                        .sum::<f64>() / values.len() as f64;
                    Ok(variance.sqrt())
                }
                Function::Var => {
                    // Variance
                    let mean = values.iter().sum::<f64>() / values.len() as f64;
                    Ok(values.iter()
                        .map(|v| (v - mean).powi(2))
                        .sum::<f64>() / values.len() as f64)
                }
                Function::Median => {
                    // Median
                    values.sort_by(|a, b| a.partial_cmp(b).unwrap());
                    if values.len() % 2 == 0 {
                        Ok((values[values.len() / 2 - 1] + values[values.len() / 2]) / 2.0)
                    } else {
                        Ok(values[values.len() / 2])
                    }
                }
                Function::CumSum => {
                    // Cumulative sum up to current period
                    Ok(values.iter().sum())
                }
                Function::CumProd => {
                    // Cumulative product
                    Ok(values.iter().product())
                }
                Function::CumMin => {
                    // Cumulative minimum
                    Ok(values.iter().fold(f64::INFINITY, |a, b| a.min(*b)))
                }
                Function::CumMax => {
                    // Cumulative maximum
                    Ok(values.iter().fold(f64::NEG_INFINITY, |a, b| a.max(*b)))
                }
                _ => unreachable!(),
            }
        }
        
        // Other functions
        Function::Shift => {
            // Similar to lag but with different semantics
            if args.len() != 2 {
                return Err(Error::eval("shift() requires 2 arguments"));
            }
            let shift_periods = evaluate_expr(&args[1], context)? as i32;
            
            if shift_periods == 0 {
                evaluate_expr(&args[0], context)
            } else {
                // For now, return 0 for shifted values
                Ok(0.0)
            }
        }
        
        Function::Rank | Function::Quantile | Function::EwmMean | 
        Function::EwmStd | Function::EwmVar => {
            // These require more complex implementations
            // For now, return a placeholder value
            evaluate_expr(&args[0], context)
        }
    }
}
