use finstack_core::expr::{CompiledExpr, EvalOpts, Expr, Function, SimpleContext};

fn ctx_single() -> SimpleContext {
    SimpleContext::new(["x"])
}

fn col_x() -> Vec<f64> {
    vec![1.0, 2.0, 3.0, 6.0, 10.0]
}

#[test]
fn scalar_lag_lead() {
    let ctx = ctx_single();
    let x = col_x();
    let cols: [&[f64]; 1] = [&x];
    let e_lag = CompiledExpr::new(Expr::call(
        Function::Lag,
        vec![Expr::column("x"), Expr::literal(1.0)],
    ));
    let e_lead = CompiledExpr::new(Expr::call(
        Function::Lead,
        vec![Expr::column("x"), Expr::literal(2.0)],
    ));
    let lag = e_lag.eval(&ctx, &cols, EvalOpts::default()).values;
    let lead = e_lead.eval(&ctx, &cols, EvalOpts::default()).values;
    assert!(lag[0].is_nan() && (lag[2] - 2.0).abs() < 1e-12);
    assert!(lead[3].is_nan() && (lead[1] - 6.0).abs() < 1e-12);
}

#[test]
fn scalar_diff_pct_cum_rolling_ewm() {
    let ctx = ctx_single();
    let x = col_x();
    let cols: [&[f64]; 1] = [&x];

    let diff = CompiledExpr::new(Expr::call(
        Function::Diff,
        vec![Expr::column("x"), Expr::literal(1.0)],
    ))
    .eval(&ctx, &cols, EvalOpts::default())
    .values;
    assert!(diff[0].is_nan());
    assert!((diff[1] - 1.0).abs() < 1e-12);

    let pct = CompiledExpr::new(Expr::call(
        Function::PctChange,
        vec![Expr::column("x"), Expr::literal(1.0)],
    ))
    .eval(&ctx, &cols, EvalOpts::default())
    .values;
    assert!(pct[0].is_nan());
    assert!((pct[2] - (3.0 / 2.0 - 1.0)).abs() < 1e-12);

    let csum = CompiledExpr::new(Expr::call(Function::CumSum, vec![Expr::column("x")]))
        .eval(&ctx, &cols, EvalOpts::default())
        .values;
    assert!((csum[4] - (1.0 + 2.0 + 3.0 + 6.0 + 10.0)).abs() < 1e-12);

    let cprod = CompiledExpr::new(Expr::call(Function::CumProd, vec![Expr::column("x")]))
        .eval(&ctx, &cols, EvalOpts::default())
        .values;
    assert!((cprod[2] - (1.0 * 2.0 * 3.0)).abs() < 1e-12);

    let roll = CompiledExpr::new(Expr::call(
        Function::RollingMean,
        vec![Expr::column("x"), Expr::literal(3.0)],
    ))
    .eval(&ctx, &cols, EvalOpts::default())
    .values;
    assert!(roll[1].is_nan());
    assert!((roll[2] - (1.0 + 2.0 + 3.0) / 3.0).abs() < 1e-12);

    let rsum = CompiledExpr::new(Expr::call(
        Function::RollingSum,
        vec![Expr::column("x"), Expr::literal(3.0)],
    ))
    .eval(&ctx, &cols, EvalOpts::default())
    .values;
    assert!(rsum[1].is_nan());
    assert!((rsum[2] - (1.0 + 2.0 + 3.0)).abs() < 1e-12);

    let ewm = CompiledExpr::new(Expr::call(
        Function::EwmMean,
        vec![Expr::column("x"), Expr::literal(0.5), Expr::literal(1.0)],
    ))
    .eval(&ctx, &cols, EvalOpts::default())
    .values;
    assert!(ewm[0] - x[0] < 1e-12);
}
