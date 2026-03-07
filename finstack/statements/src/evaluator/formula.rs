//! Evaluate compiled formulas against an evaluation context.
//!
//! Arithmetic operators are handled locally for performance and separation of
//! concerns, while statistical/time-series functions delegate to the shared
//! `finstack-core` helpers.
//!
//! # Numerical Behavior
//!
//! ## NaN Handling
//! - Division by zero → NaN (with log warning)
//! - Missing historical values in lag/shift → NaN
//! - Insufficient data for variance (< 2 values) → NaN
//! - pct_change with near-zero denominator → NaN (with log warning)
//!
//! ## Overflow Protection
//! - Compound growth (`growth_pct`) errors on overflow
//! - Growth rates > 100% produce warnings
//!
//! ## Precision
//! - Equality comparisons use [`EPSILON`] from [`crate::utils::constants`]
//! - Suitable for rate comparisons (0.01 bp precision)
//! - Monetary comparisons should use the `Money` type for currency safety

use crate::error::{Error, Result};
use crate::evaluator::context::EvaluationContext;
use crate::evaluator::results::EvalWarning;
use crate::utils::constants::EPSILON;
use finstack_core::dates::{PeriodId, PeriodKind};
use finstack_core::expr::{Expr, ExprNode, Function};
use finstack_core::math::{kahan_sum, neumaier_sum};
use std::collections::BTreeMap;

fn annotate_error(err: Error, node_id: Option<&str>) -> Error {
    match (node_id, err) {
        (Some(id), Error::Eval(msg)) => {
            if msg.starts_with("[node ") {
                Error::Eval(msg)
            } else {
                Error::Eval(format!("[node {}] {}", id, msg))
            }
        }
        (_, other) => other,
    }
}

fn eval_error(node_id: Option<&str>, msg: impl Into<String>) -> Error {
    annotate_error(Error::eval(msg), node_id)
}

fn map_err_with_node<T, E>(res: std::result::Result<T, E>, node_id: Option<&str>) -> Result<T>
where
    E: Into<Error>,
{
    res.map_err(|err| annotate_error(err.into(), node_id))
}

/// Convert boolean to f64 (1.0 for true, 0.0 for false).
#[inline]
fn bool_to_f64(b: bool) -> f64 {
    if b {
        1.0
    } else {
        0.0
    }
}

/// Validate that a function has exactly the expected number of arguments.
#[inline]
fn require_args(
    func_name: &str,
    args: &[Expr],
    expected: usize,
    node_id: Option<&str>,
) -> Result<()> {
    if args.len() != expected {
        return Err(eval_error(
            node_id,
            format!(
                "{}() requires exactly {} argument{}",
                func_name,
                expected,
                if expected == 1 { "" } else { "s" }
            ),
        ));
    }
    Ok(())
}

/// Validate that a function has at least the minimum number of arguments.
#[inline]
fn require_min_args(
    func_name: &str,
    args: &[Expr],
    min: usize,
    node_id: Option<&str>,
) -> Result<()> {
    if args.len() < min {
        return Err(eval_error(
            node_id,
            format!(
                "{}() requires at least {} argument{}",
                func_name,
                min,
                if min == 1 { "" } else { "s" }
            ),
        ));
    }
    Ok(())
}

#[inline]
fn evaluate_non_negative_integer_arg(
    func_name: &str,
    expr: &Expr,
    context: &mut EvaluationContext,
    node_id: Option<&str>,
) -> Result<i32> {
    let value = evaluate_expr(expr, context, node_id)?;
    if !value.is_finite() {
        return Err(eval_error(
            node_id,
            format!("{func_name}() requires a finite integer argument"),
        ));
    }
    if value.fract().abs() > EPSILON {
        return Err(eval_error(
            node_id,
            format!("{func_name}() requires an integer argument"),
        ));
    }

    let integer = value as i32;
    if integer < 0 {
        return Err(eval_error(
            node_id,
            format!("{func_name}() argument must be non-negative"),
        ));
    }
    Ok(integer)
}

#[inline]
fn evaluate_integer_arg(
    func_name: &str,
    expr: &Expr,
    context: &mut EvaluationContext,
    node_id: Option<&str>,
) -> Result<i32> {
    let value = evaluate_expr(expr, context, node_id)?;
    if !value.is_finite() {
        return Err(eval_error(
            node_id,
            format!("{func_name}() requires a finite integer argument"),
        ));
    }
    if value.fract().abs() > EPSILON {
        return Err(eval_error(
            node_id,
            format!("{func_name}() requires an integer argument"),
        ));
    }
    Ok(value as i32)
}

/// Evaluate a compiled expression.
///
/// Handles both basic arithmetic operations (evaluated directly) and
/// advanced financial/statistical functions (delegated to specialized handlers).
pub(crate) fn evaluate_formula(
    expr: &Expr,
    context: &mut EvaluationContext,
    node_id: Option<&str>,
) -> Result<f64> {
    evaluate_expr(expr, context, node_id)
}

/// Collect historical values sorted chronologically.
///
/// Returns a BTreeMap of period → value for all historical periods plus current.
/// This is a common helper used by rolling window and statistical functions.
fn collect_historical_values_sorted(
    node_name: &str,
    context: &EvaluationContext,
) -> Result<BTreeMap<PeriodId, f64>> {
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

    Ok(sorted_periods)
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

    let sorted = collect_historical_values_sorted(node_name, context)?;

    // Collect the most recent `window_size` values
    let mut values: Vec<f64> = sorted
        .into_values()
        .rev() // Most recent first
        .take(window_size)
        .collect();

    // Reverse to get chronological order (oldest to newest)
    values.reverse();

    Ok(values)
}

/// Collect all historical values for a node including current.
fn collect_all_historical_values(node_name: &str, context: &EvaluationContext) -> Result<Vec<f64>> {
    let sorted = collect_historical_values_sorted(node_name, context)?;
    Ok(sorted.into_values().collect())
}

