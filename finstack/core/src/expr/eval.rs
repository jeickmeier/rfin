//! Scalar evaluator and Polars lowering with DAG planning and caching.

use super::{
    ast::*,
    cache::{CacheManager, CachedResult},
    context::ExpressionContext,
    dag::{DagBuilder, ExecutionPlan},
};
use polars::prelude as pl;
use polars::prelude::{IntoColumn, IntoLazy, NamedFrom};
use std::collections::HashSet;
use std::vec::Vec;

/// Options controlling evaluation strategy and caching.
///
/// Examples:
/// - Evaluate with a cache budget:
/// ```no_run
/// use finstack_core::expr::{CompiledExpr, Expr, SimpleContext, EvalOpts};
/// let ctx = SimpleContext::new(["x"]);
/// let x = vec![1.0, 2.0, 3.0];
/// let cols: [&[f64]; 1] = [&x];
/// let expr = CompiledExpr::new(Expr::column("x"));
/// let out = expr.eval(&ctx, &cols, EvalOpts { plan: None, cache_budget_mb: Some(16) });
/// assert_eq!(out.values, vec![1.0, 2.0, 3.0]);
/// ```
#[derive(Clone, Debug, Default)]
pub struct EvalOpts {
    /// Optional pre-built execution plan to follow. If not provided, the
    /// evaluator will either use the internal plan (if present) or fallback to
    /// a minimal evaluation path for the expression.
    pub plan: Option<ExecutionPlan>,
    /// Optional cache budget in megabytes. When provided, a cache will be
    /// instantiated (and sized for the plan when available) and cache stats
    /// will be embedded in the returned metadata.
    pub cache_budget_mb: Option<usize>,
}

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
    pub fn with_planning(ast: Expr, meta: crate::config::ResultsMeta) -> Self {
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

    /// Unified evaluation entrypoint returning values with execution metadata.
    ///
    /// This replaces legacy variants and will use either a provided plan,
    /// an internal plan, or a minimal scalar/Polars path.
    pub fn eval<C: ExpressionContext>(
        &self,
        ctx: &C,
        cols: &[&[crate::F]],
        opts: EvalOpts,
    ) -> EvaluationResult {
        let start_time = std::time::Instant::now();

        // Decide on execution plan preference: opts > self > none
        let plan_to_use: Option<ExecutionPlan> = if let Some(p) = opts.plan {
            Some(p)
        } else {
            self.plan.clone()
        };

        // Decide on cache to use for this evaluation
        let eval_cache: Option<CacheManager> = if let Some(budget) = opts.cache_budget_mb {
            if let Some(ref p) = plan_to_use {
                Some(CacheManager::for_plan(p, budget))
            } else {
                Some(CacheManager::new(budget))
            }
        } else {
            self.cache.clone()
        };

        // Compute values using the chosen strategy
        let values: Vec<crate::F> = if let Some(ref plan) = plan_to_use {
            // Execute nodes in topological order, honoring cache strategy
            let mut results: std::collections::HashMap<u64, Vec<crate::F>> =
                std::collections::HashMap::new();

            for node in &plan.nodes {
                // Cache lookup
                if let Some(ref cache) = eval_cache {
                    if let Some(cached) = cache.get(node.id) {
                        if let Ok(scalar_result) = cached.as_scalar() {
                            results.insert(node.id, scalar_result);
                            continue;
                        }
                    }
                }

                // Evaluate node
                let result = self.eval_node(ctx, cols, node, &results);

                // Cache store
                if let Some(ref cache) = eval_cache {
                    if plan.cache_strategy.cache_nodes.contains(&node.id) {
                        let arc: std::sync::Arc<[crate::F]> = std::sync::Arc::from(result.clone().into_boxed_slice());
                        cache.put(node.id, CachedResult::Scalar(arc));
                    }
                }

                results.insert(node.id, result);
            }

            // Root result
            if let Some(&root_id) = plan.roots.first() {
                results.remove(&root_id).unwrap_or_default()
            } else {
                Vec::new()
            }
        } else {
            // No plan: try Polars lowering for the whole expression first
            if let Some(v) = self.eval_via_polars(ctx, cols, &self.ast) {
                v
            } else {
                self.eval_simple(ctx, cols, &self.ast)
            }
        };

        // Stamp metadata
        let mut meta = crate::config::results_meta(&crate::config::FinstackConfig::default());
        meta.execution_time_ns = Some(start_time.elapsed().as_nanos() as u64);
        meta.cache_hit_ratio = eval_cache.as_ref().map(|c| c.hit_ratio());
        meta.parallel = plan_to_use.is_some();

        EvaluationResult { values, metadata: meta }
    }

    /// Evaluate a single DAG node.
    fn eval_node<C: ExpressionContext>(
        &self,
        ctx: &C,
        cols: &[&[crate::F]],
        node: &super::dag::DagNode,
        results: &std::collections::HashMap<u64, Vec<crate::F>>,
    ) -> Vec<crate::F> {
        // If node is Polars-eligible, attempt evaluation via Polars
        if node.polars_eligible {
            if let Some(v) = self.eval_via_polars(ctx, cols, &node.expr) {
                return v;
            }
        }

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
                let arg_results: Vec<Vec<crate::F>> = node
                    .dependencies
                    .iter()
                    .map(|&dep_id| results.get(&dep_id).cloned().unwrap_or_else(Vec::new))
                    .collect();

                self.eval_function(*func, &arg_results, ctx, cols)
            }
        }
    }

    /// Simple evaluation without DAG planning (legacy path).
    fn eval_simple<C: ExpressionContext>(
        &self,
        ctx: &C,
        cols: &[&[crate::F]],
        expr: &Expr,
    ) -> Vec<crate::F> {
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
                let arg_results: Vec<Vec<crate::F>> = args
                    .iter()
                    .map(|arg| self.eval_simple(ctx, cols, arg))
                    .collect();
                return self.eval_function(*fun, &arg_results, ctx, cols);
            }
        }
        out
    }

    // --- Polars evaluation helper ---

    fn eval_via_polars<C: ExpressionContext>(
        &self,
        ctx: &C,
        cols: &[&[crate::F]],
        expr: &Expr,
    ) -> Option<Vec<crate::F>> {
        // Lower to a Polars Expr; if not possible, return None
        let pexpr = Self {
            ast: expr.clone(),
            plan: None,
            cache: None,
        }
        .to_polars_expr()?;
        let names = Self::collect_column_names(expr);
        if names.is_empty() {
            // A pure literal expression - defer to scalar path to match row count semantics.
            return None;
        }
        // Build a DataFrame with just the required columns
        let mut columns: Vec<pl::Column> = Vec::with_capacity(names.len());
        for name in names.iter() {
            let idx = ctx.resolve_index(name)?;
            let f64_vec: Vec<f64> = cols[idx].to_vec();
            let s = pl::Series::new(name.as_str().into(), f64_vec);
            columns.push(s.into_column());
        }
        let df = pl::DataFrame::new(columns).ok()?;
        let lf = df.lazy();
        let out = lf.select([pexpr.alias("__out")]).collect().ok()?;
        let s = out.column("__out").ok()?.clone();
        let vals: Vec<crate::F> = s
            .f64()
            .ok()?
            .into_iter()
            .map(|o| o.unwrap_or(f64::NAN) as crate::F)
            .collect();
        Some(vals)
    }

    fn collect_column_names(expr: &Expr) -> HashSet<String> {
        fn walk(e: &Expr, acc: &mut HashSet<String>) {
            match &e.node {
                ExprNode::Column(name) => {
                    acc.insert(name.clone());
                }
                ExprNode::Literal(_) => {}
                ExprNode::Call(_, args) => {
                    for a in args {
                        walk(a, acc);
                    }
                }
            }
        }
        let mut set = HashSet::new();
        walk(expr, &mut set);
        set
    }

    // --- Helper evaluators (scalar path) ---

    #[inline]
    fn rolling_apply(
        base: &[crate::F],
        win: usize,
        mut op: impl FnMut(&[crate::F]) -> crate::F,
    ) -> Vec<crate::F> {
        let len = base.len();
        let mut out = Vec::with_capacity(len);
        for i in 0..len {
            if i + 1 < win {
                out.push(f64::NAN as crate::F);
            } else {
                out.push(op(&base[i + 1 - win..=i]));
            }
        }
        out
    }

    fn eval_lag(&self, arg_results: &[Vec<crate::F>]) -> Vec<crate::F> {
        let len = arg_results.first().map(|a| a.len()).unwrap_or(0);
        let mut out = Vec::with_capacity(len);
        if arg_results.len() >= 2 && !arg_results[1].is_empty() {
            let base = &arg_results[0];
            let n = arg_results[1][0] as usize;
            out.extend((0..len).map(|i| {
                if i < n {
                    f64::NAN as crate::F
                } else {
                    base[i - n]
                }
            }));
        }
        out
    }

    fn eval_lead(&self, arg_results: &[Vec<crate::F>]) -> Vec<crate::F> {
        let len = arg_results.first().map(|a| a.len()).unwrap_or(0);
        let mut out = Vec::with_capacity(len);
        if arg_results.len() >= 2 && !arg_results[1].is_empty() {
            let base = &arg_results[0];
            let n = arg_results[1][0] as usize;
            out.extend((0..len).map(|i| {
                if i + n >= len {
                    f64::NAN as crate::F
                } else {
                    base[i + n]
                }
            }));
        }
        out
    }

    fn eval_diff(&self, arg_results: &[Vec<crate::F>]) -> Vec<crate::F> {
        let len = arg_results.first().map(|a| a.len()).unwrap_or(0);
        let mut out = Vec::with_capacity(len);
        if !arg_results.is_empty() {
            let base = &arg_results[0];
            let n = if arg_results.len() >= 2 && !arg_results[1].is_empty() {
                arg_results[1][0] as usize
            } else {
                1
            };
            out.extend((0..len).map(|i| {
                if i < n {
                    f64::NAN as crate::F
                } else {
                    base[i] - base[i - n]
                }
            }));
        }
        out
    }

    fn eval_pct_change(&self, arg_results: &[Vec<crate::F>]) -> Vec<crate::F> {
        let len = arg_results.first().map(|a| a.len()).unwrap_or(0);
        let mut out = Vec::with_capacity(len);
        if !arg_results.is_empty() {
            let base = &arg_results[0];
            let n = if arg_results.len() >= 2 && !arg_results[1].is_empty() {
                arg_results[1][0] as usize
            } else {
                1
            };
            out.extend((0..len).map(|i| {
                if i < n || base[i - n] == 0.0 {
                    f64::NAN as crate::F
                } else {
                    (base[i] / base[i - n]) - 1.0
                }
            }));
        }
        out
    }

    fn eval_cum_sum(&self, arg_results: &[Vec<crate::F>]) -> Vec<crate::F> {
        let len = arg_results.first().map(|a| a.len()).unwrap_or(0);
        let mut out = Vec::with_capacity(len);
        if !arg_results.is_empty() {
            let base = &arg_results[0];
            let mut acc: crate::F = 0.0;
            for &v in base {
                acc += v;
                out.push(acc);
            }
        }
        out
    }

    fn eval_cum_prod(&self, arg_results: &[Vec<crate::F>]) -> Vec<crate::F> {
        let len = arg_results.first().map(|a| a.len()).unwrap_or(0);
        let mut out = Vec::with_capacity(len);
        if !arg_results.is_empty() {
            let base = &arg_results[0];
            let mut acc: crate::F = 1.0;
            for &v in base {
                acc *= v;
                out.push(acc);
            }
        }
        out
    }

    fn eval_cum_min(&self, arg_results: &[Vec<crate::F>]) -> Vec<crate::F> {
        let len = arg_results.first().map(|a| a.len()).unwrap_or(0);
        let mut out = Vec::with_capacity(len);
        if !arg_results.is_empty() {
            let base = &arg_results[0];
            let mut cur = f64::INFINITY;
            for &v in base {
                cur = if cur < v { cur } else { v };
                out.push(cur);
            }
        }
        out
    }

    fn eval_cum_max(&self, arg_results: &[Vec<crate::F>]) -> Vec<crate::F> {
        let len = arg_results.first().map(|a| a.len()).unwrap_or(0);
        let mut out = Vec::with_capacity(len);
        if !arg_results.is_empty() {
            let base = &arg_results[0];
            let mut cur = f64::NEG_INFINITY;
            for &v in base {
                cur = if cur > v { cur } else { v };
                out.push(cur);
            }
        }
        out
    }

    fn eval_rolling_mean(&self, arg_results: &[Vec<crate::F>]) -> Vec<crate::F> {
        let len = arg_results.first().map(|a| a.len()).unwrap_or(0);
        if arg_results.len() < 2 || arg_results[1].is_empty() || len == 0 {
            return Vec::with_capacity(len);
        }
        let base = &arg_results[0];
        let win = arg_results[1][0] as usize;
        Self::rolling_apply(base, win, |w| w.iter().copied().sum::<crate::F>() / (win as crate::F))
    }

    fn eval_rolling_sum(&self, arg_results: &[Vec<crate::F>]) -> Vec<crate::F> {
        let len = arg_results.first().map(|a| a.len()).unwrap_or(0);
        if arg_results.len() < 2 || arg_results[1].is_empty() || len == 0 {
            return Vec::with_capacity(len);
        }
        let base = &arg_results[0];
        let win = arg_results[1][0] as usize;
        Self::rolling_apply(base, win, |w| w.iter().copied().sum::<crate::F>())
    }

    fn eval_ewm_mean(&self, arg_results: &[Vec<crate::F>]) -> Vec<crate::F> {
        let len = arg_results.first().map(|a| a.len()).unwrap_or(0);
        let mut out = Vec::with_capacity(len);
        if arg_results.len() >= 2 && !arg_results[1].is_empty() {
            let base = &arg_results[0];
            let alpha = arg_results[1][0];
            let adjust = if arg_results.len() >= 3 && !arg_results[2].is_empty() {
                arg_results[2][0] != 0.0
            } else {
                true
            };
            let mut outv = Vec::with_capacity(len);
            let mut prev: crate::F = 0.0;
            let mut wsum: crate::F = 0.0;
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
        out
    }

    fn eval_std(&self, arg_results: &[Vec<crate::F>]) -> Vec<crate::F> {
        let len = arg_results.first().map(|a| a.len()).unwrap_or(0);
        let mut out = Vec::with_capacity(len);
        if !arg_results.is_empty() {
            let data = &arg_results[0];
            if data.len() > 1 {
                let mean = data.iter().copied().sum::<f64>() / data.len() as f64;
                let variance = data
                    .iter()
                    .map(|&x| {
                        let dx = x - mean;
                        dx * dx
                    })
                    .sum::<f64>()
                    / (data.len() - 1) as f64;
                let std = variance.sqrt() as crate::F;
                out.resize(len, std);
            } else {
                out.resize(len, f64::NAN as crate::F);
            }
        }
        out
    }

    fn eval_var(&self, arg_results: &[Vec<crate::F>]) -> Vec<crate::F> {
        let len = arg_results.first().map(|a| a.len()).unwrap_or(0);
        let mut out = Vec::with_capacity(len);
        if !arg_results.is_empty() {
            let data = &arg_results[0];
            if data.len() > 1 {
                let mean = data.iter().copied().sum::<f64>() / data.len() as f64;
                let variance = data
                    .iter()
                    .map(|&x| {
                        let dx = x - mean;
                        dx * dx
                    })
                    .sum::<f64>()
                    / (data.len() - 1) as f64;
                out.resize(len, variance as crate::F);
            } else {
                out.resize(len, f64::NAN as crate::F);
            }
        }
        out
    }

    fn eval_median(&self, arg_results: &[Vec<crate::F>]) -> Vec<crate::F> {
        let len = arg_results.first().map(|a| a.len()).unwrap_or(0);
        let mut out = Vec::with_capacity(len);
        if !arg_results.is_empty() {
            let mut data = arg_results[0].clone();
            if !data.is_empty() {
                data.sort_by(|a, b| a.partial_cmp(b).unwrap());
                let n = data.len();
                let median = if n % 2 == 1 {
                    data[n / 2]
                } else {
                    (data[n / 2 - 1] + data[n / 2]) * (0.5 as crate::F)
                };
                out.resize(len, median);
            } else {
                out.resize(len, f64::NAN as crate::F);
            }
        }
        out
    }

    fn eval_rolling_std(&self, arg_results: &[Vec<crate::F>]) -> Vec<crate::F> {
        let len = arg_results.first().map(|a| a.len()).unwrap_or(0);
        if arg_results.len() < 2 || arg_results[1].is_empty() || len == 0 {
            return Vec::with_capacity(len);
        }
        let base = &arg_results[0];
        let win = arg_results[1][0] as usize;
        Self::rolling_apply(base, win, |w| {
            let m = w.iter().copied().sum::<f64>() / (win as f64);
            let var = w.iter().map(|v| {
                let dv = *v - m;
                dv * dv
            }).sum::<f64>() / (win as f64);
            var.sqrt() as crate::F
        })
    }

    fn eval_rolling_var(&self, arg_results: &[Vec<crate::F>]) -> Vec<crate::F> {
        let len = arg_results.first().map(|a| a.len()).unwrap_or(0);
        if arg_results.len() < 2 || arg_results[1].is_empty() || len == 0 {
            return Vec::with_capacity(len);
        }
        let base = &arg_results[0];
        let win = arg_results[1][0] as usize;
        Self::rolling_apply(base, win, |w| {
            let m = w.iter().copied().sum::<f64>() / (win as f64);
            let var = w.iter().map(|v| {
                let dv = *v - m;
                dv * dv
            }).sum::<f64>() / (win as f64);
            var as crate::F
        })
    }

    fn eval_rolling_median(&self, arg_results: &[Vec<crate::F>]) -> Vec<crate::F> {
        let len = arg_results.first().map(|a| a.len()).unwrap_or(0);
        if arg_results.len() < 2 || arg_results[1].is_empty() || len == 0 {
            return Vec::with_capacity(len);
        }
        let base = &arg_results[0];
        let win = arg_results[1][0] as usize;
        Self::rolling_apply(base, win, |w| {
            let mut v = w.to_vec();
            v.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let k = v.len();
            if k % 2 == 1 { v[k / 2] } else { (v[k / 2 - 1] + v[k / 2]) * (0.5 as crate::F) }
        })
    }

    // Time-based rolling (Dynamic windows) are handled via Polars only.

    fn eval_shift(&self, arg_results: &[Vec<crate::F>]) -> Vec<crate::F> {
        let len = arg_results.first().map(|a| a.len()).unwrap_or(0);
        let mut out = Vec::with_capacity(len);
        if arg_results.len() >= 2 && !arg_results[1].is_empty() {
            let base = &arg_results[0];
            let n = arg_results[1][0] as i32;
            for i in 0..len {
                let shifted_idx = i as i32 - n;
                if shifted_idx >= 0 && shifted_idx < len as i32 {
                    out.push(base[shifted_idx as usize]);
                } else {
                    out.push(f64::NAN as crate::F);
                }
            }
        }
        out
    }

    fn eval_rank(&self, arg_results: &[Vec<crate::F>]) -> Vec<crate::F> {
        let len = arg_results.first().map(|a| a.len()).unwrap_or(0);
        let mut out: Vec<crate::F> = Vec::with_capacity(len);
        if !arg_results.is_empty() {
            let base = &arg_results[0];
            let mut indexed: Vec<(f64, usize)> =
                base.iter().enumerate().map(|(i, &v)| (v, i)).collect();
            indexed.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
            let mut ranks: Vec<crate::F> = vec![0.0 as crate::F; len];
            let mut current_rank: f64 = 1.0;
            let mut last_value: f64 = f64::NAN;
            for (value, orig_idx) in indexed {
                if !value.is_nan() {
                    if value != last_value && !last_value.is_nan() {
                        current_rank += 1.0;
                    }
                    ranks[orig_idx] = current_rank as crate::F;
                    last_value = value;
                } else {
                    ranks[orig_idx] = f64::NAN as crate::F;
                }
            }
            out = ranks;
        }
        out
    }

    fn eval_quantile(&self, arg_results: &[Vec<crate::F>]) -> Vec<crate::F> {
        let len = arg_results.first().map(|a| a.len()).unwrap_or(0);
        let mut out = Vec::with_capacity(len);
        if arg_results.len() >= 2 && !arg_results[1].is_empty() {
            let base = &arg_results[0];
            let q = arg_results[1][0].clamp(0.0, 1.0);
            let mut valid_values: Vec<f64> = base
                .iter()
                .filter_map(|&x| if x.is_nan() { None } else { Some(x) })
                .collect();
            if !valid_values.is_empty() {
                valid_values.sort_by(|a, b| a.partial_cmp(b).unwrap());
                let index = q * (valid_values.len() - 1) as f64;
                let lower = index.floor() as usize;
                let upper = index.ceil() as usize;
                let quantile_value = if lower == upper {
                    valid_values[lower]
                } else {
                    let weight = index - lower as f64;
                    valid_values[lower] * (1.0 - weight) + valid_values[upper] * weight
                } as crate::F;
                out.resize(len, quantile_value);
            } else {
                out.resize(len, f64::NAN as crate::F);
            }
        }
        out
    }

    fn eval_rolling_min(&self, arg_results: &[Vec<crate::F>]) -> Vec<crate::F> {
        let len = arg_results.first().map(|a| a.len()).unwrap_or(0);
        if arg_results.len() < 2 || arg_results[1].is_empty() || len == 0 {
            return Vec::with_capacity(len);
        }
        let base = &arg_results[0];
        let win = arg_results[1][0] as usize;
        Self::rolling_apply(base, win, |w| {
            w.iter().copied().filter(|x| !x.is_nan()).min_by(|a, b| a.partial_cmp(b).unwrap()).unwrap_or(f64::NAN)
        })
    }

    fn eval_rolling_max(&self, arg_results: &[Vec<crate::F>]) -> Vec<crate::F> {
        let len = arg_results.first().map(|a| a.len()).unwrap_or(0);
        if arg_results.len() < 2 || arg_results[1].is_empty() || len == 0 {
            return Vec::with_capacity(len);
        }
        let base = &arg_results[0];
        let win = arg_results[1][0] as usize;
        Self::rolling_apply(base, win, |w| {
            w.iter().copied().filter(|x| !x.is_nan()).max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap_or(f64::NAN)
        })
    }

    fn eval_rolling_count(&self, arg_results: &[Vec<crate::F>]) -> Vec<crate::F> {
        let len = arg_results.first().map(|a| a.len()).unwrap_or(0);
        if arg_results.len() < 2 || arg_results[1].is_empty() || len == 0 {
            return Vec::with_capacity(len);
        }
        let base = &arg_results[0];
        let win = arg_results[1][0] as usize;
        Self::rolling_apply(base, win, |w| w.iter().copied().filter(|x| !x.is_nan()).count() as crate::F)
    }

    fn eval_ewm_std(&self, arg_results: &[Vec<crate::F>]) -> Vec<crate::F> {
        let len = arg_results.first().map(|a| a.len()).unwrap_or(0);
        let mut out: Vec<crate::F> = Vec::with_capacity(len);
        if arg_results.len() >= 2 && !arg_results[1].is_empty() {
            let base = &arg_results[0];
            let alpha = arg_results[1][0].clamp(0.001, 0.999);
            let adjust = arg_results
                .get(2)
                .and_then(|v| v.first())
                .map(|&x| x > 0.0)
                .unwrap_or(true);

            let mut ema = base[0];
            let mut ema_sq = base[0] * base[0];
            let mut n: crate::F = 1.0;

            out.push(0.0);

            for &value in base.iter().skip(1) {
                if !value.is_nan() {
                    n += 1.0;
                    let n_f64 = n as f64;
                    let alpha_f64 = alpha;
                    let weight = if adjust {
                        alpha_f64 / (1.0 - (1.0 - alpha_f64).powf(n_f64))
                    } else {
                        alpha_f64
                    };
                    ema = (((1.0 - weight) as crate::F) * ema) + ((weight as crate::F) * value);
                    ema_sq = (((1.0 - weight) as crate::F) * ema_sq)
                        + ((weight as crate::F) * value * value);
                    let variance = ema_sq - ema * ema;
                    out.push(if variance > 0.0 { variance.sqrt() } else { 0.0 });
                } else {
                    out.push(f64::NAN as crate::F);
                }
            }
        }
        out
    }

    fn eval_ewm_var(&self, arg_results: &[Vec<crate::F>]) -> Vec<crate::F> {
        let len = arg_results.first().map(|a| a.len()).unwrap_or(0);
        let mut out: Vec<crate::F> = Vec::with_capacity(len);
        if arg_results.len() >= 2 && !arg_results[1].is_empty() {
            let base = &arg_results[0];
            let alpha = arg_results[1][0].clamp(0.001, 0.999);
            let adjust = arg_results
                .get(2)
                .and_then(|v| v.first())
                .map(|&x| x > 0.0)
                .unwrap_or(true);

            let mut ema = base[0];
            let mut ema_sq = base[0] * base[0];
            let mut n: crate::F = 1.0;

            out.push(0.0);

            for &value in base.iter().skip(1) {
                if !value.is_nan() {
                    n += 1.0;
                    let n_f64 = n as f64;
                    let alpha_f64 = alpha;
                    let weight = if adjust {
                        alpha_f64 / (1.0 - (1.0 - alpha_f64).powf(n_f64))
                    } else {
                        alpha_f64
                    };
                    ema = (((1.0 - weight) as crate::F) * ema) + ((weight as crate::F) * value);
                    ema_sq = (((1.0 - weight) as crate::F) * ema_sq)
                        + ((weight as crate::F) * value * value);
                    let variance = ema_sq - ema * ema;
                    out.push(if variance > 0.0 { variance } else { 0.0 });
                } else {
                    out.push(f64::NAN as crate::F);
                }
            }
        }
        out
    }

    /// Evaluate a function with given argument results.
    fn eval_function<C: ExpressionContext>(
        &self,
        fun: Function,
        arg_results: &[Vec<crate::F>],
        _ctx: &C,
        _cols: &[&[crate::F]],
    ) -> Vec<crate::F> {
        match fun {
            Function::Lag => self.eval_lag(arg_results),
            Function::Lead => self.eval_lead(arg_results),
            Function::Diff => self.eval_diff(arg_results),
            Function::PctChange => self.eval_pct_change(arg_results),
            Function::CumSum => self.eval_cum_sum(arg_results),
            Function::CumProd => self.eval_cum_prod(arg_results),
            Function::CumMin => self.eval_cum_min(arg_results),
            Function::CumMax => self.eval_cum_max(arg_results),
            Function::RollingMean => self.eval_rolling_mean(arg_results),
            Function::RollingSum => self.eval_rolling_sum(arg_results),
            Function::EwmMean => self.eval_ewm_mean(arg_results),
            Function::Std => self.eval_std(arg_results),
            Function::Var => self.eval_var(arg_results),
            Function::Median => self.eval_median(arg_results),
            Function::RollingStd => self.eval_rolling_std(arg_results),
            Function::RollingVar => self.eval_rolling_var(arg_results),
            Function::RollingMedian => self.eval_rolling_median(arg_results),

            Function::Shift => self.eval_shift(arg_results),
            Function::Rank => self.eval_rank(arg_results),
            Function::Quantile => self.eval_quantile(arg_results),
            Function::RollingMin => self.eval_rolling_min(arg_results),
            Function::RollingMax => self.eval_rolling_max(arg_results),
            Function::RollingCount => self.eval_rolling_count(arg_results),
            Function::EwmStd => self.eval_ewm_std(arg_results),
            Function::EwmVar => self.eval_ewm_var(arg_results),
        }
    }

    /// Lower to a Polars expression when possible.
    pub fn to_polars_expr(&self) -> Option<polars::lazy::dsl::Expr> {
        use polars::lazy::dsl::{col, lit};
        match &self.ast.node {
            ExprNode::Column(name) => Some(col(name)),
            #[cfg(not(feature = "decimal128"))]
            ExprNode::Literal(v) => Some(lit(*v)),
            #[cfg(feature = "decimal128")]
            ExprNode::Literal(_) => None,
            ExprNode::Call(fun, args) => match fun {
                Function::Lag => Self::lower_binary(&args[0], &args[1], |x, n| {
                    x.shift(lit(arg_as_i64(n)))
                }),
                Function::Lead => Self::lower_binary(&args[0], &args[1], |x, n| {
                    x.shift(lit(-(arg_as_i64(n))))
                }),
                Function::Diff => Self::lower_unary_int(&args[0], args.get(1), |x, n| {
                    x.clone() - x.shift(lit(n as i64))
                }),
                Function::PctChange => {
                    Self::lower_unary_int(&args[0], args.get(1), |x, n| {
                        (x.clone() / x.shift(lit(n as i64))) - lit(1.0)
                    })
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
                        cache: None,
                    }
                    .to_polars_expr()
                    .unwrap();
                    let mut acc = base.clone();
                    for k in 1..n {
                        acc = acc + base.clone().shift(lit(k as i64));
                    }
                    acc
                }),
                // Cumulative functions - fallback to scalar implementation for determinism
                Function::CumSum | Function::CumProd | Function::CumMin | Function::CumMax => {
                    // Cumulative functions use scalar implementation for consistent behavior
                    None
                }

                // Statistical functions
                Function::Std => Some({
                    let base = Self {
                        ast: args[0].clone(),
                        plan: None,
                        cache: None,
                    }
                    .to_polars_expr()
                    .unwrap();
                    base.std(1) // ddof=1 for sample standard deviation
                }),
                Function::Var => Some({
                    let base = Self {
                        ast: args[0].clone(),
                        plan: None,
                        cache: None,
                    }
                    .to_polars_expr()
                    .unwrap();
                    base.var(1) // ddof=1 for sample variance
                }),
                Function::Median => Some({
                    let base = Self {
                        ast: args[0].clone(),
                        plan: None,
                        cache: None,
                    }
                    .to_polars_expr()
                    .unwrap();
                    base.median()
                }),
                // Complex EWM functions still use scalar fallback for now
                Function::EwmMean => None,

                // Rolling statistical functions - fallback to scalar for complex operations
                Function::RollingStd | Function::RollingVar | Function::RollingMedian => {
                    // Complex rolling statistical functions require scalar implementation
                    None
                }

                // New time-series functions with possible Polars lowering
                Function::Shift => {
                    let base = Self {
                        ast: args[0].clone(),
                        plan: None,
                        cache: None,
                    }
                    .to_polars_expr()?;
                    let n = arg_as_i64(&args[1]);
                    Some(base.shift(lit(n)))
                }

                Function::RollingMin => {
                    // Use scalar fallback for now - proper rolling min/max need window options
                    None
                }

                Function::RollingMax => {
                    // Use scalar fallback for now - proper rolling min/max need window options
                    None
                }

                // Complex functions: use scalar fallback for determinism
                Function::Rank
                | Function::Quantile
                | Function::RollingCount
                | Function::EwmStd
                | Function::EwmVar => None,


            },
        }
    }

    fn lower_unary_int<FN>(e: &Expr, n: Option<&Expr>, f: FN) -> Option<polars::prelude::Expr>
    where
        FN: FnOnce(polars::prelude::Expr, usize) -> polars::prelude::Expr,
    {
        let x = Self {
            ast: e.clone(),
            plan: None,
            cache: None,
        }
        .to_polars_expr()?;
        let n = n
            .map(|n_expr| match &n_expr.node {
                ExprNode::Literal(val) => (*val as i64).max(0) as usize,
                _ => 1,
            })
            .unwrap_or(1);
        Some(f(x, n))
    }

    fn lower_binary<FN>(lhs: &Expr, rhs: &Expr, f: FN) -> Option<polars::prelude::Expr>
    where
        FN: FnOnce(polars::prelude::Expr, &Expr) -> polars::prelude::Expr,
    {
        let x = Self {
            ast: lhs.clone(),
            plan: None,
            cache: None,
        }
        .to_polars_expr()?;
        Some(f(x, rhs))
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
// Note: arg_as_f64 removed as unused helper
