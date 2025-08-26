//! Scalar evaluator and Polars lowering with DAG planning and caching.

use super::{
    ast::*, 
    cache::{CacheManager, CachedResult},
    dag::{ExecutionPlan, DagBuilder},
    context::ExpressionContext
};
use std::vec::Vec;

/// A compiled expression can evaluate scalars and optionally lower to Polars.
/// Compiled expression wrapper with DAG planning and caching support.
#[derive(Clone, Debug)]
pub struct CompiledExpr {
    /// Underlying expression AST.
    pub ast: Expr,
    /// Optional execution plan for complex expressions.
    pub plan: Option<ExecutionPlan>,
    /// Cache manager for intermediate results.
    pub cache: Option<CacheManager>,
}

impl CompiledExpr {
    /// Construct a new compiled expression from an AST.
    pub fn new(ast: Expr) -> Self {
        Self { 
            ast,
            plan: None,
            cache: None,
        }
    }

    /// Construct with DAG planning enabled.
    pub fn with_planning(ast: Expr, meta: ExecMeta) -> Self {
        let mut builder = DagBuilder::new();
        let plan = builder.build_plan(vec![ast.clone()], meta);
        let cache = CacheManager::for_plan(&plan, 100); // 100MB default
        
        Self {
            ast,
            plan: Some(plan),
            cache: Some(cache),
        }
    }

    /// Enable caching with the given budget.
    pub fn with_cache(mut self, budget_mb: usize) -> Self {
        if let Some(ref plan) = self.plan {
            self.cache = Some(CacheManager::for_plan(plan, budget_mb));
        } else {
            self.cache = Some(CacheManager::new(budget_mb));
        }
        self
    }

    /// Evaluate using DAG plan if available, otherwise fall back to simple evaluation.
    pub fn eval_scalar<C: ExpressionContext>(&self, ctx: &C, cols: &[&[f64]]) -> Vec<f64> {
        if let Some(ref plan) = self.plan {
            self.eval_with_plan(ctx, cols, plan)
        } else {
            self.eval_simple(ctx, cols, &self.ast)
        }
    }

    /// Evaluate using execution plan with caching.
    fn eval_with_plan<C: ExpressionContext>(
        &self, 
        ctx: &C, 
        cols: &[&[f64]], 
        plan: &ExecutionPlan
    ) -> Vec<f64> {
        let mut results: std::collections::HashMap<u64, Vec<f64>> = std::collections::HashMap::new();
        
        // Execute nodes in topological order
        for node in &plan.nodes {
            // Check cache first
            if let Some(ref cache) = self.cache {
                if let Some(cached) = cache.get(node.id) {
                    if let Ok(scalar_result) = cached.as_scalar() {
                        results.insert(node.id, scalar_result);
                        continue;
                    }
                }
            }

            // Evaluate node
            let result = self.eval_node(ctx, cols, node, &results);
            
            // Cache result if strategy recommends it
            if let Some(ref cache) = self.cache {
                if plan.cache_strategy.cache_nodes.contains(&node.id) {
                    cache.put(node.id, CachedResult::Scalar(result.clone()));
                }
            }
            
            results.insert(node.id, result);
        }

        // Return result of the root node (should be the last one)
        if let Some(&root_id) = plan.roots.first() {
            results.remove(&root_id).unwrap_or_default()
        } else {
            Vec::new()
        }
    }

    /// Evaluate with metadata stamping for determinism tracking.
    pub fn eval_with_metadata<C: ExpressionContext>(&self, ctx: &C, cols: &[&[f64]], exec_meta: ExecMeta) -> EvaluationResult {
        let start_time = std::time::Instant::now();
        
        // Use the appropriate evaluation path based on determinism setting
        let values = if exec_meta.deterministic && self.plan.is_some() {
            // Force sequential execution for determinism
            let mut sequential_self = self.clone();
            sequential_self.cache = None; // Disable cache for full determinism
            sequential_self.eval_scalar(ctx, cols)
        } else {
            self.eval_scalar(ctx, cols)
        };
        
        let execution_time_ns = start_time.elapsed().as_nanos() as u64;
        
        // Calculate cache hit ratio if cache is available
        let cache_hit_ratio = self.cache.as_ref().map(|cache| cache.hit_ratio());
        
        let metadata = ResultMetadata {
            deterministic: exec_meta.deterministic,
            parallel_execution: exec_meta.parallel && self.plan.is_some(),
            numeric_mode: exec_meta.numeric_mode,
            rounding_context: exec_meta.rounding_mode,
            fx_policy_applied: exec_meta.fx_policy,
            execution_time_ns,
            cache_hit_ratio,
        };
        
        EvaluationResult { values, metadata }
    }