/// Build a period-specific evaluation context so an expression can be
/// re-evaluated historically with the correct current/historical split.
fn build_context_for_period(
    target_period: PeriodId,
    context: &EvaluationContext,
) -> Result<EvaluationContext> {
    let historical_results = context
        .historical_results
        .iter()
        .filter(|(period, _)| **period < target_period)
        .map(|(period, values)| (*period, values.clone()))
        .collect();

    let current_period_values = if target_period == context.period_id {
        context
            .node_to_column
            .iter()
            .filter_map(|(node_id, idx)| {
                context.current_values[*idx].map(|value| (node_id.clone(), value))
            })
            .collect()
    } else {
        context
            .historical_results
            .get(&target_period)
            .cloned()
            .unwrap_or_default()
    };

    let mut period_context = EvaluationContext::new(
        target_period,
        context.node_to_column.clone(),
        historical_results,
    );
    period_context.node_value_types = context.node_value_types.clone();
    period_context.capital_structure_cashflows = context.capital_structure_cashflows.clone();

    for (node_id, value) in current_period_values {
        period_context.set_value(&node_id, value)?;
    }

    Ok(period_context)
}

/// Collect expression values over all available periods in chronological order.
fn collect_expression_values_sorted(
    expr: &Expr,
    context: &EvaluationContext,
    node_id: Option<&str>,
) -> Result<BTreeMap<PeriodId, f64>> {
    let mut periods = BTreeMap::new();
    for period in context.historical_results.keys() {
        periods.insert(*period, ());
    }
    periods.insert(context.period_id, ());

    let mut values = BTreeMap::new();
    for (period, _) in periods {
        let mut period_context = build_context_for_period(period, context)?;
        let value = evaluate_expr(expr, &mut period_context, node_id)?;
        values.insert(period, value);
    }

    Ok(values)
}

/// Collect expression values for a rolling window in chronological order.
fn collect_expression_window_values(
    expr: &Expr,
    context: &EvaluationContext,
    window_size: usize,
    node_id: Option<&str>,
) -> Result<Vec<f64>> {
    if window_size == 0 {
        return Ok(Vec::new());
    }

    let mut values: Vec<f64> = collect_expression_values_sorted(expr, context, node_id)?
        .into_values()
        .rev()
        .take(window_size)
        .collect();
    values.reverse();
    Ok(values)
}

/// Collect values for a node over a closed period range [start, end].
///
/// Periods are compared using their natural ordering. Values are returned in
/// chronological order (oldest → newest).
fn collect_period_range_values(
    node_name: &str,
    context: &EvaluationContext,
    start: PeriodId,
    end: PeriodId,
) -> Result<Vec<f64>> {
    let sorted = collect_historical_values_sorted(node_name, context)?;
    Ok(sorted
        .into_iter()
        .filter(|(period, _)| *period >= start && *period <= end)
        .map(|(_, value)| value)
        .collect())
}

/// Calculate mean of values.
fn calculate_mean(values: &[f64]) -> Result<f64> {
    if values.is_empty() {
        return Ok(f64::NAN);
    }
    Ok(kahan_sum(values.iter().copied()) / values.len() as f64)
}

/// Calculate standard deviation of values.
///
/// Uses sample standard deviation (sqrt of sample variance) per financial industry standards.
fn calculate_std(values: &[f64]) -> Result<f64> {
    if values.len() < 2 {
        return Ok(f64::NAN); // Undefined for < 2 values with sample variance
    }
    let variance = calculate_variance(values)?;
    Ok(variance.sqrt())
}

/// Calculate variance of values.
///
/// Uses sample variance (Bessel's correction with n-1 denominator) per financial industry standards.
/// This is the unbiased estimator required by Bloomberg, Excel VAR.S(), pandas.var(ddof=1), etc.
fn calculate_variance(values: &[f64]) -> Result<f64> {
    if values.is_empty() {
        return Ok(f64::NAN);
    }
    if values.len() == 1 {
        return Ok(f64::NAN); // Undefined for single value with sample variance
    }
    let mean = calculate_mean(values)?;
    // Use sample variance (n-1) per market standards (Bessel's correction)
    let squared_diffs = values.iter().map(|v| (v - mean).powi(2));
    Ok(kahan_sum(squared_diffs) / (values.len() - 1) as f64)
}

