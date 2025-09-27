//! Tests for expression evaluator functionality.

mod common;

use common::TestExprCtx;
use finstack_core::expr::{CompiledExpr, EvalOpts, Expr, Function};

fn create_test_data() -> (TestExprCtx, Vec<Vec<f64>>) {
    let ctx = TestExprCtx::new().with_column("x", 0).with_column("y", 1);

    let data = vec![
        vec![1.0, 2.0, 3.0, 4.0, 5.0],      // x column
        vec![10.0, 20.0, 30.0, 40.0, 50.0], // y column
    ];

    (ctx, data)
}

#[test]
fn test_compiled_expr_column_and_literal() {
    let (ctx, data) = create_test_data();
    let cols: Vec<&[f64]> = data.iter().map(|v| v.as_slice()).collect();

    // Test column access
    let col_expr = CompiledExpr::new(Expr::column("x"));
    let result = col_expr.eval(&ctx, &cols, EvalOpts::default()).values;
    assert_eq!(result, vec![1.0, 2.0, 3.0, 4.0, 5.0]);

    // Test literal
    let lit_expr = CompiledExpr::new(Expr::literal(42.0));
    let result = lit_expr.eval(&ctx, &cols, EvalOpts::default()).values;
    assert_eq!(result, vec![42.0, 42.0, 42.0, 42.0, 42.0]);
}

#[test]
fn test_compiled_expr_lag_and_lead() {
    let (ctx, data) = create_test_data();
    let cols: Vec<&[f64]> = data.iter().map(|v| v.as_slice()).collect();

    // Test Lag
    let lag_expr = CompiledExpr::new(Expr::call(
        Function::Lag,
        vec![Expr::column("x"), Expr::literal(1.0)],
    ));
    let result = lag_expr.eval(&ctx, &cols, EvalOpts::default()).values;
    assert!(result[0].is_nan()); // First value should be NaN
    assert_eq!(result[1], 1.0); // Second value should be first input
    assert_eq!(result[2], 2.0); // Third value should be second input

    // Test Lead
    let lead_expr = CompiledExpr::new(Expr::call(
        Function::Lead,
        vec![Expr::column("x"), Expr::literal(1.0)],
    ));
    let result = lead_expr.eval(&ctx, &cols, EvalOpts::default()).values;
    assert_eq!(result[0], 2.0); // First value should be second input
    assert_eq!(result[1], 3.0); // Second value should be third input
    assert!(result[4].is_nan()); // Last value should be NaN
}

#[test]
fn test_compiled_expr_diff_and_pct_change() {
    let (ctx, data) = create_test_data();
    let cols: Vec<&[f64]> = data.iter().map(|v| v.as_slice()).collect();

    // Test Diff
    let diff_expr = CompiledExpr::new(Expr::call(
        Function::Diff,
        vec![Expr::column("x"), Expr::literal(1.0)],
    ));
    let result = diff_expr.eval(&ctx, &cols, EvalOpts::default()).values;
    assert!(result[0].is_nan()); // First value should be NaN
    assert_eq!(result[1], 1.0); // 2 - 1 = 1
    assert_eq!(result[2], 1.0); // 3 - 2 = 1

    // Test PctChange
    let pct_expr = CompiledExpr::new(Expr::call(
        Function::PctChange,
        vec![Expr::column("x"), Expr::literal(1.0)],
    ));
    let result = pct_expr.eval(&ctx, &cols, EvalOpts::default()).values;
    assert!(result[0].is_nan()); // First value should be NaN
    assert_eq!(result[1], 1.0); // (2/1) - 1 = 1
    assert_eq!(result[2], 0.5); // (3/2) - 1 = 0.5
}

#[test]
fn test_compiled_expr_cumulative_functions() {
    let (ctx, data) = create_test_data();
    let cols: Vec<&[f64]> = data.iter().map(|v| v.as_slice()).collect();

    // Test CumSum
    let cumsum_expr = CompiledExpr::new(Expr::call(Function::CumSum, vec![Expr::column("x")]));
    let result = cumsum_expr.eval(&ctx, &cols, EvalOpts::default()).values;
    assert_eq!(result, vec![1.0, 3.0, 6.0, 10.0, 15.0]); // 1, 1+2, 1+2+3, etc.

    // Test CumProd
    let cumprod_expr = CompiledExpr::new(Expr::call(Function::CumProd, vec![Expr::column("x")]));
    let result = cumprod_expr.eval(&ctx, &cols, EvalOpts::default()).values;
    assert_eq!(result, vec![1.0, 2.0, 6.0, 24.0, 120.0]); // 1, 1*2, 1*2*3, etc.

    // Test CumMin
    let cummin_expr = CompiledExpr::new(Expr::call(Function::CumMin, vec![Expr::column("x")]));
    let result = cummin_expr.eval(&ctx, &cols, EvalOpts::default()).values;
    assert_eq!(result, vec![1.0, 1.0, 1.0, 1.0, 1.0]); // Min so far

    // Test CumMax
    let cummax_expr = CompiledExpr::new(Expr::call(Function::CumMax, vec![Expr::column("x")]));
    let result = cummax_expr.eval(&ctx, &cols, EvalOpts::default()).values;
    assert_eq!(result, vec![1.0, 2.0, 3.0, 4.0, 5.0]); // Max so far
}

