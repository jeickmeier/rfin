//! Scalar evaluator and Polars lowering.

use super::{ast::Expr, ast::Function, context::ExpressionContext};
use std::vec::Vec;

/// A compiled expression can evaluate scalars and optionally lower to Polars.
/// Compiled expression wrapper.
#[derive(Clone, Debug)]
pub struct CompiledExpr {
    /// Underlying expression AST.
    pub ast: Expr,
}

impl CompiledExpr {
    /// Construct a new compiled expression from an AST.
    pub fn new(ast: Expr) -> Self {
        Self { ast }
    }

    /// Evaluate over columns of equal length (row-wise), returning a new vector.
    /// Inputs are columns as slices; context resolves column names to indices.
    pub fn eval_scalar<C: ExpressionContext>(&self, ctx: &C, cols: &[&[f64]]) -> Vec<f64> {
        let len = cols.first().map(|c| c.len()).unwrap_or(0);
        let mut out = Vec::with_capacity(len);
        match &self.ast {
            Expr::Column(name) => {
                let idx = ctx.resolve_index(name).expect("unknown column");
                out.extend_from_slice(cols[idx]);
            }
            Expr::Literal(v) => {
                out.resize(len, *v);
            }
            Expr::Call(fun, args) => match fun {
                Function::Lag => {
                    let n = arg_as_usize(&args[1]);
                    let base = Self::eval_child(ctx, &args[0], cols);
                    out.extend((0..len).map(|i| if i < n { f64::NAN } else { base[i - n] }));
                }
                Function::Lead => {
                    let n = arg_as_usize(&args[1]);
                    let base = Self::eval_child(ctx, &args[0], cols);
                    out.extend((0..len).map(|i| if i + n >= len { f64::NAN } else { base[i + n] }));
                }
                Function::Diff => {
                    let n = args.get(1).map(arg_as_usize).unwrap_or(1);
                    let base = Self::eval_child(ctx, &args[0], cols);
                    out.extend((0..len).map(|i| {
                        if i < n {
                            f64::NAN
                        } else {
                            base[i] - base[i - n]
                        }
                    }));
                }
                Function::PctChange => {
                    let n = args.get(1).map(arg_as_usize).unwrap_or(1);
                    let base = Self::eval_child(ctx, &args[0], cols);
                    out.extend((0..len).map(|i| {
                        if i < n || base[i - n] == 0.0 {
                            f64::NAN
                        } else {
                            (base[i] / base[i - n]) - 1.0
                        }
                    }));
                }
                Function::CumSum => {
                    let base = Self::eval_child(ctx, &args[0], cols);
                    let mut acc = 0.0;
                    for v in base {
                        acc += v;
                        out.push(acc);
                    }
                }
                Function::CumProd => {
                    let base = Self::eval_child(ctx, &args[0], cols);
                    let mut acc = 1.0;
                    for v in base {
                        acc *= v;
                        out.push(acc);
                    }
                }
                Function::CumMin => {
                    let base = Self::eval_child(ctx, &args[0], cols);
                    let mut cur = f64::INFINITY;
                    for v in base {
                        cur = cur.min(v);
                        out.push(cur);
                    }
                }
                Function::CumMax => {
                    let base = Self::eval_child(ctx, &args[0], cols);
                    let mut cur = f64::NEG_INFINITY;
                    for v in base {
                        cur = cur.max(v);
                        out.push(cur);
                    }
                }
                Function::RollingMean => {
                    let win = arg_as_usize(&args[1]);
                    let base = Self::eval_child(ctx, &args[0], cols);
                    for i in 0..len {
                        if i + 1 < win {
                            out.push(f64::NAN);
                        } else {
                            let s: f64 = base[i + 1 - win..=i].iter().copied().sum();
                            out.push(s / win as f64);
                        }
                    }
                }
                Function::RollingSum => {
                    let win = arg_as_usize(&args[1]);
                    let base = Self::eval_child(ctx, &args[0], cols);
                    for i in 0..len {
                        if i + 1 < win {
                            out.push(f64::NAN);
                        } else {
                            let s: f64 = base[i + 1 - win..=i].iter().copied().sum();
                            out.push(s);
                        }
                    }
                }
                Function::EwmMean => {
                    let alpha = arg_as_f64(&args[1]);
                    let adjust = args
                        .get(2)
                        .map(|a| matches!(a, Expr::Literal(v) if *v != 0.0))
                        .unwrap_or(true);
                    let base = Self::eval_child(ctx, &args[0], cols);
                    let mut outv = Vec::with_capacity(len);
                    let mut prev = 0.0;
                    let mut wsum = 0.0;
                    for (i, &x) in base.iter().enumerate() {
                        if i == 0 {
                            prev = x;
                            wsum = 1.0;
                            outv.push(x);
                            continue;
                        }
                        if adjust {
                            wsum = 1.0 + (1.0 - alpha) * wsum;
                        }
                        prev = alpha * x + (1.0 - alpha) * prev;
                        outv.push(prev / if adjust { wsum } else { 1.0 });
                    }
                    out = outv;
                }
            },
        }
        out
    }

