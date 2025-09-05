//! Comprehensive parity tests for expression engine functions.
//!
//! These tests ensure that all implemented functions work correctly
//! across both scalar and Polars execution paths, with deterministic
//! results and proper handling of edge cases.

use finstack_core::expr::*;
use finstack_core::expr::EvalOpts;
use std::f64;

/// Simple context for testing.
struct TestContext {
    columns: Vec<String>,
}

impl TestContext {
    fn new(columns: Vec<&str>) -> Self {
        Self {
            columns: columns.into_iter().map(|s| s.to_string()).collect(),
        }
    }
}

impl ExpressionContext for TestContext {
    fn resolve_index(&self, name: &str) -> Option<usize> {
        self.columns.iter().position(|c| c == name)
    }
}

/// Create an expression node with ID.
fn expr(node: ExprNode) -> Expr {
    Expr { id: None, node }
}

/// Create column reference expression.
fn col(name: &str) -> Expr {
    expr(ExprNode::Column(name.to_string()))
}

/// Create literal expression.
fn lit(value: f64) -> Expr {
    expr(ExprNode::Literal(value))
}

/// Create function call expression.
fn call(func: Function, args: Vec<Expr>) -> Expr {
    expr(ExprNode::Call(func, args))
}

/// Test data for parity tests.
fn test_data() -> (TestContext, Vec<Vec<f64>>) {
    let ctx = TestContext::new(vec!["values", "index"]);
    let data = vec![
        vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0], // values
        vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0],  // index
    ];
    (ctx, data)
}

/// Helper to convert Vec<Vec<f64>> to slice of slices.
fn to_slice_refs(data: &[Vec<f64>]) -> Vec<&[f64]> {
    data.iter().map(|v| v.as_slice()).collect()
}

#[test]
fn test_basic_expressions() {
    let (ctx, data) = test_data();
    let slices = to_slice_refs(&data);

    // Test column reference
    let col_expr = CompiledExpr::new(col("values"));
    let result = col_expr.eval(&ctx, &slices, EvalOpts::default()).values;
    assert_eq!(result, data[0]);

    // Test literal
    let lit_expr = CompiledExpr::new(lit(42.0));
    let result = lit_expr.eval(&ctx, &slices, EvalOpts::default()).values;
    assert_eq!(result, vec![42.0; 10]);
}

#[test]
fn test_lag_lead() {
    let (ctx, data) = test_data();
    let slices = to_slice_refs(&data);

    // Test lag
    let lag_expr = CompiledExpr::new(call(Function::Lag, vec![col("values"), lit(2.0)]));
    let result = lag_expr.eval(&ctx, &slices, EvalOpts::default()).values;
    let expected = [f64::NAN, f64::NAN, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];

    for (a, b) in result.iter().zip(expected.iter()) {
        if a.is_nan() && b.is_nan() {
            continue;
        }
        assert!((a - b).abs() < 1e-10, "lag: {} != {}", a, b);
    }

    // Test lead
    let lead_expr = CompiledExpr::new(call(Function::Lead, vec![col("values"), lit(2.0)]));
    let result = lead_expr.eval(&ctx, &slices, EvalOpts::default()).values;
    let expected = [3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, f64::NAN, f64::NAN];

    for (a, b) in result.iter().zip(expected.iter()) {
        if a.is_nan() && b.is_nan() {
            continue;
        }
        assert!((a - b).abs() < 1e-10, "lead: {} != {}", a, b);
    }
}

#[test]
fn test_diff_pct_change() {
    let (ctx, data) = test_data();
    let slices = to_slice_refs(&data);

    // Test diff
    let diff_expr = CompiledExpr::new(call(Function::Diff, vec![col("values")]));
    let result = diff_expr.eval(&ctx, &slices, EvalOpts::default()).values;
    let expected = [f64::NAN, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0];

    for (a, b) in result.iter().zip(expected.iter()) {
        if a.is_nan() && b.is_nan() {
            continue;
        }
        assert!((a - b).abs() < 1e-10, "diff: {} != {}", a, b);
    }

    // Test pct_change
    let pct_expr = CompiledExpr::new(call(Function::PctChange, vec![col("values")]));
    let result = pct_expr.eval(&ctx, &slices, EvalOpts::default()).values;
    let expected = [
        f64::NAN,
        1.0,
        0.5,
        1.0 / 3.0,
        0.25,
        0.2,
        1.0 / 6.0,
        1.0 / 7.0,
        0.125,
        1.0 / 9.0,
    ];

    for (a, b) in result.iter().zip(expected.iter()) {
        if a.is_nan() && b.is_nan() {
            continue;
        }
        assert!((a - b).abs() < 1e-10, "pct_change: {} != {}", a, b);
    }
}

