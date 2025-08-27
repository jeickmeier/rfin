use finstack_core::expr::{CompiledExpr, Expr, Function};
use polars::prelude::*;

#[test]
fn polars_lowering_lag_lead_parity() {
    let df = df! { "x" => &[1.0, 2.0, 3.0, 6.0, 10.0] }.unwrap();

    let lag_e = CompiledExpr::new(Expr::call(
        Function::Lag,
        vec![Expr::column("x"), Expr::literal(1.0)],
    ));
    let lead_e = CompiledExpr::new(Expr::call(
        Function::Lead,
        vec![Expr::column("x"), Expr::literal(2.0)],
    ));

    let lag_p = lag_e.to_polars_expr().unwrap();
    let lead_p = lead_e.to_polars_expr().unwrap();
    let roll_e = CompiledExpr::new(Expr::call(
        Function::RollingMean,
        vec![Expr::column("x"), Expr::literal(3.0)],
    ));
    let roll_p = roll_e.to_polars_expr().unwrap();

    let out = df
        .clone()
        .lazy()
        .with_columns([
            lag_p.alias("lag"),
            lead_p.alias("lead"),
            roll_p.alias("roll"),
        ])
        .collect()
        .unwrap();
    let x = df
        .column("x")
        .unwrap()
        .f64()
        .unwrap()
        .into_no_null_iter()
        .collect::<Vec<_>>();
    let cols: [&[f64]; 1] = [&x];
    let ctx = finstack_core::expr::SimpleContext::new(["x"]);
    let lag_s = lag_e.eval_scalar(&ctx, &cols);
    let lead_s = lead_e.eval_scalar(&ctx, &cols);

    let lag_pv: Vec<f64> = out
        .column("lag")
        .unwrap()
        .f64()
        .unwrap()
        .into_iter()
        .map(|o| o.unwrap_or(f64::NAN))
        .collect();
    let lead_pv: Vec<f64> = out
        .column("lead")
        .unwrap()
        .f64()
        .unwrap()
        .into_iter()
        .map(|o| o.unwrap_or(f64::NAN))
        .collect();

    for i in 0..x.len() {
        assert!((lag_s[i] - lag_pv[i]).abs() < 1e-12 || (lag_s[i].is_nan() && lag_pv[i].is_nan()));
    }
    for i in 0..x.len() {
        assert!(
            (lead_s[i] - lead_pv[i]).abs() < 1e-12 || (lead_s[i].is_nan() && lead_pv[i].is_nan())
        );
    }
    // Spot-check rolling mean parity at index 2 (first defined)
    let roll_pv: Vec<f64> = out
        .column("roll")
        .unwrap()
        .f64()
        .unwrap()
        .into_iter()
        .map(|o| o.unwrap_or(f64::NAN))
        .collect();
    let cols: [&[f64]; 1] = [&x];
    let roll_s = roll_e.eval_scalar(&ctx, &cols);
    assert!((roll_s[2] - roll_pv[2]).abs() < 1e-12);
}