#[test]
fn test_compiled_expr_rolling_functions() {
    let (ctx, data) = create_test_data();
    let cols: Vec<&[f64]> = data.iter().map(|v| v.as_slice()).collect();

    // Test RollingMean with window 3
    let rolling_mean_expr = CompiledExpr::new(Expr::call(
        Function::RollingMean,
        vec![Expr::column("x"), Expr::literal(3.0)],
    ));
    let result = rolling_mean_expr
        .eval(&ctx, &cols, EvalOpts::default())
        .values;
    assert!(result[0].is_nan()); // Not enough data
    assert!(result[1].is_nan()); // Not enough data
    assert_eq!(result[2], 2.0); // (1+2+3)/3 = 2
    assert_eq!(result[3], 3.0); // (2+3+4)/3 = 3

    // Test RollingSum with window 2
    let rolling_sum_expr = CompiledExpr::new(Expr::call(
        Function::RollingSum,
        vec![Expr::column("x"), Expr::literal(2.0)],
    ));
    let result = rolling_sum_expr
        .eval(&ctx, &cols, EvalOpts::default())
        .values;
    assert!(result[0].is_nan()); // Not enough data
    assert_eq!(result[1], 3.0); // 1+2 = 3
    assert_eq!(result[2], 5.0); // 2+3 = 5
    assert_eq!(result[3], 7.0); // 3+4 = 7
}

#[test]
fn test_compiled_expr_ewm_mean() {
    let (ctx, data) = create_test_data();
    let cols: Vec<&[f64]> = data.iter().map(|v| v.as_slice()).collect();

    // Test EwmMean with alpha=0.5, adjust=true (default)
    let ewm_expr = CompiledExpr::new(Expr::call(
        Function::EwmMean,
        vec![Expr::column("x"), Expr::literal(0.5)],
    ));
    let result = ewm_expr.eval(&ctx, &cols, EvalOpts::default()).values;

    // First value should be the input value
    assert_eq!(result[0], 1.0);

    // All values should be finite and reasonable
    assert_eq!(result.len(), 5);
    for val in result {
        assert!(val.is_finite());
        assert!(val > 0.0); // Should be positive given positive inputs
    }
}

#[test]
fn test_compiled_expr_statistical_functions() {
    let (ctx, data) = create_test_data();
    let cols: Vec<&[f64]> = data.iter().map(|v| v.as_slice()).collect();

    // Test Std
    let std_expr = CompiledExpr::new(Expr::call(Function::Std, vec![Expr::column("x")]));
    let result = std_expr.eval(&ctx, &cols, EvalOpts::default()).values;

    // All values should be the same (standard deviation of the entire series)
    let expected_std = result[0];
    assert!(expected_std > 0.0);
    for val in &result {
        assert!((val - expected_std).abs() < 1e-10);
    }

    // Test Var
    let var_expr = CompiledExpr::new(Expr::call(Function::Var, vec![Expr::column("x")]));
    let result = var_expr.eval(&ctx, &cols, EvalOpts::default()).values;

    // Variance should be std^2
    let expected_var = expected_std * expected_std;
    for val in &result {
        assert!((val - expected_var).abs() < 1e-10);
    }

    // Test Median
    let median_expr = CompiledExpr::new(Expr::call(Function::Median, vec![Expr::column("x")]));
    let result = median_expr.eval(&ctx, &cols, EvalOpts::default()).values;

    // Median of [1,2,3,4,5] should be 3
    for val in &result {
        assert_eq!(*val, 3.0);
    }
}

#[test]
fn test_compiled_expr_with_metadata() {
    let (ctx, data) = create_test_data();
    let cols: Vec<&[f64]> = data.iter().map(|v| v.as_slice()).collect();

    let expr = CompiledExpr::new(Expr::column("x"));
    let result = expr.eval(&ctx, &cols, EvalOpts::default());

    // Check that metadata is present and sensible (minimal shape)
    assert_eq!(result.values, vec![1.0, 2.0, 3.0, 4.0, 5.0]);
    assert_eq!(format!("{:?}", result.metadata.numeric_mode), "F64");
}

#[test]
fn test_compiled_expr_with_planning() {
    let (ctx, data) = create_test_data();
    let cols: Vec<&[f64]> = data.iter().map(|v| v.as_slice()).collect();

    // Create expression with DAG planning
    let expr = Expr::call(
        Function::RollingMean,
        vec![Expr::column("x"), Expr::literal(2.0)],
    );
    let meta =
        finstack_core::config::results_meta(&finstack_core::config::FinstackConfig::default());
    let compiled = CompiledExpr::with_planning(expr, meta);

    let result = compiled.eval(&ctx, &cols, EvalOpts::default()).values;

    // Should produce same result as without planning
    assert!(result[0].is_nan());
    assert_eq!(result[1], 1.5); // (1+2)/2
    assert_eq!(result[2], 2.5); // (2+3)/2
}

#[test]
fn test_compiled_expr_edge_cases() {
    let ctx = TestExprCtx::new().with_column("empty", 0);
    let empty_data = [Vec::<f64>::new()];
    let cols: Vec<&[f64]> = empty_data.iter().map(|v| v.as_slice()).collect();

    // Test with empty data
    let expr = CompiledExpr::new(Expr::column("empty"));
    let result = expr.eval(&ctx, &cols, EvalOpts::default()).values;
    assert!(result.is_empty());

    // Test with literal on empty data
    let lit_expr = CompiledExpr::new(Expr::literal(5.0));
    let result = lit_expr.eval(&ctx, &cols, EvalOpts::default()).values;
    assert!(result.is_empty());
}
