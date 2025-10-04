//! Formula evaluation logic.
//!
//! # Design Note: Arithmetic Operations Handling
//!
//! This module evaluates arithmetic operations (Add, Sub, Mul, Div, Mod) directly
//! rather than delegating them to finstack-core's Function enum. This design choice:
//!
//! 1. **Maintains separation of concerns**: Core focuses on financial/statistical
//!    functions while basic arithmetic is handled locally.
//!
//! 2. **Optimizes performance**: Direct evaluation avoids function dispatch overhead
//!    for the most common operations.
//!
//! 3. **Provides flexibility**: Each crate can tailor arithmetic evaluation to its
//!    specific needs (numeric types, precision, error handling).
//!
//! While this means arithmetic follows a different evaluation path than advanced
//! functions, both paths are well-tested and produce consistent results.

use crate::error::{Error, Result};
use crate::evaluator::context::EvaluationContext;
use finstack_core::dates::PeriodId;
use finstack_core::expr::{Expr, ExprNode, Function};
use std::collections::BTreeMap;

/// Epsilon value for floating point comparisons
const EPSILON: f64 = 1e-10;

/// Evaluate a compiled expression.
///
/// Handles both basic arithmetic operations (evaluated directly) and
/// advanced financial/statistical functions (delegated to specialized handlers).
pub(crate) fn evaluate_formula(expr: &Expr, context: &EvaluationContext) -> Result<f64> {
    evaluate_expr(expr, context)
}

/// Collect values for a rolling window in chronological order.
/// Returns values from oldest to newest within the window.
fn collect_rolling_window_values(
    node_name: &str,
    context: &EvaluationContext,
    window_size: usize,
) -> Result<Vec<f64>> {
    if window_size == 0 {
        return Ok(Vec::new());
    }

    // Use BTreeMap to sort periods chronologically
    let mut sorted_periods = BTreeMap::new();

    // Add historical values
    for (period, values) in &context.historical_results {
        if let Some(value) = values.get(node_name) {
            sorted_periods.insert(*period, *value);
        }
    }

    // Add current period value if it exists
    if let Ok(current) = context.get_value(node_name) {
        sorted_periods.insert(context.period_id, current);
    }

    // Collect the most recent `window_size` values
    let mut values: Vec<f64> = sorted_periods
        .into_iter()
        .rev() // Most recent first
        .take(window_size)
        .map(|(_, v)| v)
        .collect();

    // Reverse to get chronological order (oldest to newest)
    values.reverse();

    Ok(values)
}

/// Collect all historical values for a node including current.
fn collect_all_historical_values(node_name: &str, context: &EvaluationContext) -> Result<Vec<f64>> {
    // Use BTreeMap to sort periods chronologically
    let mut sorted_periods = BTreeMap::new();

    // Add historical values
    for (period, values) in &context.historical_results {
        if let Some(value) = values.get(node_name) {
            sorted_periods.insert(*period, *value);
        }
    }

    // Add current period value if it exists
    if let Ok(current) = context.get_value(node_name) {
        sorted_periods.insert(context.period_id, current);
    }

    // Return values in chronological order
    Ok(sorted_periods.into_values().collect())
}

/// Calculate mean of values.
fn calculate_mean(values: &[f64]) -> Result<f64> {
    if values.is_empty() {
        return Ok(f64::NAN);
    }
    Ok(values.iter().sum::<f64>() / values.len() as f64)
}

/// Calculate standard deviation of values.
fn calculate_std(values: &[f64]) -> Result<f64> {
    if values.len() <= 1 {
        return Ok(0.0);
    }
    let variance = calculate_variance(values)?;
    Ok(variance.sqrt())
}

/// Calculate variance of values.
fn calculate_variance(values: &[f64]) -> Result<f64> {
    if values.is_empty() {
        return Ok(f64::NAN);
    }
    if values.len() == 1 {
        return Ok(0.0);
    }
    let mean = calculate_mean(values)?;
    Ok(values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64)
}