/// Calculate median of values.
fn calculate_median(values: &[f64]) -> Result<f64> {
    if values.is_empty() {
        return Ok(f64::NAN);
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let len = sorted.len();
    if len.is_multiple_of(2) {
        Ok((sorted[len / 2 - 1] + sorted[len / 2]) / 2.0)
    } else {
        Ok(sorted[len / 2])
    }
}

/// Helper to offset a PeriodId by N periods.
/// Positive offset goes forward, negative goes backward.
///
/// Now uses core API methods (next/prev) to avoid code duplication.
fn offset_period(period: PeriodId, offset: i32, node_id: Option<&str>) -> Result<PeriodId> {
    if offset == 0 {
        return Ok(period);
    }

    let mut result = period;
    let steps = offset.unsigned_abs() as usize;

    for _ in 0..steps {
        result = if offset > 0 {
            map_err_with_node(result.next(), node_id)?
        } else {
            map_err_with_node(result.prev(), node_id)?
        };
    }

    Ok(result)
}

/// Recursively evaluate an expression.
pub(crate) fn evaluate_expr(
    expr: &Expr,
    context: &mut EvaluationContext,
    node_id: Option<&str>,
) -> Result<f64> {
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
                    return map_err_with_node(
                        context.get_cs_value(component, instrument_or_total),
                        node_id,
                    );
                }
            }
            map_err_with_node(context.get_value(name), node_id)
        }
        ExprNode::Call(func, args) => evaluate_function(func, args, context, node_id),
        ExprNode::BinOp { op, left, right } => {
            // Note: Binary operations are evaluated directly here rather than
            // through the Function enum. This is intentional - see module docs.
            let left_val = evaluate_expr(left, context, node_id)?;
            let right_val = evaluate_expr(right, context, node_id)?;

            let result = match op {
                // Arithmetic operations - evaluated directly for performance
                BinOp::Add => left_val + right_val,
                BinOp::Sub => left_val - right_val,
                BinOp::Mul => left_val * right_val,
                BinOp::Div => {
                    if right_val == 0.0 {
                        tracing::warn!(
                            "Division by zero in formula evaluation (period: {:?})",
                            context.period_id
                        );
                        if let Some(id) = node_id {
                            context.push_warning(EvalWarning::DivisionByZero {
                                node_id: id.to_string(),
                                period: context.period_id,
                            });
                        }
                        f64::NAN
                    } else {
                        left_val / right_val
                    }
                }
                BinOp::Mod => left_val % right_val,

                // Comparison operations (use approximate equality for == and !=)
                BinOp::Eq => bool_to_f64((left_val - right_val).abs() <= EPSILON),
                BinOp::Ne => bool_to_f64((left_val - right_val).abs() > EPSILON),
                BinOp::Lt => bool_to_f64(left_val < right_val),
                BinOp::Le => bool_to_f64(left_val <= right_val),
                BinOp::Gt => bool_to_f64(left_val > right_val),
                BinOp::Ge => bool_to_f64(left_val >= right_val),

                // Logical operations
                BinOp::And => bool_to_f64(left_val != 0.0 && right_val != 0.0),
                BinOp::Or => bool_to_f64(left_val != 0.0 || right_val != 0.0),
            };
            Ok(result)
        }
        ExprNode::UnaryOp { op, operand } => {
            let val = evaluate_expr(operand, context, node_id)?;
            let result = match op {
                UnaryOp::Neg => -val,
                UnaryOp::Not => bool_to_f64(val == 0.0),
            };
            Ok(result)
        }
        ExprNode::IfThenElse {
            condition,
            then_expr,
            else_expr,
        } => {
            let cond_val = evaluate_expr(condition, context, node_id)?;
            if cond_val != 0.0 {
                evaluate_expr(then_expr, context, node_id)
            } else {
                evaluate_expr(else_expr, context, node_id)
            }
        }
    }
}

