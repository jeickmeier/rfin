//! Evaluate compiled formulas against an evaluation context.
//!
//! Arithmetic operators are handled locally for performance and separation of
//! concerns, while statistical/time-series functions delegate to the shared
//! `finstack-core` helpers.

use crate::error::{Error, Result};
use crate::evaluator::context::EvaluationContext;
use finstack_core::dates::PeriodId;
use finstack_core::expr::{Expr, ExprNode, Function};
use std::collections::BTreeMap;

/// Epsilon value for floating point comparisons.
///
/// Set to 1e-10 for rate/ratio comparisons (basis point precision level).
/// This is appropriate for:
/// - Interest rate comparisons (0.01 basis point precision)
/// - Ratio comparisons (leverage, margins)
/// - Percentage change comparisons
/// - Equality comparisons in DSL (== and != operators)
///
/// # Breaking Change (v2.0)
///
/// As of version 2.0, the DSL `==` and `!=` operators use approximate equality
/// with this epsilon tolerance. This prevents spurious inequality from floating-point
/// rounding errors. For example:
/// - `100000.0 == 100000.0000000001` returns `true` (within epsilon)
/// - `100000.0 == 100001.0` returns `false` (exceeds epsilon)
///
/// Note: For currency comparisons, use the Money type and currency-safe logic, not raw f64 arithmetic.
const EPSILON: f64 = 1e-10;

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
fn require_args(func_name: &str, args: &[Expr], expected: usize) -> Result<()> {
    if args.len() != expected {
        return Err(Error::eval(format!(
            "{}() requires exactly {} argument{}",
            func_name,
            expected,
            if expected == 1 { "" } else { "s" }
        )));
    }
    Ok(())
}

/// Validate that a function has at least the minimum number of arguments.
#[inline]
fn require_min_args(func_name: &str, args: &[Expr], min: usize) -> Result<()> {
    if args.len() < min {
        return Err(Error::eval(format!(
            "{}() requires at least {} argument{}",
            func_name,
            min,
            if min == 1 { "" } else { "s" }
        )));
    }
    Ok(())
}

