//! Scalar evaluator with DAG planning and caching.

use super::{
    ast::*,
    cache::{CacheManager, CachedResult},
    context::ExpressionContext,
    dag::{DagBuilder, ExecutionPlan},
};
use std::sync::Mutex;
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
/// # assert_eq!(out.values, vec![1.0, 2.0, 3.0]);
/// ```
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CompiledExpr {
    /// Underlying expression AST.
    pub ast: Expr,
    /// Optional execution plan for complex expressions.
    pub plan: Option<ExecutionPlan>,
    /// Cache manager for intermediate results.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub cache: Option<CacheManager>,
    /// Small scratch arena to reuse temporary buffers within hot paths.
    #[cfg_attr(feature = "serde", serde(skip, default = "default_scratch"))]
    scratch: Mutex<ScratchArena>,
}

#[cfg(feature = "serde")]
fn default_scratch() -> Mutex<ScratchArena> {
    Mutex::new(ScratchArena::default())
}

/// Tiny reusable scratch buffers for hot evaluation paths.
#[derive(Default, Debug)]
struct ScratchArena {
    /// Generic temporary buffer for algorithms (e.g., median, sorts).
    tmp: Vec<f64>,
    /// Window buffer for rolling operations that need a writable copy.
    window: Vec<f64>,
}

impl Clone for CompiledExpr {
    fn clone(&self) -> Self {
        Self {
            ast: self.ast.clone(),
            plan: self.plan.clone(),
            cache: self.cache.clone(),
            // Fresh scratch for clones; per-instance reuse only.
            scratch: Mutex::new(ScratchArena::default()),
        }
    }
}

impl CompiledExpr {
    /// Construct a new compiled expression from an AST.
    pub fn new(ast: Expr) -> Self {
        Self {
            ast,
            plan: None,
            cache: None,
            scratch: Mutex::new(ScratchArena::default()),
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
            scratch: Mutex::new(ScratchArena::default()),
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
    /// Uses scalar implementations for all functions, with optional DAG planning
    /// and caching for complex expressions.
    pub fn eval<C: ExpressionContext>(
        &self,
        ctx: &C,
        cols: &[&[f64]],
        opts: EvalOpts,
    ) -> EvaluationResult {
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
        let values: Vec<f64> = if let Some(ref plan) = plan_to_use {
            // Execute nodes in topological order, honoring cache strategy
            let mut results: std::collections::HashMap<u64, Vec<f64>> =
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
                        let arc: std::sync::Arc<[f64]> =
                            std::sync::Arc::from(result.clone().into_boxed_slice());
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
            // No plan: use simple scalar evaluation
            self.eval_simple(ctx, cols, &self.ast)
        };

        // Stamp minimal metadata only at IO boundaries; evaluator does not record timings/cache/parallel
        let meta = crate::config::results_meta(&crate::config::FinstackConfig::default());

        EvaluationResult {
            values,
            metadata: meta,
        }
    }

    /// Evaluate a single DAG node using scalar implementation.
    fn eval_node<C: ExpressionContext>(
        &self,
        ctx: &C,
        cols: &[&[f64]],
        node: &super::dag::DagNode,
        results: &std::collections::HashMap<u64, Vec<f64>>,
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
                let arg_results: Vec<Vec<f64>> = node
                    .dependencies
                    .iter()
                    .map(|&dep_id| results.get(&dep_id).cloned().unwrap_or_else(Vec::new))
                    .collect();

                self.eval_function(*func, &arg_results, ctx, cols)
            }
            ExprNode::BinOp { op, .. } => {
                // Binary operations should have exactly 2 dependencies
                let left = results
                    .get(&node.dependencies[0])
                    .cloned()
                    .unwrap_or_else(Vec::new);
                let right = results
                    .get(&node.dependencies[1])
                    .cloned()
                    .unwrap_or_else(Vec::new);
                Self::eval_bin_op(*op, &left, &right)
            }
            ExprNode::UnaryOp { op, .. } => {
                // Unary operations should have exactly 1 dependency
                let operand = results
                    .get(&node.dependencies[0])
                    .cloned()
                    .unwrap_or_else(Vec::new);
                Self::eval_unary_op(*op, &operand)
            }
            ExprNode::IfThenElse { .. } => {
                // If-then-else should have exactly 3 dependencies
                let condition = results
                    .get(&node.dependencies[0])
                    .cloned()
                    .unwrap_or_else(Vec::new);
                let then_vals = results
                    .get(&node.dependencies[1])
                    .cloned()
                    .unwrap_or_else(Vec::new);
                let else_vals = results
                    .get(&node.dependencies[2])
                    .cloned()
                    .unwrap_or_else(Vec::new);
                Self::eval_if_then_else(&condition, &then_vals, &else_vals)
            }
        }
    }

