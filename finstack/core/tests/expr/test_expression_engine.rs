use finstack_core::config::{results_meta, FinstackConfig};
use finstack_core::expr::{BinOp, CompiledExpr, EvalOpts, Expr, ExprNode, Function, SimpleContext};

#[test]
fn ast_builders_and_equality_semantics() {
    let a = Expr::column("x").with_id(1);
    let b = Expr::column("x").with_id(999);
    assert_eq!(a, b);

    let add = Expr::bin_op(BinOp::Add, Expr::literal(2.0), Expr::literal(3.0));
    if let ExprNode::BinOp { op, .. } = &add.node {
        assert!(matches!(op, BinOp::Add));
    } else {
        panic!("expected binop");
    }
}

#[test]
fn dag_dedup_and_topological_order() {
    let mut builder = finstack_core::expr::dag::DagBuilder::new();
    let expr1 = Expr::call(
        Function::RollingMean,
        vec![Expr::column("x"), Expr::literal(3.0)],
    );
    let expr2 = Expr::call(
        Function::RollingMean,
        vec![Expr::column("x"), Expr::literal(3.0)],
    );
    let meta = results_meta(&FinstackConfig::default());
    let plan = builder.build_plan(vec![expr1, expr2], meta);
    // Expect nodes: column, literal, rolling mean
    assert!(plan.nodes.len() >= 3);
    assert_eq!(plan.roots.len(), 2);
    assert_eq!(plan.roots[0], plan.roots[1]);
}

#[test]
fn evaluator_scalar_and_with_plan_cache() {
    let ctx = SimpleContext::new(["x", "y"]);
    let x = vec![1.0, 2.0, 3.0, 4.0];
    let y = vec![2.0, 1.0, 0.0, -1.0];
    let cols: Vec<&[f64]> = vec![x.as_slice(), y.as_slice()];

    // Simple scalar evaluation: if x > y then x - y else y - x
    let cond = Expr::bin_op(BinOp::Gt, Expr::column("x"), Expr::column("y"));
    let then_expr = Expr::bin_op(BinOp::Sub, Expr::column("x"), Expr::column("y"));
    let else_expr = Expr::bin_op(BinOp::Sub, Expr::column("y"), Expr::column("x"));
    let expr = Expr::if_then_else(cond, then_expr, else_expr);
    let compiled = CompiledExpr::new(expr);
    let out = compiled.eval(&ctx, &cols, EvalOpts::default()).values;
    assert_eq!(out, vec![1.0, 1.0, 3.0, 5.0]);

    // With planning and cache: rolling sum
    let expr2 = Expr::call(
        Function::RollingSum,
        vec![Expr::column("x"), Expr::literal(2.0)],
    );
    let meta = results_meta(&FinstackConfig::default());
    let compiled2 = CompiledExpr::with_planning(expr2, meta).with_cache(1);
    let out2 = compiled2
        .eval(
            &ctx,
            &cols,
            EvalOpts {
                plan: None,
                cache_budget_mb: Some(1),
            },
        )
        .values;
    assert!(out2[0].is_nan());
    assert!((out2[1] - 3.0).abs() < 1e-12);
    assert!((out2[2] - 5.0).abs() < 1e-12);
    assert!((out2[3] - 7.0).abs() < 1e-12);
}

#[test]
fn lag_lead_and_shift_behaviors() {
    let ctx = SimpleContext::new(["x"]);
    let x = vec![10.0, 20.0, 30.0, 40.0];
    let cols: Vec<&[f64]> = vec![x.as_slice()];

    // lag(x, 1)
    let lag_expr = Expr::call(Function::Lag, vec![Expr::column("x"), Expr::literal(1.0)]);
    let lag_out = CompiledExpr::new(lag_expr)
        .eval(&ctx, &cols, EvalOpts::default())
        .values;
    assert!(lag_out[0].is_nan());
    assert_eq!(lag_out[1], 10.0);
    assert_eq!(lag_out[2], 20.0);
    assert_eq!(lag_out[3], 30.0);

    // lead(x, 1)
    let lead_expr = Expr::call(Function::Lead, vec![Expr::column("x"), Expr::literal(1.0)]);
    let lead_out = CompiledExpr::new(lead_expr)
        .eval(&ctx, &cols, EvalOpts::default())
        .values;
    assert_eq!(lead_out[0], 20.0);
    assert_eq!(lead_out[1], 30.0);
    assert_eq!(lead_out[2], 40.0);
    assert!(lead_out[3].is_nan());

    // shift(x, +1) == lag(x, 1); shift(x, -1) == lead(x, 1)
    let shift_pos = Expr::call(Function::Shift, vec![Expr::column("x"), Expr::literal(1.0)]);
    let shift_neg = Expr::call(
        Function::Shift,
        vec![Expr::column("x"), Expr::literal(-1.0)],
    );
    let shift_pos_out = CompiledExpr::new(shift_pos)
        .eval(&ctx, &cols, EvalOpts::default())
        .values;
    let shift_neg_out = CompiledExpr::new(shift_neg)
        .eval(&ctx, &cols, EvalOpts::default())
        .values;
    for i in 0..4 {
        if i == 0 {
            assert!(shift_pos_out[i].is_nan());
        } else {
            assert_eq!(shift_pos_out[i], lag_out[i]);
        }
        if i == 3 {
            assert!(shift_neg_out[i].is_nan());
        } else {
            assert_eq!(shift_neg_out[i], lead_out[i]);
        }
    }
}