    /// Evaluate a single DAG node.
    fn eval_node<C: ExpressionContext>(
        &self,
        ctx: &C,
        cols: &[&[f64]],
        node: &super::dag::DagNode,
        results: &std::collections::HashMap<u64, Vec<f64>>
    ) -> Vec<f64> {
        match &node.expr.node {
            ExprNode::Column(name) => {
                let idx = ctx.resolve_index(name).expect("unknown column");
                cols[idx].to_vec()
            }
            ExprNode::Literal(val) => {
                let len = cols.first().map(|c| c.len()).unwrap_or(0);
                vec![*val; len]
            }
            ExprNode::Call(func, _args) => {
                // Get argument results from dependencies
                let arg_results: Vec<Vec<f64>> = node.dependencies.iter()
                    .map(|&dep_id| results.get(&dep_id).cloned().unwrap_or_else(Vec::new))
                    .collect();
                
                self.eval_function(*func, &arg_results)
            }
        }
    }

    /// Simple evaluation without DAG planning (legacy path).
    fn eval_simple<C: ExpressionContext>(&self, ctx: &C, cols: &[&[f64]], expr: &Expr) -> Vec<f64> {
        let len = cols.first().map(|c| c.len()).unwrap_or(0);
        let mut out = Vec::with_capacity(len);
        match &expr.node {
            ExprNode::Column(name) => {
                let idx = ctx.resolve_index(name).expect("unknown column");
                out.extend_from_slice(cols[idx]);
            }
            ExprNode::Literal(v) => {
                out.resize(len, *v);
            }
            ExprNode::Call(fun, args) => {
                // Recursively evaluate arguments
                let arg_results: Vec<Vec<f64>> = args.iter()
                    .map(|arg| self.eval_simple(ctx, cols, arg))
                    .collect();
                return self.eval_function(*fun, &arg_results);
            }
        }
        out
    }

