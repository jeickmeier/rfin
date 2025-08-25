use finstack_core::expr::{CompiledExpr, Expr, Function, SimpleContext};

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
    let e_lag = CompiledExpr::new(Expr::Call(
        Function::Lag,
        vec![Expr::Column("x".into()), Expr::Literal(1.0)],
    ));
    let e_lead = CompiledExpr::new(Expr::Call(
        Function::Lead,
        vec![Expr::Column("x".into()), Expr::Literal(2.0)],
    ));
    let lag = e_lag.eval_scalar(&ctx, &cols);
    let lead = e_lead.eval_scalar(&ctx, &cols);
    assert!(lag[0].is_nan() && (lag[2] - 2.0).abs() < 1e-12);
    assert!(lead[3].is_nan() && (lead[1] - 6.0).abs() < 1e-12);
}

#[test]
fn scalar_diff_pct_cum_rolling_ewm() {
    let ctx = ctx_single();
    let x = col_x();
    let cols: [&[f64]; 1] = [&x];

    let diff = CompiledExpr::new(Expr::Call(
        Function::Diff,
        vec![Expr::Column("x".into()), Expr::Literal(1.0)],
    ))
    .eval_scalar(&ctx, &cols);
    assert!(diff[0].is_nan());
    assert!((diff[1] - 1.0).abs() < 1e-12);

    let pct = CompiledExpr::new(Expr::Call(
        Function::PctChange,
        vec![Expr::Column("x".into()), Expr::Literal(1.0)],
    ))
    .eval_scalar(&ctx, &cols);
    assert!(pct[0].is_nan());
    assert!((pct[2] - (3.0 / 2.0 - 1.0)).abs() < 1e-12);

    let csum = CompiledExpr::new(Expr::Call(Function::CumSum, vec![Expr::Column("x".into())]))
        .eval_scalar(&ctx, &cols);
    assert!((csum[4] - (1.0 + 2.0 + 3.0 + 6.0 + 10.0)).abs() < 1e-12);

    let cprod = CompiledExpr::new(Expr::Call(Function::CumProd, vec![Expr::Column("x".into())]))
        .eval_scalar(&ctx, &cols);
    assert!((cprod[2] - (1.0 * 2.0 * 3.0)).abs() < 1e-12);

    let roll = CompiledExpr::new(Expr::Call(
        Function::RollingMean,
        vec![Expr::Column("x".into()), Expr::Literal(3.0)],
    ))
    .eval_scalar(&ctx, &cols);
    assert!(roll[1].is_nan());
    assert!((roll[2] - (1.0 + 2.0 + 3.0) / 3.0).abs() < 1e-12);

    let rsum = CompiledExpr::new(Expr::Call(
        Function::RollingSum,
        vec![Expr::Column("x".into()), Expr::Literal(3.0)],
    ))
    .eval_scalar(&ctx, &cols);
    assert!(rsum[1].is_nan());
    assert!((rsum[2] - (1.0 + 2.0 + 3.0)).abs() < 1e-12);

    let ewm = CompiledExpr::new(Expr::Call(
        Function::EwmMean,
        vec![
            Expr::Column("x".into()),
            Expr::Literal(0.5),
            Expr::Literal(1.0),
        ],
    ))
    .eval_scalar(&ctx, &cols);
    assert!(ewm[0] - x[0] < 1e-12);
}