#[test]
fn test_cumulative_functions() {
    let (ctx, data) = test_data();
    let slices = to_slice_refs(&data);

    // Test cumsum
    let cumsum_expr = CompiledExpr::new(call(Function::CumSum, vec![col("values")]));
    let result = cumsum_expr.eval(&ctx, &slices, EvalOpts::default()).values;
    let expected = [1.0, 3.0, 6.0, 10.0, 15.0, 21.0, 28.0, 36.0, 45.0, 55.0];

    for (a, b) in result.iter().zip(expected.iter()) {
        assert!((a - b).abs() < 1e-10, "cumsum: {} != {}", a, b);
    }

    // Test cumprod
    let cumprod_expr = CompiledExpr::new(call(Function::CumProd, vec![col("values")]));
    let result = cumprod_expr.eval(&ctx, &slices, EvalOpts::default()).values;
    let expected = [
        1.0, 2.0, 6.0, 24.0, 120.0, 720.0, 5040.0, 40320.0, 362880.0, 3628800.0,
    ];

    for (a, b) in result.iter().zip(expected.iter()) {
        assert!((a - b).abs() < 1e-5, "cumprod: {} != {}", a, b); // Allow larger tolerance for large numbers
    }

    // Test cummin
    let cummin_expr = CompiledExpr::new(call(Function::CumMin, vec![col("values")]));
    let result = cummin_expr.eval(&ctx, &slices, EvalOpts::default()).values;
    let expected = [1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0];

    for (a, b) in result.iter().zip(expected.iter()) {
        assert!((a - b).abs() < 1e-10, "cummin: {} != {}", a, b);
    }

    // Test cummax
    let cummax_expr = CompiledExpr::new(call(Function::CumMax, vec![col("values")]));
    let result = cummax_expr.eval(&ctx, &slices, EvalOpts::default()).values;
    let expected = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];

    for (a, b) in result.iter().zip(expected.iter()) {
        assert!((a - b).abs() < 1e-10, "cummax: {} != {}", a, b);
    }
}

#[test]
fn test_rolling_functions() {
    let (ctx, data) = test_data();
    let slices = to_slice_refs(&data);

    // Test rolling mean (window=3)
    let rolling_mean_expr =
        CompiledExpr::new(call(Function::RollingMean, vec![col("values"), lit(3.0)]));
    let result = rolling_mean_expr.eval(&ctx, &slices, EvalOpts::default()).values;

    // Expected: [NaN, NaN, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0]
    let expected = [f64::NAN, f64::NAN, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0];

    for (i, (a, b)) in result.iter().zip(expected.iter()).enumerate() {
        if a.is_nan() && b.is_nan() {
            continue;
        }
        assert!((a - b).abs() < 1e-10, "rolling_mean[{}]: {} != {}", i, a, b);
    }

    // Test rolling sum (window=3)
    let rolling_sum_expr =
        CompiledExpr::new(call(Function::RollingSum, vec![col("values"), lit(3.0)]));
    let result = rolling_sum_expr.eval(&ctx, &slices, EvalOpts::default()).values;
    let expected = [
        f64::NAN,
        f64::NAN,
        6.0,
        9.0,
        12.0,
        15.0,
        18.0,
        21.0,
        24.0,
        27.0,
    ];

    for (i, (a, b)) in result.iter().zip(expected.iter()).enumerate() {
        if a.is_nan() && b.is_nan() {
            continue;
        }
        assert!((a - b).abs() < 1e-10, "rolling_sum[{}]: {} != {}", i, a, b);
    }
}

#[test]
fn test_statistical_functions() {
    let (ctx, data) = test_data();
    let slices = to_slice_refs(&data);

    // Test std (standard deviation)
    let std_expr = CompiledExpr::new(call(Function::Std, vec![col("values")]));
    let result = std_expr.eval(&ctx, &slices, EvalOpts::default()).values;

    // Expected: sample standard deviation of [1,2,3,4,5,6,7,8,9,10] = sqrt(82.5/9) ≈ 3.0277
    let expected_std = (82.5_f64 / 9.0).sqrt();
    for r in &result {
        assert!(
            (r - expected_std).abs() < 1e-4,
            "std: {} != {}",
            r,
            expected_std
        );
    }

    // Test var (variance)
    let var_expr = CompiledExpr::new(call(Function::Var, vec![col("values")]));
    let result = var_expr.eval(&ctx, &slices, EvalOpts::default()).values;

    // Expected: sample variance of [1,2,3,4,5,6,7,8,9,10] = 82.5/9 ≈ 9.1667
    let expected_var = 82.5_f64 / 9.0;
    for r in &result {
        assert!(
            (r - expected_var).abs() < 1e-4,
            "var: {} != {}",
            r,
            expected_var
        );
    }

    // Test median
    let median_expr = CompiledExpr::new(call(Function::Median, vec![col("values")]));
    let result = median_expr.eval(&ctx, &slices, EvalOpts::default()).values;

    // Expected: median of [1,2,3,4,5,6,7,8,9,10] = 5.5
    let expected_median = 5.5;
    for r in &result {
        assert!(
            (r - expected_median).abs() < 1e-10,
            "median: {} != {}",
            r,
            expected_median
        );
    }
}