/// Evaluate a compiled expression.
///
/// Handles both basic arithmetic operations (evaluated directly) and
/// advanced financial/statistical functions (delegated to specialized handlers).
pub(crate) fn evaluate_formula(expr: &Expr, context: &EvaluationContext) -> Result<f64> {
    evaluate_expr(expr, context)
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

/// Calculate mean of values.
fn calculate_mean(values: &[f64]) -> Result<f64> {
    if values.is_empty() {
        return Ok(f64::NAN);
    }
    Ok(values.iter().sum::<f64>() / values.len() as f64)
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
    Ok(values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (values.len() - 1) as f64)
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
fn offset_period(period: PeriodId, offset: i32) -> Result<PeriodId> {
    if offset == 0 {
        return Ok(period);
    }

    let mut result = period;
    let steps = offset.unsigned_abs() as usize;

    for _ in 0..steps {
        result = if offset > 0 {
            result.next()?
        } else {
            result.prev()?
        };
    }

    Ok(result)
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
            let val = evaluate_expr(operand, context)?;
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
            require_args("lag", args, 2)?;

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
            let target_period = offset_period(context.period_id, -lag_periods)?;

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
                let target_period = offset_period(context.period_id, -lag_periods)?;
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
                let target_period = offset_period(context.period_id, -lag_periods)?;
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
            require_args(&format!("{:?}", func), args, 2)?;

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
                return Ok(f64::NAN); // Return NaN for insufficient data (market standard)
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
            require_min_args(&format!("{:?}", func), args, 1)?;

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
            require_min_args(&format!("{:?}", func), args, 1)?;

            // Collect all historical values
            let values = if let ExprNode::Column(node_name) = &args[0].node {
                collect_all_historical_values(node_name, context)?
            } else {
                // For complex expressions, just use current value
                vec![evaluate_expr(&args[0], context)?]
            };

            if values.is_empty() {
                return Ok(f64::NAN); // Return NaN for insufficient data (market standard)
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
            require_args("shift", args, 2)?;
            let shift_periods = evaluate_expr(&args[1], context)? as i32;

            if shift_periods == 0 {
                return evaluate_expr(&args[0], context);
            }

            // Shift works like lag/lead: positive shift goes backward (like lag)
            // negative shift goes forward (like lead, but we'll return NaN for forward-looking)
            if shift_periods < 0 {
                // Forward-looking shifts return NaN (no peeking into the future)
                return Ok(f64::NAN);
            }

            // Calculate the target period (shift backward)
            let target_period = offset_period(context.period_id, -shift_periods)?;

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

        Function::Rank => {
            require_min_args("rank", args, 1)?;

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
            require_args("quantile", args, 2)?;

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
            require_args("ewm_mean", args, 2)?;

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
            require_min_args("sum", args, 1)?;

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
            require_min_args("mean", args, 1)?;

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
            require_args("ttm", args, 1)?;

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
                let sum: f64 = values.iter().filter(|v| !v.is_nan()).sum();
                Ok(sum)
            } else {
                // For complex expressions, annualize by multiplying by periods per year
                let value = evaluate_expr(&args[0], context)?;
                if value.is_nan() {
                    Ok(f64::NAN)
                } else {
                    let annualization_factor = window as f64;
                    Ok(value * annualization_factor)
                }
            }
        }

        Function::Annualize => {
            require_args("annualize", args, 2)?;

            // Annualize a FLOW value (cash flows, income, expenses)
            // by multiplying by periods per year.
            //
            // For periodic RATES, use annualize_rate() instead.
            let value = evaluate_expr(&args[0], context)?;
            let periods_per_year = evaluate_expr(&args[1], context)?;

            if value.is_nan() || periods_per_year.is_nan() {
                Ok(f64::NAN)
            } else {
                Ok(value * periods_per_year)
            }
        }

        Function::AnnualizeRate => {
            require_args("annualize_rate", args, 3)?;

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
            let rate = evaluate_expr(&args[0], context)?;
            let periods_per_year = evaluate_expr(&args[1], context)?;
            let compounding = evaluate_expr(&args[2], context)?;

            if rate.is_nan() || periods_per_year.is_nan() || compounding.is_nan() {
                return Ok(f64::NAN);
            }

            if periods_per_year <= 0.0 {
                return Err(Error::eval(
                    "annualize_rate() periods_per_year must be positive",
                ));
            }

            // Determine methodology based on compounding parameter
            if compounding == 0.0 {
                // Simple annualization
                Ok(rate * periods_per_year)
            } else {
                // Compound annualization: (1 + rate)^periods - 1
                Ok((1.0 + rate).powf(periods_per_year) - 1.0)
            }
        }

        Function::Coalesce => {
            require_min_args("coalesce", args, 2)?;

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
            // EWM variance and std support 2 or 3 arguments:
            // - 2 args: ewm_var(series, alpha) — non-bias-corrected (pandas adjust=False)
            // - 3 args: ewm_var(series, alpha, adjust) — bias correction enabled if adjust=1.0
            if args.len() < 2 || args.len() > 3 {
                return Err(Error::eval(format!(
                    "{}() requires 2 or 3 arguments (series, alpha, [adjust])",
                    format!("{:?}", func).to_lowercase()
                )));
            }

            // Get smoothing factor (alpha)
            let alpha = evaluate_expr(&args[1], context)?;
            if !(0.0..=1.0).contains(&alpha) {
                return Err(Error::eval("ewm alpha must be between 0 and 1"));
            }

            // Get optional bias correction flag (default: false for backward compatibility)
            let adjust = if args.len() == 3 {
                evaluate_expr(&args[2], context)? != 0.0
            } else {
                false
            };

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

            for (i, (_, value)) in values.iter().enumerate().skip(1) {
                let diff = value - ewm_mean;
                ewm_mean = alpha * value + (1.0 - alpha) * ewm_mean;
                ewm_var = (1.0 - alpha) * (ewm_var + alpha * diff * diff);

                // Apply bias correction if requested (pandas adjust=True)
                if adjust {
                    // Bias correction factor: 1 / (1 - (1-alpha)^(i+1))
                    let bias_factor = 1.0 / (1.0 - (1.0 - alpha).powi((i + 1) as i32));
                    ewm_var *= bias_factor;
                }
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