/// Evaluate a function call.
fn evaluate_function(
    func: &Function,
    args: &[Expr],
    context: &mut EvaluationContext,
    node_id: Option<&str>,
) -> Result<f64> {
    // Handle real functions from finstack-core
    match func {
        Function::Lag => {
            require_args("lag", args, 2, node_id)?;

            // Get the number of periods to lag
            let lag_periods = evaluate_non_negative_integer_arg("lag", &args[1], context, node_id)?;

            if lag_periods == 0 {
                // No lag, just evaluate the expression
                return evaluate_expr(&args[0], context, node_id);
            }

            // Calculate the target period
            let target_period = offset_period(context.period_id, -lag_periods, node_id)?;

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
            Err(eval_error(node_id, "lead() function is not available (forward-looking operations are not supported in financial modeling)"))
        }
        Function::Diff => {
            if args.is_empty() || args.len() > 2 {
                return Err(eval_error(
                    node_id,
                    "diff() requires 1 or 2 arguments (expression, [periods])",
                ));
            }

            // Get the lag periods (default to 1)
            let lag_periods = if args.len() == 2 {
                evaluate_non_negative_integer_arg("diff", &args[1], context, node_id)?
            } else {
                1
            };

            if lag_periods <= 0 {
                return Err(eval_error(node_id, "diff() periods must be positive"));
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
                let target_period = offset_period(context.period_id, -lag_periods, node_id)?;
                if let Some(lagged_value) = context.get_historical_value(node_name, &target_period)
                {
                    Ok(current_value - lagged_value)
                } else {
                    // No historical value, return NaN
                    Ok(f64::NAN)
                }
            } else {
                // For complex expressions, evaluate current value
                let _current_value = evaluate_expr(&args[0], context, node_id)?;
                // Can't get historical value for complex expressions
                Ok(f64::NAN)
            }
        }
        Function::PctChange => {
            if args.is_empty() || args.len() > 2 {
                return Err(eval_error(
                    node_id,
                    "pct_change() requires 1 or 2 arguments (expression, [periods])",
                ));
            }

            // Get the lag periods (default to 1)
            let lag_periods = if args.len() == 2 {
                evaluate_non_negative_integer_arg("pct_change", &args[1], context, node_id)?
            } else {
                1
            };

            if lag_periods <= 0 {
                return Err(eval_error(node_id, "pct_change() periods must be positive"));
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
                let target_period = offset_period(context.period_id, -lag_periods, node_id)?;
                if let Some(lagged_value) = context.get_historical_value(node_name, &target_period)
                {
                    if lagged_value.abs() < EPSILON {
                        // Avoid division by zero
                        tracing::warn!(
                            "pct_change() division by near-zero lagged value in period {:?}",
                            context.period_id
                        );
                        if let Some(id) = node_id {
                            context.push_warning(EvalWarning::DivisionByZero {
                                node_id: id.to_string(),
                                period: context.period_id,
                            });
                        }
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
                let _current_value = evaluate_expr(&args[0], context, node_id)?;
                // Can't get historical value for complex expressions
                Ok(f64::NAN)
            }
        }
        Function::GrowthRate => {
            if args.is_empty() || args.len() > 2 {
                return Err(eval_error(
                    node_id,
                    "growth_rate() requires 1 or 2 arguments (series, [periods])",
                ));
            }

            let periods_raw = if args.len() == 2 {
                evaluate_expr(&args[1], context, node_id)?
            } else {
                context.period_kind.periods_per_year() as f64
            };

            if !periods_raw.is_finite() || periods_raw <= 0.0 {
                return Err(eval_error(
                    node_id,
                    "growth_rate() periods must be a positive integer",
                ));
            }
            if periods_raw.fract() != 0.0 {
                return Err(eval_error(
                    node_id,
                    "growth_rate() periods must be a positive integer",
                ));
            }
            if periods_raw > i32::MAX as f64 {
                return Err(eval_error(
                    node_id,
                    "growth_rate() periods value is too large",
                ));
            }
            let periods = periods_raw as i32;

            if let ExprNode::Column(node_name) = &args[0].node {
                let current_value = context.get_value(node_name).unwrap_or(f64::NAN);
                if current_value.is_nan() {
                    return Ok(f64::NAN);
                }

                let target_period = offset_period(context.period_id, -periods, node_id)?;
                if let Some(start_value) = context.get_historical_value(node_name, &target_period) {
                    if start_value.abs() < EPSILON {
                        tracing::warn!(
                            "growth_rate() division by near-zero base value in period {:?}",
                            context.period_id
                        );
                        if let Some(id) = node_id {
                            context.push_warning(EvalWarning::DivisionByZero {
                                node_id: id.to_string(),
                                period: context.period_id,
                            });
                        }
                        return Ok(f64::NAN);
                    }
                    let ratio = current_value / start_value;
                    if !ratio.is_finite() {
                        return Ok(f64::NAN);
                    }
                    let exponent = 1.0 / periods as f64;
                    let growth = ratio.powf(exponent) - 1.0;
                    if growth.is_finite() {
                        Ok(growth)
                    } else {
                        Ok(f64::NAN)
                    }
                } else {
                    Ok(f64::NAN)
                }
            } else {
                Err(eval_error(
                    node_id,
                    "growth_rate() currently supports only simple column references; use an intermediate node for complex expressions",
                ))
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
            require_args(&format!("{:?}", func), args, 2, node_id)?;

            let window = evaluate_non_negative_integer_arg(
                &format!("{:?}", func),
                &args[1],
                context,
                node_id,
            )? as usize;
            if window == 0 {
                return Err(eval_error(node_id, "Window size must be greater than 0"));
            }

            // Collect values in chronological order for the rolling window
            let values = if let ExprNode::Column(node_name) = &args[0].node {
                collect_rolling_window_values(node_name, context, window)?
            } else {
                collect_expression_window_values(&args[0], context, window, node_id)?
            };

            if values.is_empty() {
                return Ok(f64::NAN); // Return NaN for insufficient data (market standard)
            }

            match func {
                Function::RollingMean => calculate_mean(&values),
                Function::RollingSum => Ok(neumaier_sum(values.iter().copied())),
                Function::RollingStd => calculate_std(&values),
                Function::RollingVar => calculate_variance(&values),
                Function::RollingMedian => calculate_median(&values),
                Function::RollingMin => Ok(values.iter().fold(f64::INFINITY, |a, b| a.min(*b))),
                Function::RollingMax => Ok(values.iter().fold(f64::NEG_INFINITY, |a, b| a.max(*b))),
                Function::RollingCount => Ok(values.len() as f64),
                _ => Err(eval_error(
                    node_id,
                    format!("Function {:?} is not a rolling window function", func),
                )),
            }
        }

        // Statistical functions (operate on all historical values)
        Function::Std | Function::Var | Function::Median => {
            require_min_args(&format!("{:?}", func), args, 1, node_id)?;

            // Collect all historical values
            let values = if let ExprNode::Column(node_name) = &args[0].node {
                collect_all_historical_values(node_name, context)?
            } else {
                collect_expression_values_sorted(&args[0], context, node_id)?
                    .into_values()
                    .collect()
            };

            match func {
                Function::Std => calculate_std(&values),
                Function::Var => calculate_variance(&values),
                Function::Median => calculate_median(&values),
                _ => Err(eval_error(
                    node_id,
                    format!("Function {:?} is not a statistical function", func),
                )),
            }
        }

        // Cumulative functions (operate on all historical values)
        Function::CumSum | Function::CumProd | Function::CumMin | Function::CumMax => {
            require_min_args(&format!("{:?}", func), args, 1, node_id)?;

            // Collect all historical values
            let values = if let ExprNode::Column(node_name) = &args[0].node {
                collect_all_historical_values(node_name, context)?
            } else {
                collect_expression_values_sorted(&args[0], context, node_id)?
                    .into_values()
                    .collect()
            };

            if values.is_empty() {
                return Ok(f64::NAN); // Return NaN for insufficient data (market standard)
            }

            match func {
                Function::CumSum => Ok(neumaier_sum(values.iter().copied())),
                Function::CumProd => {
                    // Use iterative multiplication with overflow detection.
                    // For long series with values > 1, naïve product can overflow
                    // to Inf. We detect non-finite intermediate results early.
                    let mut product = 1.0_f64;
                    for &v in &values {
                        product *= v;
                        if !product.is_finite() {
                            tracing::warn!(
                                "cumprod() overflow detected in period {:?}",
                                context.period_id
                            );
                            return Ok(f64::NAN);
                        }
                    }
                    Ok(product)
                }
                Function::CumMin => Ok(values.iter().fold(f64::INFINITY, |a, b| a.min(*b))),
                Function::CumMax => Ok(values.iter().fold(f64::NEG_INFINITY, |a, b| a.max(*b))),
                _ => Err(eval_error(
                    node_id,
                    format!("Function {:?} is not a cumulative function", func),
                )),
            }
        }

        // Other functions
        Function::Shift => {
            require_args("shift", args, 2, node_id)?;
            let shift_periods = evaluate_integer_arg("shift", &args[1], context, node_id)?;

            if shift_periods == 0 {
                return evaluate_expr(&args[0], context, node_id);
            }

            // Shift works like lag/lead: positive shift goes backward (like lag)
            // negative shift goes forward (like lead, but we'll return NaN for forward-looking)
            if shift_periods < 0 {
                // Forward-looking shifts return NaN (no peeking into the future)
                return Ok(f64::NAN);
            }

            // Calculate the target period (shift backward)
            let target_period = offset_period(context.period_id, -shift_periods, node_id)?;

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
                let _ = evaluate_expr(&args[0], context, node_id)?;
                Ok(f64::NAN)
            }
        }

        Function::Rank => {
            require_min_args("rank", args, 1, node_id)?;

            // Get the value to rank
            let current_value = evaluate_expr(&args[0], context, node_id)?;

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
            // Quantile/Percentile Calculation
            //
            // # Interpolation Method
            //
            // This implementation uses **linear interpolation** (equivalent to numpy's
            // `interpolation='linear'` or pandas' `interpolation='linear'`, also known
            // as R's type=7 quantile).
            //
            // The formula is:
            //   index = q * (n - 1)
            //   quantile = x[floor(index)] * (1 - frac) + x[ceil(index)] * frac
            //
            // where frac = index - floor(index).
            //
            // # Comparison with Other Methods
            //
            // - **R-1 to R-9**: R provides 9 different quantile types. This is R-7.
            // - **Excel PERCENTILE**: Uses a similar linear interpolation (R-7 equivalent).
            // - **numpy default**: Also uses linear interpolation (R-7 equivalent).
            // - **SciPy**: Defaults to R-9 (Blom's method) for some functions.
            //
            // For most financial applications, R-7/linear interpolation is appropriate
            // as it provides intuitive results and matches Excel behavior.
            //
            // # Edge Cases
            //
            // - q=0.0: Returns minimum value
            // - q=1.0: Returns maximum value
            // - n=1: Returns the single value for any q
            // - Empty data: Returns NaN
            require_args("quantile", args, 2, node_id)?;

            // Get the quantile level (e.g., 0.25 for 25th percentile)
            let quantile = evaluate_expr(&args[1], context, node_id)?;
            if !(0.0..=1.0).contains(&quantile) {
                return Err(eval_error(node_id, "quantile must be between 0 and 1"));
            }

            // Get node name for historical data
            let node_name = if let ExprNode::Column(name) = &args[0].node {
                name
            } else {
                return Err(eval_error(
                    node_id,
                    "quantile() requires a column reference",
                ));
            };

            // Collect and sort all values
            let mut all_values = collect_all_historical_values(node_name, context)?;

            if all_values.is_empty() {
                return Ok(f64::NAN);
            }

            all_values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

            // Calculate quantile using linear interpolation (R-7 / Excel / numpy default)
            let n = all_values.len() as f64;
            let index = quantile * (n - 1.0);
            let lower = index.floor() as usize;
            let upper = index.ceil() as usize;

            if lower == upper {
                Ok(all_values[lower])
            } else {
                // Linear interpolation between adjacent values
                let weight = index - lower as f64;
                Ok(all_values[lower] * (1.0 - weight) + all_values[upper] * weight)
            }
        }

        Function::EwmMean => {
            require_args("ewm_mean", args, 2, node_id)?;

            // Get smoothing factor (alpha)
            let alpha = evaluate_expr(&args[1], context, node_id)?;
            if !(0.0..=1.0).contains(&alpha) {
                return Err(eval_error(
                    node_id,
                    "ewm_mean alpha must be between 0 and 1",
                ));
            }

            // Get node name
            let node_name = if let ExprNode::Column(name) = &args[0].node {
                name
            } else {
                return Err(eval_error(
                    node_id,
                    "ewm_mean() requires a column reference",
                ));
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

        Function::Abs => {
            require_args("abs", args, 1, node_id)?;
            let value = evaluate_expr(&args[0], context, node_id)?;
            Ok(value.abs())
        }

        Function::Sign => {
            require_args("sign", args, 1, node_id)?;
            let value = evaluate_expr(&args[0], context, node_id)?;
            if value.is_nan() {
                Ok(f64::NAN)
            } else if value > 0.0 {
                Ok(1.0)
            } else if value < 0.0 {
                Ok(-1.0)
            } else {
                Ok(0.0)
            }
        }

        // Custom financial functions with NaN handling
        Function::Sum => {
            require_min_args("sum", args, 1, node_id)?;

            let mut values = Vec::new();

            for arg in args {
                let value = evaluate_expr(arg, context, node_id)?;
                if !value.is_nan() {
                    values.push(value);
                }
            }

            if values.is_empty() {
                Ok(f64::NAN)
            } else {
                Ok(kahan_sum(values.iter().copied()))
            }
        }

        Function::Mean => {
            require_min_args("mean", args, 1, node_id)?;

            let mut values = Vec::new();
            for arg in args {
                let value = evaluate_expr(arg, context, node_id)?;
                if !value.is_nan() {
                    values.push(value);
                }
            }

            if values.is_empty() {
                Ok(f64::NAN)
            } else {
                Ok(kahan_sum(values.iter().copied()) / values.len() as f64)
            }
        }

        Function::Ytd => {
            require_args("ytd", args, 1, node_id)?;

            let current = context.period_id;

            // Determine the first period of the calendar year for the current
            // frequency. This keeps semantics consistent across quarterly,
            // monthly, weekly, semi-annual, and annual models.
            let start_of_year = match context.period_kind {
                PeriodKind::Daily => PeriodId::day(current.year, 1),
                PeriodKind::Quarterly => PeriodId::quarter(current.year, 1),
                PeriodKind::Monthly => PeriodId::month(current.year, 1),
                PeriodKind::Weekly => PeriodId::week(current.year, 1),
                PeriodKind::SemiAnnual => PeriodId::half(current.year, 1),
                PeriodKind::Annual => PeriodId::annual(current.year),
            };

            if let ExprNode::Column(node_name) = &args[0].node {
                let values =
                    collect_period_range_values(node_name, context, start_of_year, current)?;
                let filtered: Vec<f64> = values.into_iter().filter(|v| !v.is_nan()).collect();
                Ok(neumaier_sum(filtered.iter().copied()))
            } else {
                Err(eval_error(
                    node_id,
                    "ytd() currently supports only simple column references; use an intermediate node for complex expressions",
                ))
            }
        }

        Function::Qtd => {
            require_args("qtd", args, 1, node_id)?;

            // QTD is defined only for monthly statement models.
            if context.period_kind != PeriodKind::Monthly {
                return Err(eval_error(
                    node_id,
                    "qtd() is only supported for monthly period models",
                ));
            }

            let current = context.period_id;
            let month = current.index as u32;
            let quarter_start_month = ((month - 1) / 3) * 3 + 1;
            let start = PeriodId::month(current.year, quarter_start_month as u8);
            let end = current;

            if let ExprNode::Column(node_name) = &args[0].node {
                let values = collect_period_range_values(node_name, context, start, end)?;
                let filtered: Vec<f64> = values.into_iter().filter(|v| !v.is_nan()).collect();
                Ok(neumaier_sum(filtered.iter().copied()))
            } else {
                Err(eval_error(
                    node_id,
                    "qtd() currently supports only simple column references; use an intermediate node for complex expressions",
                ))
            }
        }

        Function::FiscalYtd => {
            require_args("fiscal_ytd", args, 2, node_id)?;

            // Fiscal YTD is defined for monthly statement models, using a
            // configurable fiscal start month (1-12).
            if context.period_kind != PeriodKind::Monthly {
                return Err(eval_error(
                    node_id,
                    "fiscal_ytd() is only supported for monthly period models",
                ));
            }

            let start_month_raw = evaluate_expr(&args[1], context, node_id)?;
            if !start_month_raw.is_finite()
                || start_month_raw.fract() != 0.0
                || !(1.0..=12.0).contains(&start_month_raw)
            {
                return Err(eval_error(
                    node_id,
                    "fiscal_ytd() fiscal_start_month must be an integer between 1 and 12",
                ));
            }
            let start_month = start_month_raw as u8;

            let current = context.period_id;
            let current_month = current.index;

            // If the current month is on/after the fiscal start month, the
            // fiscal year starts in the current calendar year. Otherwise, it
            // started in the prior calendar year.
            let fiscal_start_year = if current_month >= start_month as u16 {
                current.year
            } else {
                current.year - 1
            };

            let start = PeriodId::month(fiscal_start_year, start_month);
            let end = current;

            if let ExprNode::Column(node_name) = &args[0].node {
                let values = collect_period_range_values(node_name, context, start, end)?;
                let filtered: Vec<f64> = values.into_iter().filter(|v| !v.is_nan()).collect();
                Ok(neumaier_sum(filtered.iter().copied()))
            } else {
                Err(eval_error(
                    node_id,
                    "fiscal_ytd() currently supports only simple column references; use an intermediate node for complex expressions",
                ))
            }
        }

        Function::Ttm => {
            require_args("ttm", args, 1, node_id)?;

            // Trailing Twelve Months (TTM) - Financial Reporting Standard
            //
            // TTM computes a rolling sum over a period-appropriate window:
            // - For quarterly periods: sums current period + prior 3 periods (4 periods total)
            // - For monthly periods: sums current period + prior 11 periods (12 periods total)
            // - For annual periods: returns current period value only (1 period total)
            //
            // Behavior:
            // 1. Window size = periods_per_year (4 for quarterly, 12 for monthly, 1 for annual)
            // 2. Includes current period + (N-1) historical periods in chronological order
            // 3. Incomplete windows (< N periods available): returns sum of available data
            // 4. NaN values are excluded from summation (skipped, not propagated)
            // 5. All-NaN window: returns 0.0
            //
            // Example (quarterly):
            // Period | Revenue | ttm(revenue)
            // -------|---------|-------------
            // 2024Q1 |   100   |    100       (only 1 period available)
            // 2024Q2 |   105   |    205       (2 periods: 100 + 105)
            // 2024Q3 |   110   |    315       (3 periods: 100 + 105 + 110)
            // 2024Q4 |   115   |    430       (4 periods: 100 + 105 + 110 + 115)
            // 2025Q1 |   120   |    450       (4 periods: 105 + 110 + 115 + 120)
            let window = context.period_kind.periods_per_year() as usize;

            // For column references, get rolling sum over appropriate window
            if let ExprNode::Column(node_name) = &args[0].node {
                let values = collect_rolling_window_values(node_name, context, window)?;
                let filtered: Vec<f64> = values.into_iter().filter(|v| !v.is_nan()).collect();
                Ok(neumaier_sum(filtered.iter().copied()))
            } else {
                let values = collect_expression_window_values(&args[0], context, window, node_id)?;
                let filtered: Vec<f64> = values.into_iter().filter(|v| !v.is_nan()).collect();
                Ok(neumaier_sum(filtered.iter().copied()))
            }
        }

        Function::Annualize => {
            if args.is_empty() || args.len() > 2 {
                return Err(eval_error(
                    node_id,
                    "annualize() requires 1 or 2 arguments (value, [periods_per_year])",
                ));
            }

            // Annualize a FLOW value (cash flows, income, expenses)
            // by multiplying by periods per year.
            //
            // For periodic RATES, use annualize_rate() instead.
            let value = evaluate_expr(&args[0], context, node_id)?;
            let periods_per_year = if args.len() == 2 {
                evaluate_expr(&args[1], context, node_id)?
            } else {
                context.period_kind.periods_per_year() as f64
            };

            if value.is_nan() || periods_per_year.is_nan() {
                Ok(f64::NAN)
            } else if periods_per_year <= 0.0 {
                Err(eval_error(
                    node_id,
                    "annualize() periods_per_year must be positive",
                ))
            } else {
                Ok(value * periods_per_year)
            }
        }

        Function::AnnualizeRate => {
            require_args("annualize_rate", args, 3, node_id)?;

            // Annualize a PERIODIC RATE (interest rates, returns, growth rates)
            // using either simple or compound methodology.
            //
            // Arguments:
            // - rate: Periodic rate (e.g., 0.02 for 2% quarterly return)
            // - periods_per_year: Number of periods in a year (4 for quarterly, 12 for monthly)
            // - compounding: 0.0 for simple, 1.0 for compound
            //
            // Simple:   annual_rate = periodic_rate × periods_per_year
            // Compound: annual_rate = (1 + periodic_rate)^periods_per_year - 1
            //
            // Examples:
            // - Quarterly return of 2%:
            //   Simple:   annualize_rate(0.02, 4, 0) = 0.08 (8%)
            //   Compound: annualize_rate(0.02, 4, 1) = 0.0824 (8.24%)
            let rate = evaluate_expr(&args[0], context, node_id)?;
            let periods_per_year = evaluate_expr(&args[1], context, node_id)?;
            let compounding = evaluate_expr(&args[2], context, node_id)?;

            if rate.is_nan() || periods_per_year.is_nan() || compounding.is_nan() {
                return Ok(f64::NAN);
            }

            if periods_per_year <= 0.0 {
                return Err(eval_error(
                    node_id,
                    "annualize_rate() periods_per_year must be positive",
                ));
            }

            // Determine methodology based on compounding parameter
            if compounding == 0.0 {
                // Simple annualization
                Ok(rate * periods_per_year)
            } else {
                // Compound annualization: (1 + rate)^periods - 1
                let result = (1.0 + rate).powf(periods_per_year) - 1.0;
                if result.is_finite() {
                    Ok(result)
                } else {
                    tracing::warn!(
                        "annualize_rate() overflow: (1 + {})^{} is not finite",
                        rate,
                        periods_per_year
                    );
                    Ok(f64::NAN)
                }
            }
        }

        Function::Coalesce => {
            require_min_args("coalesce", args, 2, node_id)?;

            for arg in args {
                let value = evaluate_expr(arg, context, node_id)?;
                if !value.is_nan() {
                    return Ok(value);
                }
            }

            // If all values are NaN, return the last one
            evaluate_expr(&args[args.len() - 1], context, node_id)
        }

        Function::EwmStd | Function::EwmVar => {
            // EWM variance and std support 2 or 3 arguments:
            // - 2 args: ewm_var(series, alpha) — bias-corrected (pandas adjust=True, market standard)
            // - 3 args: ewm_var(series, alpha, adjust) — bias correction enabled if adjust=1.0
            //
            // # Bias Correction (adjust=True)
            //
            // When adjust=True, we apply the bias correction factor at the end of the
            // computation, not iteratively inside the loop. This matches pandas semantics
            // for `ewm(..., adjust=True).var()`.
            //
            // The bias correction compensates for the fact that earlier observations have
            // exponentially decaying weights that don't sum to 1.0. The correction factor
            // is: 1 / (1 - (1-alpha)^n) where n is the number of observations.
            if args.len() < 2 || args.len() > 3 {
                return Err(eval_error(
                    node_id,
                    format!(
                        "{}() requires 2 or 3 arguments (series, alpha, [adjust])",
                        format!("{:?}", func).to_lowercase()
                    ),
                ));
            }

            // Get smoothing factor (alpha)
            let alpha = evaluate_expr(&args[1], context, node_id)?;
            if !(0.0..=1.0).contains(&alpha) {
                return Err(eval_error(node_id, "ewm alpha must be between 0 and 1"));
            }

            // Bias correction now defaults to `true` to match pandas adjust=True (market standard)
            let adjust = if args.len() == 3 {
                evaluate_expr(&args[2], context, node_id)? != 0.0
            } else {
                true
            };

            // Get node name
            let node_name = if let ExprNode::Column(name) = &args[0].node {
                name
            } else {
                return Err(eval_error(
                    node_id,
                    "ewm_std/var() requires a column reference",
                ));
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

            // Calculate EWM variance using the recursive formula:
            //   ewm_var_t = (1 - alpha) * (ewm_var_{t-1} + alpha * (x_t - ewm_mean_{t-1})^2)
            //
            // This is the non-bias-corrected (adjust=False) variance.
            let mut ewm_mean = values[0].1;
            let mut ewm_var = 0.0;

            for (_, value) in values.iter().skip(1) {
                let diff = value - ewm_mean;
                ewm_mean = alpha * value + (1.0 - alpha) * ewm_mean;
                ewm_var = (1.0 - alpha) * (ewm_var + alpha * diff * diff);
            }

            // Apply bias correction AFTER the loop if requested (pandas adjust=True)
            // This corrects for the fact that the sum of weights doesn't equal 1.0
            if adjust {
                let n = values.len();
                // Bias correction factor: 1 / (1 - (1-alpha)^n)
                // This accounts for the exponentially decaying weights not summing to 1
                let weight_sum = 1.0 - (1.0 - alpha).powi(n as i32);
                if weight_sum.abs() > EPSILON {
                    ewm_var /= weight_sum;
                }
            }

            match func {
                Function::EwmVar => Ok(ewm_var),
                Function::EwmStd => Ok(ewm_var.sqrt()),
                Function::EwmMean => Ok(ewm_mean),
                _ => Err(eval_error(
                    node_id,
                    format!(
                        "Function {:?} is not an exponentially weighted function",
                        func
                    ),
                )),
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use finstack_core::expr::{Expr, Function};
    use indexmap::IndexMap;

    fn build_context_with_history(
        current_period: PeriodId,
        node_id: &str,
        historical_values: Vec<(PeriodId, f64)>,
        current_value: f64,
    ) -> EvaluationContext {
        let mut node_to_column = IndexMap::new();
        node_to_column.insert(node_id.to_string(), 0);

        let mut historical = IndexMap::new();
        for (period, value) in historical_values {
            let mut values = IndexMap::new();
            values.insert(node_id.to_string(), value);
            historical.insert(period, values);
        }

        let mut context = EvaluationContext::new(current_period, node_to_column, historical);
        context
            .set_value(node_id, current_value)
            .expect("set node value");
        context
    }

    #[test]
    fn calculate_mean_matches_kahan_reference() {
        let mut values = vec![1e16];
        values.extend(std::iter::repeat_n(1.0, 256));

        let precise = calculate_mean(&values).expect("mean should succeed");
        let reference = kahan_sum(values.iter().copied()) / values.len() as f64;
        let naive = values.iter().sum::<f64>() / values.len() as f64;

        assert!((precise - reference).abs() < 1e-12);
        assert!(
            (naive - reference).abs() > 1e-6,
            "Expected naive mean to deviate from reference"
        );
    }

    #[test]
    fn ewm_var_defaults_to_bias_correction() {
        let p1 = PeriodId::quarter(2025, 1);
        let p2 = PeriodId::quarter(2025, 2);

        let mut context = build_context_with_history(p2, "series", vec![(p1, 1.0)], 2.0);
        let value_default = evaluate_function(
            &Function::EwmVar,
            &[Expr::column("series"), Expr::literal(0.5)],
            &mut context,
            Some("ewm_var"),
        )
        .expect("default ewm_var");

        let mut context_no_adjust = build_context_with_history(p2, "series", vec![(p1, 1.0)], 2.0);
        let value_no_adjust = evaluate_function(
            &Function::EwmVar,
            &[
                Expr::column("series"),
                Expr::literal(0.5),
                Expr::literal(0.0),
            ],
            &mut context_no_adjust,
            Some("ewm_var"),
        )
        .expect("ewm_var without adjust");

        assert!((value_default - 1.0 / 3.0).abs() < 1e-9);
        assert!((value_no_adjust - 0.25).abs() < 1e-9);
        assert!(value_default > value_no_adjust);
    }

    #[test]
    fn sum_function_handles_large_cancellations() {
        let period = PeriodId::quarter(2025, 1);
        let mut context = EvaluationContext::new(period, IndexMap::new(), IndexMap::new());
        let args = vec![
            Expr::literal(1e16),
            Expr::literal(1.0),
            Expr::literal(-1e16),
        ];
        let sum_value = evaluate_function(&Function::Sum, &args, &mut context, Some("sum_test"))
            .expect("sum evaluation should succeed");
        let reference = kahan_sum([1e16, 1.0, -1e16]);
        assert!(
            (sum_value - reference).abs() < 1e-12,
            "sum_value={sum_value}, reference={reference}"
        );
    }

    #[test]
    fn growth_rate_defaults_to_period_frequency() {
        let history = vec![
            (PeriodId::quarter(2024, 1), 100.0),
            (PeriodId::quarter(2024, 2), 110.0),
            (PeriodId::quarter(2024, 3), 121.0),
            (PeriodId::quarter(2024, 4), 133.1),
        ];
        let current_period = PeriodId::quarter(2025, 1);
        let mut context = build_context_with_history(current_period, "series", history, 146.41);

        let value = evaluate_function(
            &Function::GrowthRate,
            &[Expr::column("series")],
            &mut context,
            Some("series"),
        )
        .expect("growth_rate evaluation");

        assert!((value - 0.10).abs() < 1e-6, "value={value}");

        let explicit = evaluate_function(
            &Function::GrowthRate,
            &[Expr::column("series"), Expr::literal(2.0)],
            &mut context,
            Some("series"),
        )
        .expect("explicit periods");

        // Between Q1 2025 and Q1 2025 minus 2 quarters (Q3 2024)
        // Values: 146.41 vs 121 → CAGR over 2 periods ≈ 10%
        assert!((explicit - 0.10).abs() < 1e-6, "explicit={explicit}");
    }

    #[test]
    fn annualize_uses_period_kind_when_periods_missing() {
        let period = PeriodId::month(2025, 3);
        let mut context = EvaluationContext::new(period, IndexMap::new(), IndexMap::new());

        let default_factor = evaluate_function(
            &Function::Annualize,
            &[Expr::literal(2.5)],
            &mut context,
            Some("annualize"),
        )
        .expect("annualize default");

        assert!((default_factor - 30.0).abs() < 1e-9);

        let override_factor = evaluate_function(
            &Function::Annualize,
            &[Expr::literal(2.5), Expr::literal(4.0)],
            &mut context,
            Some("annualize"),
        )
        .expect("annualize override");

        assert!((override_factor - 10.0).abs() < 1e-9);
    }

    #[test]
    fn abs_and_sign_helpers_cover_edge_cases() {
        let period = PeriodId::quarter(2025, 1);
        let mut context = EvaluationContext::new(period, IndexMap::new(), IndexMap::new());

        let abs_val = evaluate_function(
            &Function::Abs,
            &[Expr::literal(-42.0)],
            &mut context,
            Some("abs"),
        )
        .expect("abs eval");
        assert_eq!(abs_val, 42.0);

        let sign_pos = evaluate_function(
            &Function::Sign,
            &[Expr::literal(3.5)],
            &mut context,
            Some("sign"),
        )
        .expect("sign positive");
        assert_eq!(sign_pos, 1.0);

        let sign_neg = evaluate_function(
            &Function::Sign,
            &[Expr::literal(-3.5)],
            &mut context,
            Some("sign"),
        )
        .expect("sign negative");
        assert_eq!(sign_neg, -1.0);

        let sign_zero = evaluate_function(
            &Function::Sign,
            &[Expr::literal(0.0)],
            &mut context,
            Some("sign"),
        )
        .expect("sign zero");
        assert_eq!(sign_zero, 0.0);

        let sign_nan = evaluate_function(
            &Function::Sign,
            &[Expr::literal(f64::NAN)],
            &mut context,
            Some("sign"),
        )
        .expect("sign nan");
        assert!(sign_nan.is_nan());
    }
}