    /// Simple evaluation without DAG planning (legacy path).
    fn eval_simple<C: ExpressionContext>(&self, ctx: &C, cols: &[&[f64]], expr: &Expr) -> Vec<f64> {
        let len = cols.first().map(|c| c.len()).unwrap_or(0);
        let mut out = vec![0.0; len];
        match &expr.node {
            ExprNode::Column(name) => {
                let idx = ctx.resolve_index(name).expect("unknown column");
                // Preserve semantics: result length equals referenced column length
                return cols[idx].to_vec();
            }
            ExprNode::Literal(v) => {
                for x in &mut out {
                    *x = *v;
                }
            }
            ExprNode::Call(fun, args) => {
                // Recursively evaluate arguments
                let arg_results: Vec<Vec<f64>> = args
                    .iter()
                    .map(|arg| self.eval_simple(ctx, cols, arg))
                    .collect();
                return self.eval_function(*fun, &arg_results, ctx, cols);
            }
            ExprNode::BinOp { op, left, right } => {
                let left_result = self.eval_simple(ctx, cols, left);
                let right_result = self.eval_simple(ctx, cols, right);
                return Self::eval_bin_op(*op, &left_result, &right_result);
            }
            ExprNode::UnaryOp { op, operand } => {
                let operand_result = self.eval_simple(ctx, cols, operand);
                return Self::eval_unary_op(*op, &operand_result);
            }
            ExprNode::IfThenElse {
                condition,
                then_expr,
                else_expr,
            } => {
                let cond_result = self.eval_simple(ctx, cols, condition);
                let then_result = self.eval_simple(ctx, cols, then_expr);
                let else_result = self.eval_simple(ctx, cols, else_expr);
                return Self::eval_if_then_else(&cond_result, &then_result, &else_result);
            }
        }
        out
    }

    // --- Scalar evaluators ---

    #[inline]
    fn rolling_apply(base: &[f64], win: usize, mut op: impl FnMut(&[f64]) -> f64) -> Vec<f64> {
        let len = base.len();
        let mut out = vec![0.0; len];
        Self::rolling_apply_into(base, win, &mut out, &mut op);
        out
    }

    #[inline]
    fn rolling_apply_into(
        base: &[f64],
        win: usize,
        out: &mut [f64],
        op: &mut impl FnMut(&[f64]) -> f64,
    ) {
        let len = base.len();
        debug_assert_eq!(out.len(), len);
        for i in 0..len {
            if i + 1 < win {
                out[i] = f64::NAN;
            } else {
                out[i] = op(&base[i + 1 - win..=i]);
            }
        }
    }

    fn eval_lag(&self, arg_results: &[Vec<f64>]) -> Vec<f64> {
        let len = arg_results.first().map(|a| a.len()).unwrap_or(0);
        let mut out = Vec::with_capacity(len);
        if arg_results.len() >= 2 && !arg_results[1].is_empty() {
            let base = &arg_results[0];
            let n = arg_results[1][0] as usize;
            out.extend((0..len).map(|i| if i < n { f64::NAN } else { base[i - n] }));
        }
        out
    }