#[test]
fn diff_and_pct_change_with_steps() {
    let ctx = SimpleContext::new(["x"]);
    let x = vec![1.0, 2.0, 4.0, 8.0];
    let cols: Vec<&[f64]> = vec![x.as_slice()];

    let diff2 = Expr::call(Function::Diff, vec![Expr::column("x"), Expr::literal(2.0)]);
    let diff2_out = CompiledExpr::new(diff2)
        .eval(&ctx, &cols, EvalOpts::default())
        .values;
    assert!(diff2_out[0].is_nan() && diff2_out[1].is_nan());
    assert_eq!(diff2_out[2], 3.0);
    assert_eq!(diff2_out[3], 6.0);

    let pct2 = Expr::call(
        Function::PctChange,
        vec![Expr::column("x"), Expr::literal(2.0)],
    );
    let pct2_out = CompiledExpr::new(pct2)
        .eval(&ctx, &cols, EvalOpts::default())
        .values;
    assert!(pct2_out[0].is_nan() && pct2_out[1].is_nan());
    assert!((pct2_out[2] - 3.0).abs() < 1e-12);
    assert!((pct2_out[3] - 3.0).abs() < 1e-12);
}

#[test]
fn cumulative_operations() {
    let ctx = SimpleContext::new(["x"]);
    let x = vec![3.0, 1.0, 4.0, 2.0];
    let cols: Vec<&[f64]> = vec![x.as_slice()];

    // cumsum
    let cumsum = Expr::call(Function::CumSum, vec![Expr::column("x")]);
    let s = CompiledExpr::new(cumsum)
        .eval(&ctx, &cols, EvalOpts::default())
        .values;
    assert_eq!(s, vec![3.0, 4.0, 8.0, 10.0]);

    // cumprod
    let cumprod = Expr::call(Function::CumProd, vec![Expr::column("x")]);
    let p = CompiledExpr::new(cumprod)
        .eval(&ctx, &cols, EvalOpts::default())
        .values;
    assert_eq!(p, vec![3.0, 3.0, 12.0, 24.0]);

    // cummin / cummax
    let cummin = Expr::call(Function::CumMin, vec![Expr::column("x")]);
    let cmin = CompiledExpr::new(cummin)
        .eval(&ctx, &cols, EvalOpts::default())
        .values;
    assert_eq!(cmin, vec![3.0, 1.0, 1.0, 1.0]);
    let cummax = Expr::call(Function::CumMax, vec![Expr::column("x")]);
    let cmax = CompiledExpr::new(cummax)
        .eval(&ctx, &cols, EvalOpts::default())
        .values;
    assert_eq!(cmax, vec![3.0, 3.0, 4.0, 4.0]);
}