    /// Evaluate a function with given argument results.
    fn eval_function(&self, fun: Function, arg_results: &[Vec<f64>]) -> Vec<f64> {
        let len = arg_results.first().map(|a| a.len()).unwrap_or(0);
        let mut out = Vec::with_capacity(len);
        
        match fun {
            Function::Lag => {
                if arg_results.len() >= 2 && !arg_results[1].is_empty() {
                    let base = &arg_results[0];
                    let n = arg_results[1][0] as usize; // Assume literal
                    out.extend((0..len).map(|i| if i < n { f64::NAN } else { base[i - n] }));
                }
            }
            Function::Lead => {
                if arg_results.len() >= 2 && !arg_results[1].is_empty() {
                    let base = &arg_results[0];
                    let n = arg_results[1][0] as usize; // Assume literal
                    out.extend((0..len).map(|i| if i + n >= len { f64::NAN } else { base[i + n] }));
                }
            }
            Function::Diff => {
                if !arg_results.is_empty() {
                    let base = &arg_results[0];
                    let n = if arg_results.len() >= 2 && !arg_results[1].is_empty() { 
                        arg_results[1][0] as usize 
                    } else { 
                        1 
                    };
                    out.extend((0..len).map(|i| {
                        if i < n {
                            f64::NAN
                        } else {
                            base[i] - base[i - n]
                        }
                    }));
                }
            }
            Function::PctChange => {
                if !arg_results.is_empty() {
                    let base = &arg_results[0];
                    let n = if arg_results.len() >= 2 && !arg_results[1].is_empty() { 
                        arg_results[1][0] as usize 
                    } else { 
                        1 
                    };
                    out.extend((0..len).map(|i| {
                        if i < n || base[i - n] == 0.0 {
                            f64::NAN
                        } else {
                            (base[i] / base[i - n]) - 1.0
                        }
                    }));
                }
            }
            Function::CumSum => {
                if !arg_results.is_empty() {
                    let base = &arg_results[0];
                    let mut acc = 0.0;
                    for &v in base {
                        acc += v;
                        out.push(acc);
                    }
                }
            }
            Function::CumProd => {
                if !arg_results.is_empty() {
                    let base = &arg_results[0];
                    let mut acc = 1.0;
                    for &v in base {
                        acc *= v;
                        out.push(acc);
                    }
                }
            }
            Function::CumMin => {
                if !arg_results.is_empty() {
                    let base = &arg_results[0];
                    let mut cur = f64::INFINITY;
                    for &v in base {
                        cur = cur.min(v);
                        out.push(cur);
                    }
                }
            }
            Function::CumMax => {
                if !arg_results.is_empty() {
                    let base = &arg_results[0];
                    let mut cur = f64::NEG_INFINITY;
                    for &v in base {
                        cur = cur.max(v);
                        out.push(cur);
                    }
                }
            }
            Function::RollingMean => {
                if arg_results.len() >= 2 && !arg_results[1].is_empty() {
                    let base = &arg_results[0];
                    let win = arg_results[1][0] as usize;
                    for i in 0..len {
                        if i + 1 < win {
                            out.push(f64::NAN);
                        } else {
                            let s: f64 = base[i + 1 - win..=i].iter().copied().sum();
                            out.push(s / win as f64);
                        }
                    }
                }
            }
            Function::RollingSum => {
                if arg_results.len() >= 2 && !arg_results[1].is_empty() {
                    let base = &arg_results[0];
                    let win = arg_results[1][0] as usize;
                    for i in 0..len {
                        if i + 1 < win {
                            out.push(f64::NAN);
                        } else {
                            let s: f64 = base[i + 1 - win..=i].iter().copied().sum();
                            out.push(s);
                        }
                    }
                }
            }
            Function::EwmMean => {
                if arg_results.len() >= 2 && !arg_results[1].is_empty() {
                    let base = &arg_results[0];
                    let alpha = arg_results[1][0];
                    let adjust = if arg_results.len() >= 3 && !arg_results[2].is_empty() { 
                        arg_results[2][0] != 0.0 
                    } else { 
                        true 
                    };
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
            }
            Function::Std => {
                if !arg_results.is_empty() {
                    let data = &arg_results[0];
                    if data.len() > 1 {
                        let mean = data.iter().sum::<f64>() / data.len() as f64;
                        let variance = data.iter()
                            .map(|x| (x - mean).powi(2))
                            .sum::<f64>() / (data.len() - 1) as f64;
                        let std = variance.sqrt();
                        out.resize(len, std);
                    } else {
                        out.resize(len, f64::NAN);
                    }
                }
            }
            Function::Var => {
                if !arg_results.is_empty() {
                    let data = &arg_results[0];
                    if data.len() > 1 {
                        let mean = data.iter().sum::<f64>() / data.len() as f64;
                        let variance = data.iter()
                            .map(|x| (x - mean).powi(2))
                            .sum::<f64>() / (data.len() - 1) as f64;
                        out.resize(len, variance);
                    } else {
                        out.resize(len, f64::NAN);
                    }
                }
            }
            Function::Median => {
                if !arg_results.is_empty() {
                    let mut data = arg_results[0].clone();
                    if !data.is_empty() {
                        data.sort_by(|a, b| a.partial_cmp(b).unwrap());
                        let n = data.len();
                        let median = if n % 2 == 1 {
                            data[n / 2]
                        } else {
                            (data[n / 2 - 1] + data[n / 2]) * 0.5
                        };
                        out.resize(len, median);
                    } else {
                        out.resize(len, f64::NAN);
                    }
                }
            }
            Function::RollingStd => {
                if arg_results.len() >= 2 && !arg_results[1].is_empty() {
                    let base = &arg_results[0];
                    let win = arg_results[1][0] as usize;
                    for i in 0..len {
                        if i + 1 < win { 
                            out.push(f64::NAN); 
                        } else {
                            let slice = &base[i + 1 - win..=i];
                            let m = slice.iter().copied().sum::<f64>() / win as f64;
                            let var = slice.iter().map(|v| (v - m)*(v - m)).sum::<f64>() / win as f64;
                            out.push(var.sqrt());
                        }
                    }
                }
            }
            Function::RollingVar => {
                if arg_results.len() >= 2 && !arg_results[1].is_empty() {
                    let base = &arg_results[0];
                    let win = arg_results[1][0] as usize;
                    for i in 0..len {
                        if i + 1 < win { 
                            out.push(f64::NAN); 
                        } else {
                            let slice = &base[i + 1 - win..=i];
                            let m = slice.iter().copied().sum::<f64>() / win as f64;
                            let var = slice.iter().map(|v| (v - m)*(v - m)).sum::<f64>() / win as f64;
                            out.push(var);
                        }
                    }
                }
            }
            Function::RollingMedian => {
                if arg_results.len() >= 2 && !arg_results[1].is_empty() {
                    let base = &arg_results[0];
                    let win = arg_results[1][0] as usize;
                    for i in 0..len {
                        if i + 1 < win { 
                            out.push(f64::NAN); 
                        } else {
                            let mut v = base[i + 1 - win..=i].to_vec();
                            v.sort_by(|a,b| a.partial_cmp(b).unwrap());
                            let k = v.len();
                            let med = if k % 2 == 1 { v[k/2] } else { (v[k/2 -1] + v[k/2]) * 0.5 };
                            out.push(med);
                        }
                    }
                }
            }
            // Time-based rolling functions - placeholder implementations
            Function::RollingMeanTime | Function::RollingSumTime | 
            Function::RollingStdTime | Function::RollingVarTime | Function::RollingMedianTime => {
                // For now, fall back to row-based behavior
                // TODO: Implement time-based windows with time column awareness
                out.resize(len, f64::NAN);
            }
        }
        out
    }

    /// Lower to a Polars expression when possible.
    pub fn to_polars_expr(&self) -> Option<polars::lazy::dsl::Expr> {
        use polars::lazy::dsl::{col, lit};
        match &self.ast.node {
            ExprNode::Column(name) => Some(col(name)),
            ExprNode::Literal(v) => Some(lit(*v)),
            ExprNode::Call(fun, args) => match fun {
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
                        plan: None,
                        cache: None,
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
                    let base = Self { 
                        ast: args[0].clone(), 
                        plan: None, 
                        cache: None 
                    }.to_polars_expr().unwrap();
                    let mut acc = base.clone();
                    for k in 1..n {
                        acc = acc + base.clone().shift(lit(k as i64));
                    }
                    acc
                }),
                // Cumulative functions - use scalar fallback for determinism
                Function::CumSum | Function::CumProd | Function::CumMin | Function::CumMax => {
                    // Complex cumulative functions fall back to scalar evaluation
                    None
                },
                
                // Statistical functions - keep as scalar fallback for determinism
                Function::Std | Function::Var | Function::Median | Function::EwmMean => None,
                
                // Rolling functions with additional types - use scalar fallback for now
                Function::RollingStd | Function::RollingVar | Function::RollingMedian => {
                    // Complex rolling functions fall back to scalar evaluation
                    None
                },
                
                // Time-based rolling and other functions: scalar fallback for now
                _ => None,
            },
        }
    }

    fn lower_unary_int<F>(e: &Expr, n: Option<&Expr>, f: F) -> polars::prelude::Expr
    where
        F: FnOnce(polars::prelude::Expr, usize) -> polars::prelude::Expr,
    {
        let x = Self { ast: e.clone(), plan: None, cache: None }.to_polars_expr().unwrap();
        let n = n.map(|n_expr| {
            match &n_expr.node {
                ExprNode::Literal(val) => *val as usize,
                _ => 1,
            }
        }).unwrap_or(1);
        f(x, n)
    }

    fn lower_binary<F>(lhs: &Expr, rhs: &Expr, f: F) -> polars::prelude::Expr
    where
        F: FnOnce(polars::prelude::Expr, &Expr) -> polars::prelude::Expr,
    {
        let x = Self { ast: lhs.clone(), plan: None, cache: None }.to_polars_expr().unwrap();
        f(x, rhs)
    }
}

fn arg_as_usize(e: &Expr) -> usize {
    match &e.node {
        ExprNode::Literal(v) => (*v as i64).max(0) as usize,
        _ => 0,
    }
}
fn arg_as_i64(e: &Expr) -> i64 {
    match &e.node {
        ExprNode::Literal(v) => (*v as i64).abs(),
        _ => 0,
    }
}
#[allow(dead_code)]
fn arg_as_f64(e: &Expr) -> f64 {
    match &e.node {
        ExprNode::Literal(v) => *v,
        _ => 0.0,
    }
}