    fn eval_lead(&self, arg_results: &[Vec<f64>]) -> Vec<f64> {
        let len = arg_results.first().map(|a| a.len()).unwrap_or(0);
        let mut out = Vec::with_capacity(len);
        if arg_results.len() >= 2 && !arg_results[1].is_empty() {
            let base = &arg_results[0];
            let n = arg_results[1][0] as usize;
            out.extend((0..len).map(|i| if i + n >= len { f64::NAN } else { base[i + n] }));
        }
        out
    }

    fn eval_diff(&self, arg_results: &[Vec<f64>]) -> Vec<f64> {
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
                    f64::NAN
                } else {
                    base[i] - base[i - n]
                }
            }));
        }
        out
    }

    fn eval_pct_change(&self, arg_results: &[Vec<f64>]) -> Vec<f64> {
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
                    f64::NAN
                } else {
                    (base[i] / base[i - n]) - 1.0
                }
            }));
        }
        out
    }

    fn eval_cum_sum(&self, arg_results: &[Vec<f64>]) -> Vec<f64> {
        let len = arg_results.first().map(|a| a.len()).unwrap_or(0);
        let mut out = Vec::with_capacity(len);
        if !arg_results.is_empty() {
            let base = &arg_results[0];
            let mut acc = 0.0;
            for &v in base {
                acc += v;
                out.push(acc);
            }
        }
        out
    }

    fn eval_cum_prod(&self, arg_results: &[Vec<f64>]) -> Vec<f64> {
        let len = arg_results.first().map(|a| a.len()).unwrap_or(0);
        let mut out = Vec::with_capacity(len);
        if !arg_results.is_empty() {
            let base = &arg_results[0];
            let mut acc = 1.0;
            for &v in base {
                acc *= v;
                out.push(acc);
            }
        }
        out
    }

    fn eval_cum_min(&self, arg_results: &[Vec<f64>]) -> Vec<f64> {
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

    fn eval_cum_max(&self, arg_results: &[Vec<f64>]) -> Vec<f64> {
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

    fn eval_rolling_mean(&self, arg_results: &[Vec<f64>]) -> Vec<f64> {
        let len = arg_results.first().map(|a| a.len()).unwrap_or(0);
        if arg_results.len() < 2 || arg_results[1].is_empty() || len == 0 {
            return Vec::with_capacity(len);
        }
        let base = &arg_results[0];
        let win = arg_results[1][0] as usize;
        Self::rolling_apply(base, win, |w| w.iter().copied().sum::<f64>() / win as f64)
    }

    fn eval_rolling_sum(&self, arg_results: &[Vec<f64>]) -> Vec<f64> {
        let len = arg_results.first().map(|a| a.len()).unwrap_or(0);
        if arg_results.len() < 2 || arg_results[1].is_empty() || len == 0 {
            return Vec::with_capacity(len);
        }
        let base = &arg_results[0];
        let win = arg_results[1][0] as usize;
        Self::rolling_apply(base, win, |w| w.iter().copied().sum())
    }

    fn eval_ewm_mean(&self, arg_results: &[Vec<f64>]) -> Vec<f64> {
        let len = arg_results.first().map(|a| a.len()).unwrap_or(0);
        if arg_results.len() >= 2 && !arg_results[1].is_empty() {
            let mut out = vec![0.0; len];
            self.eval_ewm_mean_into(arg_results, &mut out);
            return out;
        }
        Vec::with_capacity(len)
    }

    fn eval_ewm_mean_into(&self, arg_results: &[Vec<f64>], out: &mut [f64]) {
        let len = out.len();
        if len == 0 {
            return;
        }
        let base = &arg_results[0];
        let alpha = arg_results[1][0];
        let adjust = if arg_results.len() >= 3 && !arg_results[2].is_empty() {
            arg_results[2][0] != 0.0
        } else {
            true
        };
        let mut prev: f64 = 0.0;
        let mut wsum: f64 = 0.0;
        for (i, &x) in base.iter().enumerate() {
            if i == 0 {
                prev = x;
                wsum = 1.0;
                out[0] = x;
                continue;
            }
            if adjust {
                wsum = 1.0 + (1.0 - alpha) * wsum;
            }
            prev = alpha * x + (1.0 - alpha) * prev;
            out[i] = prev / if adjust { wsum } else { 1.0 };
        }
    }

    fn eval_std(&self, arg_results: &[Vec<f64>]) -> Vec<f64> {
        let len = arg_results.first().map(|a| a.len()).unwrap_or(0);
        if !arg_results.is_empty() {
            let mut out = vec![0.0; len];
            self.eval_std_into(arg_results, &mut out);
            return out;
        }
        Vec::with_capacity(len)
    }

    fn eval_std_into(&self, arg_results: &[Vec<f64>], out: &mut [f64]) {
        let len = out.len();
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
            let std = variance.sqrt();
            for v in out.iter_mut().take(len) {
                *v = std;
            }
        } else {
            for v in out.iter_mut().take(len) {
                *v = f64::NAN;
            }
        }
    }

    fn eval_var(&self, arg_results: &[Vec<f64>]) -> Vec<f64> {
        let len = arg_results.first().map(|a| a.len()).unwrap_or(0);
        if !arg_results.is_empty() {
            let mut out = vec![0.0; len];
            self.eval_var_into(arg_results, &mut out);
            return out;
        }
        Vec::with_capacity(len)
    }

    fn eval_var_into(&self, arg_results: &[Vec<f64>], out: &mut [f64]) {
        let len = out.len();
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
            for v in out.iter_mut().take(len) {
                *v = variance;
            }
        } else {
            for v in out.iter_mut().take(len) {
                *v = f64::NAN;
            }
        }
    }

    fn eval_median(&self, arg_results: &[Vec<f64>]) -> Vec<f64> {
        let len = arg_results.first().map(|a| a.len()).unwrap_or(0);
        if !arg_results.is_empty() {
            let mut out = vec![0.0; len];
            self.eval_median_into(arg_results, &mut out);
            return out;
        }
        Vec::with_capacity(len)
    }

    fn eval_median_into(&self, arg_results: &[Vec<f64>], out: &mut [f64]) {
        let len = out.len();
        let data = &arg_results[0];
        if !data.is_empty() {
            let mut guard = self.scratch.lock().unwrap();
            let tmp = &mut guard.tmp;
            tmp.clear();
            tmp.extend_from_slice(data);
            tmp.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let n = tmp.len();
            let median = if n % 2 == 1 {
                tmp[n / 2]
            } else {
                (tmp[n / 2 - 1] + tmp[n / 2]) * (0.5)
            };
            for v in out.iter_mut().take(len) {
                *v = median;
            }
        } else {
            for v in out.iter_mut().take(len) {
                *v = f64::NAN;
            }
        }
    }

    fn eval_rolling_std(&self, arg_results: &[Vec<f64>]) -> Vec<f64> {
        let len = arg_results.first().map(|a| a.len()).unwrap_or(0);
        if arg_results.len() < 2 || arg_results[1].is_empty() || len == 0 {
            return Vec::with_capacity(len);
        }
        let base = &arg_results[0];
        let win = arg_results[1][0] as usize;
        let mut out = vec![0.0; len];
        Self::rolling_apply_into(base, win, &mut out, &mut |w| {
            let m = w.iter().copied().sum::<f64>() / (win as f64);
            let var = w
                .iter()
                .map(|v| {
                    let dv = *v - m;
                    dv * dv
                })
                .sum::<f64>()
                / (win as f64);
            var.sqrt()
        });
        out
    }

    fn eval_rolling_var(&self, arg_results: &[Vec<f64>]) -> Vec<f64> {
        let len = arg_results.first().map(|a| a.len()).unwrap_or(0);
        if arg_results.len() < 2 || arg_results[1].is_empty() || len == 0 {
            return Vec::with_capacity(len);
        }
        let base = &arg_results[0];
        let win = arg_results[1][0] as usize;
        let mut out = vec![0.0; len];
        Self::rolling_apply_into(base, win, &mut out, &mut |w| {
            let m = w.iter().copied().sum::<f64>() / (win as f64);
            let var = w
                .iter()
                .map(|v| {
                    let dv = *v - m;
                    dv * dv
                })
                .sum::<f64>()
                / (win as f64);
            var
        });
        out
    }

    fn eval_rolling_median(&self, arg_results: &[Vec<f64>]) -> Vec<f64> {
        let len = arg_results.first().map(|a| a.len()).unwrap_or(0);
        if arg_results.len() < 2 || arg_results[1].is_empty() || len == 0 {
            return Vec::with_capacity(len);
        }
        let base = &arg_results[0];
        let win = arg_results[1][0] as usize;
        let mut out = vec![0.0; len];
        // Use scratch arena to avoid per-window allocations.
        let mut guard = self.scratch.lock().unwrap();
        let wbuf = &mut guard.window;
        for i in 0..len {
            if i + 1 < win {
                out[i] = f64::NAN;
            } else {
                let start = i + 1 - win;
                let slice = &base[start..=i];
                wbuf.clear();
                wbuf.extend_from_slice(slice);
                wbuf.sort_by(|a, b| a.partial_cmp(b).unwrap());
                let k = wbuf.len();
                out[i] = if k % 2 == 1 {
                    wbuf[k / 2]
                } else {
                    (wbuf[k / 2 - 1] + wbuf[k / 2]) * (0.5)
                };
            }
        }
        out
    }

    // Time-based rolling (Dynamic windows) are handled via Polars only.

    fn eval_shift(&self, arg_results: &[Vec<f64>]) -> Vec<f64> {
        let len = arg_results.first().map(|a| a.len()).unwrap_or(0);
        if arg_results.len() >= 2 && !arg_results[1].is_empty() {
            let base = &arg_results[0];
            let n = arg_results[1][0] as i32;
            let mut out = vec![0.0; len];
            for (i, slot) in out.iter_mut().enumerate().take(len) {
                let shifted_idx = i as i32 - n;
                *slot = if shifted_idx >= 0 && shifted_idx < len as i32 {
                    base[shifted_idx as usize]
                } else {
                    f64::NAN
                };
            }
            return out;
        }
        Vec::with_capacity(len)
    }

    fn eval_rank(&self, arg_results: &[Vec<f64>]) -> Vec<f64> {
        let len = arg_results.first().map(|a| a.len()).unwrap_or(0);
        if !arg_results.is_empty() {
            let base = &arg_results[0];
            let mut indexed: Vec<(f64, usize)> =
                base.iter().enumerate().map(|(i, &v)| (v, i)).collect();
            indexed.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
            let mut out: Vec<f64> = vec![0.0; len];
            let mut current_rank: f64 = 1.0;
            let mut last_value: f64 = f64::NAN;
            for (value, orig_idx) in indexed {
                if !value.is_nan() {
                    if value != last_value && !last_value.is_nan() {
                        current_rank += 1.0;
                    }
                    out[orig_idx] = current_rank;
                    last_value = value;
                } else {
                    out[orig_idx] = f64::NAN;
                }
            }
            return out;
        }
        Vec::with_capacity(len)
    }

    fn eval_quantile(&self, arg_results: &[Vec<f64>]) -> Vec<f64> {
        let len = arg_results.first().map(|a| a.len()).unwrap_or(0);
        if arg_results.len() >= 2 && !arg_results[1].is_empty() {
            let base = &arg_results[0];
            let q = arg_results[1][0].clamp(0.0, 1.0);
            let mut valid_values: Vec<f64> = base
                .iter()
                .filter_map(|&x| if x.is_nan() { None } else { Some(x) })
                .collect();
            let mut out = vec![0.0; len];
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
                };
                for v in out.iter_mut().take(len) {
                    *v = quantile_value;
                }
            } else {
                for v in out.iter_mut().take(len) {
                    *v = f64::NAN;
                }
            }
            return out;
        }
        Vec::with_capacity(len)
    }

    fn eval_rolling_min(&self, arg_results: &[Vec<f64>]) -> Vec<f64> {
        let len = arg_results.first().map(|a| a.len()).unwrap_or(0);
        if arg_results.len() < 2 || arg_results[1].is_empty() || len == 0 {
            return Vec::with_capacity(len);
        }
        let base = &arg_results[0];
        let win = arg_results[1][0] as usize;
        let mut out = vec![0.0; len];
        Self::rolling_apply_into(base, win, &mut out, &mut |w| {
            w.iter()
                .copied()
                .filter(|x| !x.is_nan())
                .min_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap_or(f64::NAN)
        });
        out
    }

    fn eval_rolling_max(&self, arg_results: &[Vec<f64>]) -> Vec<f64> {
        let len = arg_results.first().map(|a| a.len()).unwrap_or(0);
        if arg_results.len() < 2 || arg_results[1].is_empty() || len == 0 {
            return Vec::with_capacity(len);
        }
        let base = &arg_results[0];
        let win = arg_results[1][0] as usize;
        let mut out = vec![0.0; len];
        Self::rolling_apply_into(base, win, &mut out, &mut |w| {
            w.iter()
                .copied()
                .filter(|x| !x.is_nan())
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap_or(f64::NAN)
        });
        out
    }

    fn eval_rolling_count(&self, arg_results: &[Vec<f64>]) -> Vec<f64> {
        let len = arg_results.first().map(|a| a.len()).unwrap_or(0);
        if arg_results.len() < 2 || arg_results[1].is_empty() || len == 0 {
            return Vec::with_capacity(len);
        }
        let base = &arg_results[0];
        let win = arg_results[1][0] as usize;
        let mut out = vec![0.0; len];
        Self::rolling_apply_into(base, win, &mut out, &mut |w| {
            w.iter().copied().filter(|x| !x.is_nan()).count() as f64
        });
        out
    }

    fn eval_ewm_std(&self, arg_results: &[Vec<f64>]) -> Vec<f64> {
        let len = arg_results.first().map(|a| a.len()).unwrap_or(0);
        let mut out: Vec<f64> = Vec::with_capacity(len);
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
            let mut n: f64 = 1.0;

            out.push(0.0);

            for &value in base.iter().skip(1) {
                if !value.is_nan() {
                    n += 1.0;
                    let n_f64 = n;
                    let alpha_f64 = alpha;
                    let weight = if adjust {
                        alpha_f64 / (1.0 - (1.0 - alpha_f64).powf(n_f64))
                    } else {
                        alpha_f64
                    };
                    ema = ((1.0 - weight) * ema) + (weight * value);
                    ema_sq = ((1.0 - weight) * ema_sq) + (weight * value * value);
                    let variance = ema_sq - ema * ema;
                    out.push(if variance > 0.0 { variance.sqrt() } else { 0.0 });
                } else {
                    out.push(f64::NAN);
                }
            }
        }
        out
    }

    fn eval_ewm_var(&self, arg_results: &[Vec<f64>]) -> Vec<f64> {
        let len = arg_results.first().map(|a| a.len()).unwrap_or(0);
        let mut out: Vec<f64> = Vec::with_capacity(len);
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
            let mut n: f64 = 1.0;

            out.push(0.0);

            for &value in base.iter().skip(1) {
                if !value.is_nan() {
                    n += 1.0;
                    let n_f64 = n;
                    let alpha_f64 = alpha;
                    let weight = if adjust {
                        alpha_f64 / (1.0 - (1.0 - alpha_f64).powf(n_f64))
                    } else {
                        alpha_f64
                    };
                    ema = ((1.0 - weight) * ema) + (weight * value);
                    ema_sq = ((1.0 - weight) * ema_sq) + (weight * value * value);
                    let variance = ema_sq - ema * ema;
                    out.push(if variance > 0.0 { variance } else { 0.0 });
                } else {
                    out.push(f64::NAN);
                }
            }
        }
        out
    }

    /// Evaluate a binary operation element-wise.
    fn eval_bin_op(op: super::ast::BinOp, left: &[f64], right: &[f64]) -> Vec<f64> {
        use super::ast::BinOp;
        let len = left.len().max(right.len());
        let mut out = Vec::with_capacity(len);

        for i in 0..len {
            let l = *left.get(i).unwrap_or(&0.0);
            let r = *right.get(i).unwrap_or(&0.0);

            let result = match op {
                // Arithmetic
                BinOp::Add => l + r,
                BinOp::Sub => l - r,
                BinOp::Mul => l * r,
                BinOp::Div => {
                    if r == 0.0 {
                        f64::NAN
                    } else {
                        l / r
                    }
                }
                BinOp::Mod => l % r,

                // Comparison (return 1.0 for true, 0.0 for false)
                BinOp::Eq => {
                    if l == r {
                        1.0
                    } else {
                        0.0
                    }
                }
                BinOp::Ne => {
                    if l != r {
                        1.0
                    } else {
                        0.0
                    }
                }
                BinOp::Lt => {
                    if l < r {
                        1.0
                    } else {
                        0.0
                    }
                }
                BinOp::Le => {
                    if l <= r {
                        1.0
                    } else {
                        0.0
                    }
                }
                BinOp::Gt => {
                    if l > r {
                        1.0
                    } else {
                        0.0
                    }
                }
                BinOp::Ge => {
                    if l >= r {
                        1.0
                    } else {
                        0.0
                    }
                }

                // Logical (treat non-zero as true)
                BinOp::And => {
                    if l != 0.0 && r != 0.0 {
                        1.0
                    } else {
                        0.0
                    }
                }
                BinOp::Or => {
                    if l != 0.0 || r != 0.0 {
                        1.0
                    } else {
                        0.0
                    }
                }
            };
            out.push(result);
        }
        out
    }

    /// Evaluate a unary operation element-wise.
    fn eval_unary_op(op: super::ast::UnaryOp, operand: &[f64]) -> Vec<f64> {
        use super::ast::UnaryOp;
        operand
            .iter()
            .map(|&val| match op {
                UnaryOp::Neg => -val,
                UnaryOp::Not => {
                    if val == 0.0 {
                        1.0
                    } else {
                        0.0
                    }
                }
            })
            .collect()
    }

    /// Evaluate if-then-else element-wise.
    fn eval_if_then_else(condition: &[f64], then_vals: &[f64], else_vals: &[f64]) -> Vec<f64> {
        let len = condition.len().max(then_vals.len()).max(else_vals.len());
        let mut out = Vec::with_capacity(len);

        for i in 0..len {
            let cond = *condition.get(i).unwrap_or(&0.0);
            let then_val = *then_vals.get(i).unwrap_or(&0.0);
            let else_val = *else_vals.get(i).unwrap_or(&0.0);

            out.push(if cond != 0.0 { then_val } else { else_val });
        }
        out
    }

    /// Evaluate a function with given argument results.
    fn eval_function<C: ExpressionContext>(
        &self,
        fun: Function,
        arg_results: &[Vec<f64>],
        _ctx: &C,
        _cols: &[&[f64]],
    ) -> Vec<f64> {
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

            // Custom financial functions (should be evaluated at the statements layer)
            Function::Sum
            | Function::Mean
            | Function::Ttm
            | Function::Annualize
            | Function::Coalesce => {
                panic!("Custom financial functions should be evaluated in the statements layer, not in core")
            }
        }
    }
}