#[test]
fn rolling_statistics_mean_std_var_median_min_max_count() {
    let ctx = SimpleContext::new(["x"]);
    let x = vec![1.0, 2.0, 3.0, 4.0];
    let cols: Vec<&[f64]> = vec![x.as_slice()];
    let win = Expr::literal(2.0);

    let rmean = Expr::call(Function::RollingMean, vec![Expr::column("x"), win.clone()]);
    let rsum = Expr::call(Function::RollingSum, vec![Expr::column("x"), win.clone()]);
    let rstd = Expr::call(Function::RollingStd, vec![Expr::column("x"), win.clone()]);
    let rvar = Expr::call(Function::RollingVar, vec![Expr::column("x"), win.clone()]);
    let rmed = Expr::call(
        Function::RollingMedian,
        vec![Expr::column("x"), win.clone()],
    );
    let rmin = Expr::call(Function::RollingMin, vec![Expr::column("x"), win.clone()]);
    let rmax = Expr::call(Function::RollingMax, vec![Expr::column("x"), win.clone()]);
    let rcount = Expr::call(Function::RollingCount, vec![Expr::column("x"), win.clone()]);

    let rm = CompiledExpr::new(rmean)
        .eval(&ctx, &cols, EvalOpts::default())
        .values;
    let rs = CompiledExpr::new(rsum)
        .eval(&ctx, &cols, EvalOpts::default())
        .values;
    let rst = CompiledExpr::new(rstd)
        .eval(&ctx, &cols, EvalOpts::default())
        .values;
    let rv = CompiledExpr::new(rvar)
        .eval(&ctx, &cols, EvalOpts::default())
        .values;
    let rmd = CompiledExpr::new(rmed)
        .eval(&ctx, &cols, EvalOpts::default())
        .values;
    let rmn = CompiledExpr::new(rmin)
        .eval(&ctx, &cols, EvalOpts::default())
        .values;
    let rmx = CompiledExpr::new(rmax)
        .eval(&ctx, &cols, EvalOpts::default())
        .values;
    let rc = CompiledExpr::new(rcount)
        .eval(&ctx, &cols, EvalOpts::default())
        .values;

    assert!(rm[0].is_nan());
    assert!((rm[1] - 1.5).abs() < 1e-12);
    assert!((rm[2] - 2.5).abs() < 1e-12);
    assert!((rm[3] - 3.5).abs() < 1e-12);

    assert!(rs[0].is_nan());
    assert_eq!(rs[1], 3.0);
    assert_eq!(rs[2], 5.0);
    assert_eq!(rs[3], 7.0);

    assert!(rst[0].is_nan());
    assert!((rst[1] - 0.5).abs() < 1e-12);
    assert!((rst[2] - 0.5).abs() < 1e-12);
    assert!((rst[3] - 0.5).abs() < 1e-12);

    assert!(rv[0].is_nan());
    assert!((rv[1] - 0.25).abs() < 1e-12);
    assert!((rv[2] - 0.25).abs() < 1e-12);
    assert!((rv[3] - 0.25).abs() < 1e-12);

    assert!(rmd[0].is_nan());
    assert!((rmd[1] - 1.5).abs() < 1e-12);
    assert!((rmd[2] - 2.5).abs() < 1e-12);
    assert!((rmd[3] - 3.5).abs() < 1e-12);

    assert!(rmn[0].is_nan());
    assert_eq!(rmn[1], 1.0);
    assert_eq!(rmn[2], 2.0);
    assert_eq!(rmn[3], 3.0);

    assert!(rmx[0].is_nan());
    assert_eq!(rmx[1], 2.0);
    assert_eq!(rmx[2], 3.0);
    assert_eq!(rmx[3], 4.0);

    assert!(rc[0].is_nan());
    assert_eq!(rc[1], 2.0);
    assert_eq!(rc[2], 2.0);
    assert_eq!(rc[3], 2.0);
}

#[test]
fn rank_and_quantile() {
    let ctx = SimpleContext::new(["v"]);
    let v = vec![3.0, 1.0, 2.0, 2.0];
    let cols: Vec<&[f64]> = vec![v.as_slice()];

    let rank = Expr::call(Function::Rank, vec![Expr::column("v")]);
    let r = CompiledExpr::new(rank)
        .eval(&ctx, &cols, EvalOpts::default())
        .values;
    assert_eq!(r, vec![3.0, 1.0, 2.0, 2.0]);

    let q50 = Expr::call(
        Function::Quantile,
        vec![Expr::column("v"), Expr::literal(0.5)],
    );
    let q = CompiledExpr::new(q50)
        .eval(&ctx, &cols, EvalOpts::default())
        .values;
    // Median of [1, 2, 2, 3] is average of middle: 2.0
    for val in q {
        assert!((val - 2.0).abs() < 1e-12);
    }
}

#[test]
fn ewm_std_and_var_consistency() {
    let ctx = SimpleContext::new(["x"]);
    let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let cols: Vec<&[f64]> = vec![x.as_slice()];

    let alpha = Expr::literal(0.5);
    let adjust_true = Expr::literal(1.0);
    let std_expr = Expr::call(
        Function::EwmStd,
        vec![Expr::column("x"), alpha.clone(), adjust_true.clone()],
    );
    let var_expr = Expr::call(
        Function::EwmVar,
        vec![Expr::column("x"), alpha.clone(), adjust_true.clone()],
    );

    let std_vals = CompiledExpr::new(std_expr)
        .eval(&ctx, &cols, EvalOpts::default())
        .values;
    let var_vals = CompiledExpr::new(var_expr)
        .eval(&ctx, &cols, EvalOpts::default())
        .values;

    assert_eq!(std_vals.len(), var_vals.len());
    for i in 0..std_vals.len() {
        if std_vals[i].is_nan() || var_vals[i].is_nan() {
            continue;
        }
        // For the same parameters, std^2 ~ var
        let diff = std_vals[i] * std_vals[i] - var_vals[i];
        assert!(
            diff.abs() < 1e-9,
            "index {}: std^2={}, var={}",
            i,
            std_vals[i] * std_vals[i],
            var_vals[i]
        );
        assert!(var_vals[i] >= 0.0);
    }
}