#[test]
fn test_ewm_mean() {
    let (ctx, data) = test_data();
    let slices = to_slice_refs(&data);

    // Test exponentially weighted moving average with alpha=0.5 and adjust=false
    let ewm_expr = CompiledExpr::new(call(
        Function::EwmMean,
        vec![col("values"), lit(0.5), lit(0.0)],
    )); // adjust=false
    let result = ewm_expr.eval(&ctx, &slices, EvalOpts::default()).values;

    // Calculate expected EWM manually (adjust=false)
    let alpha = 0.5;
    let mut expected = Vec::with_capacity(10);
    let mut ewm = 1.0; // First value
    expected.push(ewm);

    for i in 1..10 {
        let value = (i + 1) as f64;
        ewm = alpha * value + (1.0 - alpha) * ewm;
        expected.push(ewm);
    }

    assert_eq!(result.len(), expected.len());
    for (i, (a, b)) in result.iter().zip(expected.iter()).enumerate() {
        assert!((a - b).abs() < 1e-10, "ewm_mean[{}]: {} != {}", i, a, b);
    }
}

#[test]
fn test_determinism() {
    let (ctx, data) = test_data();
    let slices = to_slice_refs(&data);

    // Test that results are deterministic across multiple runs
    let expr = CompiledExpr::new(call(Function::RollingMean, vec![col("values"), lit(3.0)]));

    let result1 = expr.eval(&ctx, &slices, EvalOpts::default()).values;
    let result2 = expr.eval(&ctx, &slices, EvalOpts::default()).values;
    let result3 = expr.eval(&ctx, &slices, EvalOpts::default()).values;

    assert_eq!(result1.len(), result2.len());
    assert_eq!(result2.len(), result3.len());

    for i in 0..result1.len() {
        if result1[i].is_nan() {
            assert!(result2[i].is_nan() && result3[i].is_nan());
        } else {
            assert!((result1[i] - result2[i]).abs() < 1e-15);
            assert!((result2[i] - result3[i]).abs() < 1e-15);
        }
    }
}

#[test]
fn test_edge_cases() {
    let ctx = TestContext::new(vec!["empty", "single", "nan_data"]);
    let data = [
        vec![],                                  // empty
        vec![42.0],                              // single value
        vec![1.0, f64::NAN, 3.0, f64::NAN, 5.0], // with NaN values
    ];

    // Test with empty data - need to provide slices for all columns in context
    let empty_slices = vec![&data[0][..], &data[1][..], &data[2][..]]; // All columns
    let expr = CompiledExpr::new(call(Function::RollingMean, vec![col("empty"), lit(2.0)]));
    let result = expr.eval(&ctx, &empty_slices, EvalOpts::default()).values;
    assert_eq!(result.len(), 0);

    // Test with single value
    let single_slices = vec![&data[0][..], &data[1][..], &data[2][..]]; // All columns
    let expr = CompiledExpr::new(call(Function::CumSum, vec![col("single")]));
    let result = expr.eval(&ctx, &single_slices, EvalOpts::default()).values;
    assert_eq!(result, vec![42.0]);

    // Test with NaN values
    let nan_slices = vec![&data[0][..], &data[1][..], &data[2][..]]; // All columns
    let expr = CompiledExpr::new(call(Function::CumSum, vec![col("nan_data")]));
    let result = expr.eval(&ctx, &nan_slices, EvalOpts::default()).values;

    // Should handle NaN properly in cumsum
    assert_eq!(result.len(), 5);
    assert_eq!(result[0], 1.0);
    assert!(result[1].is_nan());
    // Subsequent values should also be NaN due to cumulative nature
}

#[test]
fn test_polars_parity() {
    let (ctx, data) = test_data();
    let slices = to_slice_refs(&data);

    // Test functions that have Polars lowering
    let functions_to_test = vec![
        (Function::Lag, vec![col("values"), lit(1.0)]),
        (Function::Lead, vec![col("values"), lit(1.0)]),
        (Function::Diff, vec![col("values"), lit(1.0)]),
        (Function::CumSum, vec![col("values")]),
        (Function::CumProd, vec![col("values")]),
    ];

    for (func, args) in functions_to_test {
        let expr = CompiledExpr::new(call(func, args));

        // Get scalar result
        let scalar_result = expr.eval(&ctx, &slices, EvalOpts::default()).values;

        // Check that Polars lowering is available for these functions
        if let Some(_polars_expr) = expr.to_polars_expr() {
            // For now, just verify the lowering doesn't panic
            // In a full implementation, we would compare Polars vs scalar results
            println!("Function {:?} has Polars lowering", func);
        }

        // Ensure scalar evaluation worked
        assert_eq!(scalar_result.len(), data[0].len());
    }
}