/// Calculate median of values.
fn calculate_median(values: &[f64]) -> Result<f64> {
    if values.is_empty() {
        return Ok(f64::NAN);
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let len = sorted.len();
    if len % 2 == 0 {
        Ok((sorted[len / 2 - 1] + sorted[len / 2]) / 2.0)
    } else {
        Ok(sorted[len / 2])
    }
}

/// Helper to offset a PeriodId by N periods.
/// Positive offset goes forward, negative goes backward.
fn offset_period(period: PeriodId, offset: i32) -> PeriodId {
    if offset == 0 {
        return period;
    }

    let mut result = period;
    let steps = offset.unsigned_abs() as usize;

    for _ in 0..steps {
        if offset > 0 {
            // Move forward
            result = step_forward(result);
        } else {
            // Move backward
            result = step_backward(result);
        }
    }

    result
}

/// Move a period forward by one step.
fn step_forward(id: PeriodId) -> PeriodId {
    match id {
        PeriodId { year, index, .. } if id.to_string().contains('Q') => {
            // Quarterly
            if index == 4 {
                PeriodId::quarter(year + 1, 1)
            } else {
                PeriodId::quarter(year, index + 1)
            }
        }
        PeriodId { year, index, .. } if id.to_string().contains('M') => {
            // Monthly
            if index == 12 {
                PeriodId::month(year + 1, 1)
            } else {
                PeriodId::month(year, index + 1)
            }
        }
        PeriodId { year, index, .. } if id.to_string().contains('W') => {
            // Weekly
            if index >= 52 {
                PeriodId::week(year + 1, 1)
            } else {
                PeriodId::week(year, index + 1)
            }
        }
        PeriodId { year, index, .. } if id.to_string().contains('H') => {
            // Half-year / Semi-annual
            if index == 2 {
                PeriodId::half(year + 1, 1)
            } else {
                PeriodId::half(year, 2)
            }
        }
        PeriodId { year, .. } => {
            // Annual
            PeriodId::annual(year + 1)
        }
    }
}

/// Move a period backward by one step.
fn step_backward(id: PeriodId) -> PeriodId {
    match id {
        PeriodId { year, index, .. } if id.to_string().contains('Q') => {
            // Quarterly
            if index == 1 {
                PeriodId::quarter(year - 1, 4)
            } else {
                PeriodId::quarter(year, index - 1)
            }
        }
        PeriodId { year, index, .. } if id.to_string().contains('M') => {
            // Monthly
            if index == 1 {
                PeriodId::month(year - 1, 12)
            } else {
                PeriodId::month(year, index - 1)
            }
        }
        PeriodId { year, index, .. } if id.to_string().contains('W') => {
            // Weekly
            if index == 1 {
                PeriodId::week(year - 1, 52)
            } else {
                PeriodId::week(year, index - 1)
            }
        }
        PeriodId { year, index, .. } if id.to_string().contains('H') => {
            // Half-year / Semi-annual
            if index == 1 {
                PeriodId::half(year - 1, 2)
            } else {
                PeriodId::half(year, 1)
            }
        }
        PeriodId { year, .. } => {
            // Annual
            PeriodId::annual(year - 1)
        }
    }
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
            // Note: Binary operations are evaluated directly here rather than
            // through the Function enum. This is intentional - see module docs.
            let left_val = evaluate_expr(left, context)?;
            let right_val = evaluate_expr(right, context)?;

            let result = match op {
                // Arithmetic operations - evaluated directly for performance
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
                return Err(Error::eval(
                    "lag() requires 2 arguments (expression, periods)",
                ));
            }

            // Get the number of periods to lag
            let lag_periods = evaluate_expr(&args[1], context)? as i32;
            if lag_periods < 0 {
                return Err(Error::eval("lag() periods must be non-negative"));
            }

            if lag_periods == 0 {
                // No lag, just evaluate the expression
                return evaluate_expr(&args[0], context);
            }

            // Calculate the target period
            let target_period = offset_period(context.period_id, -lag_periods);

            // If it's a simple column reference, look it up in historical results
            if let ExprNode::Column(node_name) = &args[0].node {
                if let Some(value) = context.get_historical_value(node_name, &target_period) {
                    Ok(value)
                } else {
                    // No historical value found, return NaN
                    Ok(f64::NAN)
                }
            } else {
                // For complex expressions, we can't easily evaluate them in a different period context
                // Return NaN to indicate the value is not available
                Ok(f64::NAN)
            }
        }
        Function::Lead => {
            // Lead function is intentionally not supported in financial modeling
            // to prevent forward-looking bias in time series analysis
            Err(Error::eval("lead() function is not available (forward-looking operations are not supported in financial modeling)"))
        }
        Function::Diff => {
            if args.is_empty() || args.len() > 2 {
                return Err(Error::eval(
                    "diff() requires 1 or 2 arguments (expression, [periods])",
                ));
            }

            // Get the lag periods (default to 1)
            let lag_periods = if args.len() == 2 {
                evaluate_expr(&args[1], context)? as i32
            } else {
                1
            };

            if lag_periods <= 0 {
                return Err(Error::eval("diff() periods must be positive"));
            }

            // For column references, check if value exists in current period
            if let ExprNode::Column(node_name) = &args[0].node {
                // Get current value
                let current_value = context.get_value(node_name).unwrap_or(f64::NAN);
                if current_value.is_nan() {
                    // No current value, return NaN
                    return Ok(f64::NAN);
                }

                // Get the lagged value
                let target_period = offset_period(context.period_id, -lag_periods);
                if let Some(lagged_value) = context.get_historical_value(node_name, &target_period)
                {
                    Ok(current_value - lagged_value)
                } else {
                    // No historical value, return NaN
                    Ok(f64::NAN)
                }
            } else {
                // For complex expressions, evaluate current value
                let _current_value = evaluate_expr(&args[0], context)?;
                // Can't get historical value for complex expressions
                Ok(f64::NAN)
            }
        }
        Function::PctChange => {
            if args.is_empty() || args.len() > 2 {
                return Err(Error::eval(
                    "pct_change() requires 1 or 2 arguments (expression, [periods])",
                ));
            }

            // Get the lag periods (default to 1)
            let lag_periods = if args.len() == 2 {
                evaluate_expr(&args[1], context)? as i32
            } else {
                1
            };

            if lag_periods <= 0 {
                return Err(Error::eval("pct_change() periods must be positive"));
            }

            // For column references, check if value exists in current period
            if let ExprNode::Column(node_name) = &args[0].node {
                // Get current value
                let current_value = context.get_value(node_name).unwrap_or(f64::NAN);
                if current_value.is_nan() {
                    // No current value, return NaN
                    return Ok(f64::NAN);
                }

                // Get the lagged value
                let target_period = offset_period(context.period_id, -lag_periods);
                if let Some(lagged_value) = context.get_historical_value(node_name, &target_period)
                {
                    if lagged_value.abs() < EPSILON {
                        // Avoid division by zero
                        Ok(f64::NAN)
                    } else {
                        Ok((current_value - lagged_value) / lagged_value)
                    }
                } else {
                    // No historical value, return NaN
                    Ok(f64::NAN)
                }
            } else {
                // For complex expressions, evaluate current value
                let _current_value = evaluate_expr(&args[0], context)?;
                // Can't get historical value for complex expressions
                Ok(f64::NAN)
            }
        }
        // Rolling window functions
        Function::RollingMean
        | Function::RollingSum
        | Function::RollingStd
        | Function::RollingVar
        | Function::RollingMedian
        | Function::RollingMin
        | Function::RollingMax
        | Function::RollingCount => {
            if args.len() != 2 {
                return Err(Error::eval(format!(
                    "{:?} requires 2 arguments (expression, window)",
                    func
                )));
            }

            let window = evaluate_expr(&args[1], context)? as usize;
            if window == 0 {
                return Err(Error::eval("Window size must be greater than 0"));
            }

            // Collect values in chronological order for the rolling window
            let values = if let ExprNode::Column(node_name) = &args[0].node {
                collect_rolling_window_values(node_name, context, window)?
            } else {
                // For complex expressions, just use current value
                vec![evaluate_expr(&args[0], context)?]
            };

            if values.is_empty() {
                return Ok(0.0);
            }

            match func {
                Function::RollingMean => calculate_mean(&values),
                Function::RollingSum => Ok(values.iter().sum()),
                Function::RollingStd => calculate_std(&values),
                Function::RollingVar => calculate_variance(&values),
                Function::RollingMedian => calculate_median(&values),
                Function::RollingMin => Ok(values.iter().fold(f64::INFINITY, |a, b| a.min(*b))),
                Function::RollingMax => Ok(values.iter().fold(f64::NEG_INFINITY, |a, b| a.max(*b))),
                Function::RollingCount => Ok(values.len() as f64),
                _ => Err(Error::eval(format!(
                    "Function {:?} is not a rolling window function",
                    func
                ))),
            }
        }

        // Statistical functions (operate on all historical values)
        Function::Std | Function::Var | Function::Median => {
            if args.is_empty() {
                return Err(Error::eval(format!(
                    "{:?} requires at least 1 argument",
                    func
                )));
            }

            // Collect all historical values
            let values = if let ExprNode::Column(node_name) = &args[0].node {
                collect_all_historical_values(node_name, context)?
            } else {
                // For complex expressions, just use current value
                vec![evaluate_expr(&args[0], context)?]
            };

            match func {
                Function::Std => calculate_std(&values),
                Function::Var => calculate_variance(&values),
                Function::Median => calculate_median(&values),
                _ => Err(Error::eval(format!(
                    "Function {:?} is not a statistical function",
                    func
                ))),
            }
        }

        // Cumulative functions (operate on all historical values)
        Function::CumSum | Function::CumProd | Function::CumMin | Function::CumMax => {
            if args.is_empty() {
                return Err(Error::eval(format!(
                    "{:?} requires at least 1 argument",
                    func
                )));
            }

            // Collect all historical values
            let values = if let ExprNode::Column(node_name) = &args[0].node {
                collect_all_historical_values(node_name, context)?
            } else {
                // For complex expressions, just use current value
                vec![evaluate_expr(&args[0], context)?]
            };

            if values.is_empty() {
                return Ok(0.0);
            }

            match func {
                Function::CumSum => Ok(values.iter().sum()),
                Function::CumProd => Ok(values.iter().product()),
                Function::CumMin => Ok(values.iter().fold(f64::INFINITY, |a, b| a.min(*b))),
                Function::CumMax => Ok(values.iter().fold(f64::NEG_INFINITY, |a, b| a.max(*b))),
                _ => Err(Error::eval(format!(
                    "Function {:?} is not a cumulative function",
                    func
                ))),
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

        Function::Rank => {
            // Rank the current value among all historical values
            if args.is_empty() {
                return Err(Error::eval("rank() requires at least one argument"));
            }

            // Get the value to rank
            let current_value = evaluate_expr(&args[0], context)?;

            // Collect all values (historical + current)
            let node_name = if let ExprNode::Column(name) = &args[0].node {
                name
            } else {
                return Ok(1.0); // Non-column expressions get rank 1
            };

            let mut all_values = collect_all_historical_values(node_name, context)?;
            all_values.push(current_value);

            // Sort values in ascending order
            all_values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

            // Find rank (1-based)
            let rank = all_values
                .iter()
                .position(|&v| (v - current_value).abs() < EPSILON)
                .map(|pos| (pos + 1) as f64)
                .unwrap_or(1.0);

            Ok(rank)
        }

        Function::Quantile => {
            // Calculate quantile of a value in a distribution
            if args.len() < 2 {
                return Err(Error::eval(
                    "quantile() requires 2 arguments: node and quantile",
                ));
            }

            // Get the quantile level (e.g., 0.25 for 25th percentile)
            let quantile = evaluate_expr(&args[1], context)?;
            if !(0.0..=1.0).contains(&quantile) {
                return Err(Error::eval("quantile must be between 0 and 1"));
            }

            // Get node name for historical data
            let node_name = if let ExprNode::Column(name) = &args[0].node {
                name
            } else {
                return Err(Error::eval("quantile() requires a column reference"));
            };

            // Collect and sort all values
            let mut all_values = collect_all_historical_values(node_name, context)?;
            if let Ok(current) = context.get_value(node_name) {
                all_values.push(current);
            }

            if all_values.is_empty() {
                return Ok(f64::NAN);
            }

            all_values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

            // Calculate quantile value
            let n = all_values.len() as f64;
            let index = quantile * (n - 1.0);
            let lower = index.floor() as usize;
            let upper = index.ceil() as usize;

            if lower == upper {
                Ok(all_values[lower])
            } else {
                // Linear interpolation
                let weight = index - lower as f64;
                Ok(all_values[lower] * (1.0 - weight) + all_values[upper] * weight)
            }
        }

        Function::EwmMean => {
            // Exponentially weighted moving average
            if args.len() < 2 {
                return Err(Error::eval(
                    "ewm_mean() requires 2 arguments: node and alpha",
                ));
            }

            // Get smoothing factor (alpha)
            let alpha = evaluate_expr(&args[1], context)?;
            if !(0.0..=1.0).contains(&alpha) {
                return Err(Error::eval("ewm_mean alpha must be between 0 and 1"));
            }

            // Get node name
            let node_name = if let ExprNode::Column(name) = &args[0].node {
                name
            } else {
                return Err(Error::eval("ewm_mean() requires a column reference"));
            };

            // Collect historical values in chronological order
            let mut values = Vec::new();
            for (period_id, period_results) in &context.historical_results {
                if let Some(value) = period_results.get(node_name) {
                    values.push((*period_id, *value));
                }
            }

            // Add current value
            if let Ok(current) = context.get_value(node_name) {
                values.push((context.period_id, current));
            }

            if values.is_empty() {
                return Ok(f64::NAN);
            }

            // Sort by period (chronological)
            values.sort_by_key(|(period, _)| *period);

            // Calculate EWM using the formula: EWM_t = alpha * x_t + (1 - alpha) * EWM_{t-1}
            let mut ewm = values[0].1; // Initialize with first value
            for (_, value) in values.iter().skip(1) {
                ewm = alpha * value + (1.0 - alpha) * ewm;
            }

            Ok(ewm)
        }

        // Custom financial functions with NaN handling
        Function::Sum => {
            // Sum multiple values, skipping NaN
            if args.is_empty() {
                return Err(Error::eval("sum() requires at least one argument"));
            }

            let mut sum = 0.0;
            let mut has_valid = false;

            for arg in args {
                let value = evaluate_expr(arg, context)?;
                if !value.is_nan() {
                    sum += value;
                    has_valid = true;
                }
            }

            if has_valid {
                Ok(sum)
            } else {
                Ok(f64::NAN)
            }
        }

        Function::Mean => {
            // Average of multiple values, skipping NaN
            if args.is_empty() {
                return Err(Error::eval("mean() requires at least one argument"));
            }

            let mut sum = 0.0;
            let mut count = 0;

            for arg in args {
                let value = evaluate_expr(arg, context)?;
                if !value.is_nan() {
                    sum += value;
                    count += 1;
                }
            }

            if count > 0 {
                Ok(sum / count as f64)
            } else {
                Ok(f64::NAN)
            }
        }

        Function::Ttm => {
            // Trailing twelve months: rolling sum with window of 4 (for quarterly periods)
            if args.len() != 1 {
                return Err(Error::eval("ttm() requires exactly 1 argument"));
            }

            // For column references, get rolling sum
            if let ExprNode::Column(node_name) = &args[0].node {
                let values = collect_rolling_window_values(node_name, context, 4)?;
                let sum: f64 = values.iter().filter(|v| !v.is_nan()).sum();
                Ok(sum)
            } else {
                // For complex expressions, just evaluate current value * 4
                let value = evaluate_expr(&args[0], context)?;
                if value.is_nan() {
                    Ok(f64::NAN)
                } else {
                    Ok(value * 4.0)
                }
            }
        }

        Function::Annualize => {
            // Annualize a value: value * periods_per_year
            if args.len() != 2 {
                return Err(Error::eval(
                    "annualize() requires 2 arguments (value, periods_per_year)",
                ));
            }

            let value = evaluate_expr(&args[0], context)?;
            let periods_per_year = evaluate_expr(&args[1], context)?;

            if value.is_nan() || periods_per_year.is_nan() {
                Ok(f64::NAN)
            } else {
                Ok(value * periods_per_year)
            }
        }

        Function::Coalesce => {
            // Return first non-NaN/non-zero value
            if args.len() < 2 {
                return Err(Error::eval("coalesce() requires at least 2 arguments"));
            }

            for arg in args {
                let value = evaluate_expr(arg, context)?;
                if !value.is_nan() && value != 0.0 {
                    return Ok(value);
                }
            }

            // If all values are NaN or zero, return the last one
            evaluate_expr(&args[args.len() - 1], context)
        }

        Function::EwmStd | Function::EwmVar => {
            // Exponentially weighted standard deviation/variance
            if args.len() < 2 {
                return Err(Error::eval(
                    "ewm_std/var() requires 2 arguments: node and alpha",
                ));
            }

            // Get smoothing factor (alpha)
            let alpha = evaluate_expr(&args[1], context)?;
            if !(0.0..=1.0).contains(&alpha) {
                return Err(Error::eval("ewm alpha must be between 0 and 1"));
            }

            // Get node name
            let node_name = if let ExprNode::Column(name) = &args[0].node {
                name
            } else {
                return Err(Error::eval("ewm_std/var() requires a column reference"));
            };

            // Collect historical values
            let mut values = Vec::new();
            for (period_id, period_results) in &context.historical_results {
                if let Some(value) = period_results.get(node_name) {
                    values.push((*period_id, *value));
                }
            }

            // Add current value
            if let Ok(current) = context.get_value(node_name) {
                values.push((context.period_id, current));
            }

            if values.len() < 2 {
                return Ok(f64::NAN);
            }

            // Sort by period
            values.sort_by_key(|(period, _)| *period);

            // Calculate EWM mean first
            let mut ewm_mean = values[0].1;
            let mut ewm_var = 0.0;

            for (_, value) in values.iter().skip(1) {
                let diff = value - ewm_mean;
                ewm_mean = alpha * value + (1.0 - alpha) * ewm_mean;
                ewm_var = (1.0 - alpha) * (ewm_var + alpha * diff * diff);
            }

            match func {
                Function::EwmVar => Ok(ewm_var),
                Function::EwmStd => Ok(ewm_var.sqrt()),
                Function::EwmMean => Ok(ewm_mean),
                _ => Err(Error::eval(format!(
                    "Function {:?} is not an exponentially weighted function",
                    func
                ))),
            }
        }
    }
}