    fn eval_child<C: ExpressionContext>(ctx: &C, e: &Expr, cols: &[&[f64]]) -> Vec<f64> {
        Self { ast: e.clone() }.eval_scalar(ctx, cols)
    }

    /// Lower to a Polars expression when possible.
    pub fn to_polars_expr(&self) -> Option<polars::lazy::dsl::Expr> {
        use polars::lazy::dsl::{col, lit};
        match &self.ast {
            Expr::Column(name) => Some(col(name)),
            Expr::Literal(v) => Some(lit(*v)),
            Expr::Call(fun, args) => match fun {
                Function::Lag => Some(Self::lower_binary(&args[0], &args[1], |x, n| {
                    x.shift(lit(arg_as_i64(n)))
                })),
                Function::Lead => Some(Self::lower_binary(&args[0], &args[1], |x, n| {
                    x.shift(lit(-(arg_as_i64(n))))
                })),
                Function::Diff => Some(Self::lower_unary_int(&args[0], args.get(1), |x, n| {
                    x.clone() - x.shift(lit(n as i64))
                })),
                Function::PctChange => {
                    Some(Self::lower_unary_int(&args[0], args.get(1), |x, n| {
                        (x.clone() / x.shift(lit(n as i64))) - lit(1.0)
                    }))
                }
                Function::RollingMean => Some({
                    let n = arg_as_usize(&args[1]);
                    let base = Self {
                        ast: args[0].clone(),
                    }
                    .to_polars_expr()
                    .unwrap();
                    let mut acc = base.clone();
                    for k in 1..n {
                        acc = acc + base.clone().shift(lit(k as i64));
                    }
                    acc / lit(n as f64)
                }),
                Function::RollingSum => Some({
                    let n = arg_as_usize(&args[1]);
                    let base = Self { ast: args[0].clone() }.to_polars_expr().unwrap();
                    let mut acc = base.clone();
                    for k in 1..n {
                        acc = acc + base.clone().shift(lit(k as i64));
                    }
                    acc
                }),
                // cum/ewm: scalar fallback for now
                _ => None,
            },
        }
    }

    fn lower_unary_int<F>(e: &Expr, n: Option<&Expr>, f: F) -> polars::prelude::Expr
    where
        F: FnOnce(polars::prelude::Expr, usize) -> polars::prelude::Expr,
    {
        let x = Self { ast: e.clone() }.to_polars_expr().unwrap();
        let n = n.map(arg_as_usize).unwrap_or(1);
        f(x, n)
    }

    fn lower_binary<F>(lhs: &Expr, rhs: &Expr, f: F) -> polars::prelude::Expr
    where
        F: FnOnce(polars::prelude::Expr, &Expr) -> polars::prelude::Expr,
    {
        let x = Self { ast: lhs.clone() }.to_polars_expr().unwrap();
        f(x, rhs)
    }
}

fn arg_as_usize(e: &Expr) -> usize {
    match e {
        Expr::Literal(v) => (*v as i64).max(0) as usize,
        _ => 0,
    }
}
fn arg_as_i64(e: &Expr) -> i64 {
    match e {
        Expr::Literal(v) => (*v as i64).abs(),
        _ => 0,
    }
}
fn arg_as_f64(e: &Expr) -> f64 {
    match e {
        Expr::Literal(v) => *v,
        _ => 0.0,
    }
}
