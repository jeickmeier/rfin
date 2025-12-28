//! Scalar expression evaluation with DAG optimization.
//!
//! Provides the `CompiledExpr` type that evaluates expression trees using
//! optimized scalar algorithms. Supports DAG planning for shared sub-expression
//! elimination and LRU caching for intermediate results.
//!
//! # Evaluation Strategy
//!
//! - **Simple mode**: Direct recursive evaluation (no planning)
//! - **DAG mode**: Topological order execution with caching
//! - **Scratch buffers**: Reused to minimize allocations
//! - **Deterministic**: Identical results across runs

use super::{
    ast::*,
    cache::{CacheManager, CachedResult},
    context::ExpressionContext,
    dag::{DagBuilder, ExecutionPlan},
};
use crate::collections::HashMap;
use std::sync::Mutex;
use std::vec::Vec;

/// Options controlling expression evaluation strategy and caching.
///
/// Allows callers to override the execution plan and cache budget for a single
/// evaluation. Useful for scenario analysis where different cache sizes or
/// plans may be beneficial.
///
/// # Fields
///
/// - `plan`: Optional pre-built execution plan (overrides compiled plan)
/// - `cache_budget_mb`: Optional cache size in megabytes
///
/// # Examples
///
/// ```rust
/// use finstack_core::expr::{CompiledExpr, Expr, SimpleContext, EvalOpts};
///
/// let ctx = SimpleContext::new(["x"]);
/// let x = vec![1.0, 2.0, 3.0];
/// let cols: [&[f64]; 1] = [&x];
/// let expr = CompiledExpr::new(Expr::column("x"));
///
/// // Evaluate with custom cache
/// let opts = EvalOpts {
///     plan: None,
///     cache_budget_mb: Some(16),
/// };
/// let out = expr.eval(&ctx, &cols, opts);
/// assert_eq!(out.values, vec![1.0, 2.0, 3.0]);
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

/// Compiled expression with optimized evaluation and caching.
///
/// Wraps an expression AST with optional DAG planning and result caching for
/// efficient evaluation of complex formulas. Used extensively in financial
/// statement models where hundreds of interdependent formulas must be evaluated.
///
/// # Components
///
/// - **AST**: Expression tree to evaluate
/// - **Plan**: Optional execution plan (topological order, cache strategy)
/// - **Cache**: LRU cache for intermediate results
/// - **Scratch buffers**: Reused temporary storage to minimize allocations
///
/// # Evaluation Modes
///
/// - **Simple**: Direct recursive evaluation (fast for simple expressions)
/// - **DAG-optimized**: Shared sub-expression elimination (best for complex graphs)
/// - **Cached**: LRU caching of intermediate results (best for repeated patterns)
///
/// # Thread Safety
///
/// Not `Sync` due to mutable scratch buffers. Clone to share across threads.
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
        // Decide on execution plan preference: opts > self > auto-build
        let plan_to_use: ExecutionPlan =
            opts.plan.or_else(|| self.plan.clone()).unwrap_or_else(|| {
                let mut builder = DagBuilder::new();
                let meta = crate::config::results_meta(&crate::config::FinstackConfig::default());
                builder.build_plan(vec![self.ast.clone()], meta)
            });

        // Decide on cache to use for this evaluation
        let eval_cache: Option<CacheManager> = if let Some(budget) = opts.cache_budget_mb {
            Some(CacheManager::for_plan(&plan_to_use, budget))
        } else {
            self.cache.clone()
        };

        // Compute values using the chosen strategy
        let values: Vec<f64> = {
            // Execute nodes in topological order using arena allocation
            let len = cols.first().map(|c| c.len()).unwrap_or(0);

            // Pre-allocate arena for all node results to avoid per-node Vec allocations
            let mut arena = vec![0.0; len * plan_to_use.nodes.len()];
            let mut offsets: HashMap<u64, (usize, usize)> = HashMap::default();
            let mut cursor = 0;

            for node in &plan_to_use.nodes {
                // Cache lookup
                if let Some(ref cache) = eval_cache {
                    if let Some(cached) = cache.get(node.id, len) {
                        if let Ok(scalar_result) = cached.as_scalar() {
                            // Copy cached result into arena
                            debug_assert_eq!(scalar_result.len(), len);
                            let start = cursor;
                            let end = cursor + len;
                            arena[start..end].copy_from_slice(&scalar_result[..len]);
                            offsets.insert(node.id, (start, end));
                            cursor = end;
                            continue;
                        }
                    }
                }

                // Allocate space in arena for this node's result
                let start = cursor;
                let end = cursor + len;

                // Evaluate node directly into arena slice
                // Split the arena to avoid borrow conflicts
                let (arena_deps, arena_out) = arena.split_at_mut(start);
                let out_slice = &mut arena_out[..len];
                self.eval_node_into(ctx, cols, node, arena_deps, &offsets, out_slice);

                // Cache store
                if let Some(ref cache) = eval_cache {
                    if plan_to_use.cache_strategy.cache_nodes.contains(&node.id) {
                        let arc: std::sync::Arc<[f64]> =
                            std::sync::Arc::from(arena[start..end].to_vec().into_boxed_slice());
                        cache.put(node.id, CachedResult::Scalar(arc));
                    }
                }

                offsets.insert(node.id, (start, end));
                cursor = end;
            }

            // Extract root result
            if let Some(&root_id) = plan_to_use.roots.first() {
                if let Some(&(start, end)) = offsets.get(&root_id) {
                    arena[start..end].to_vec()
                } else {
                    Vec::new()
                }
            } else {
                Vec::new()
            }
        };

        // Stamp minimal metadata only at IO boundaries; evaluator does not record timings/cache/parallel
        let meta = crate::config::results_meta(&crate::config::FinstackConfig::default());

        EvaluationResult {
            values,
            metadata: meta,
        }
    }

    /// Evaluate a single DAG node directly into a provided output slice (arena-based).
    fn eval_node_into<C: ExpressionContext>(
        &self,
        ctx: &C,
        cols: &[&[f64]],
        node: &super::dag::DagNode,
        arena: &[f64],
        offsets: &HashMap<u64, (usize, usize)>,
        out: &mut [f64],
    ) {
        match &node.expr.node {
            ExprNode::Column(name) => {
                if let Some(idx) = ctx.resolve_index(name) {
                    if let Some(col_data) = cols.get(idx) {
                        let len = out.len().min(col_data.len());
                        out[..len].copy_from_slice(&col_data[..len]);
                    } else {
                        // Column index out of bounds - fill with NaN
                        out.fill(f64::NAN);
                    }
                } else {
                    // Unknown column - fill with NaN
                    out.fill(f64::NAN);
                }
            }
            ExprNode::Literal(val) => {
                out.fill(*val);
            }
            ExprNode::Call(func, _args) => {
                // Get argument results from dependencies (slices from arena)
                let arg_slices: Vec<&[f64]> = node
                    .dependencies
                    .iter()
                    .filter_map(|&dep_id| {
                        offsets.get(&dep_id).map(|&(start, end)| &arena[start..end])
                    })
                    .collect();

                self.eval_function_into(*func, &arg_slices, ctx, cols, out);
            }
            ExprNode::BinOp { op, .. } => {
                // Binary operations should have exactly 2 dependencies
                if node.dependencies.len() >= 2 {
                    let left = offsets
                        .get(&node.dependencies[0])
                        .map(|&(start, end)| &arena[start..end])
                        .unwrap_or(&[]);
                    let right = offsets
                        .get(&node.dependencies[1])
                        .map(|&(start, end)| &arena[start..end])
                        .unwrap_or(&[]);
                    Self::eval_bin_op_into(*op, left, right, out);
                }
            }
            ExprNode::UnaryOp { op, .. } => {
                // Unary operations should have exactly 1 dependency
                if !node.dependencies.is_empty() {
                    let operand = offsets
                        .get(&node.dependencies[0])
                        .map(|&(start, end)| &arena[start..end])
                        .unwrap_or(&[]);
                    Self::eval_unary_op_into(*op, operand, out);
                }
            }
            ExprNode::IfThenElse { .. } => {
                // If-then-else should have exactly 3 dependencies
                if node.dependencies.len() >= 3 {
                    let condition = offsets
                        .get(&node.dependencies[0])
                        .map(|&(start, end)| &arena[start..end])
                        .unwrap_or(&[]);
                    let then_vals = offsets
                        .get(&node.dependencies[1])
                        .map(|&(start, end)| &arena[start..end])
                        .unwrap_or(&[]);
                    let else_vals = offsets
                        .get(&node.dependencies[2])
                        .map(|&(start, end)| &arena[start..end])
                        .unwrap_or(&[]);
                    Self::eval_if_then_else_into(condition, then_vals, else_vals, out);
                }
            }
        }
    }

    // --- Scalar evaluators ---

    #[inline]
    fn validate_window(raw: f64) -> Option<usize> {
        if !raw.is_finite() {
            return None;
        }
        if raw < 1.0 {
            return None;
        }
        if raw.fract() != 0.0 {
            return None;
        }
        if raw > usize::MAX as f64 {
            return None;
        }
        Some(raw as usize)
    }

    #[inline]
    fn window_arg(arg_results: &[Vec<f64>], default: Option<usize>) -> Result<usize, ()> {
        if let Some(raw) = arg_results.get(1).and_then(|v| v.first()).copied() {
            Self::validate_window(raw).ok_or(())
        } else {
            default.ok_or(())
        }
    }

    #[inline]
    fn nan_output(len: usize) -> Vec<f64> {
        vec![f64::NAN; len]
    }

    #[inline]
    #[allow(dead_code)]
    fn rolling_apply(base: &[f64], win: usize, mut op: impl FnMut(&[f64]) -> f64) -> Vec<f64> {
        let len = base.len();
        if win == 0 {
            return Self::nan_output(len);
        }
        let mut out = vec![0.0; len];
        Self::rolling_apply_into(base, win, &mut out, &mut op);
        out
    }

    #[inline]
    fn rolling_with(
        &self,
        arg_results: &[Vec<f64>],
        default_win: Option<usize>,
        mut op: impl FnMut(&[f64]) -> f64,
    ) -> Vec<f64> {
        let len = arg_results.first().map(|a| a.len()).unwrap_or(0);
        if arg_results.is_empty() || len == 0 {
            return Vec::with_capacity(len);
        }
        let base = &arg_results[0];
        let win = match Self::window_arg(arg_results, default_win) {
            Ok(win) => win,
            Err(_) => return Self::nan_output(len),
        };
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
        if win == 0 {
            out.fill(f64::NAN);
            return;
        }
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
        if arg_results.is_empty() {
            return Vec::with_capacity(len);
        }
        let n = match Self::window_arg(arg_results, None) {
            Ok(n) => n,
            Err(_) => return Self::nan_output(len),
        };
        let base = &arg_results[0];
        (0..len)
            .map(|i| if i < n { f64::NAN } else { base[i - n] })
            .collect()
    }

    fn eval_lead(&self, arg_results: &[Vec<f64>]) -> Vec<f64> {
        let len = arg_results.first().map(|a| a.len()).unwrap_or(0);
        if arg_results.is_empty() {
            return Vec::with_capacity(len);
        }
        let n = match Self::window_arg(arg_results, None) {
            Ok(n) => n,
            Err(_) => return Self::nan_output(len),
        };
        let base = &arg_results[0];
        (0..len)
            .map(|i| if i + n >= len { f64::NAN } else { base[i + n] })
            .collect()
    }

    fn eval_diff(&self, arg_results: &[Vec<f64>]) -> Vec<f64> {
        let len = arg_results.first().map(|a| a.len()).unwrap_or(0);
        let mut out = Vec::with_capacity(len);
        if !arg_results.is_empty() {
            let base = &arg_results[0];
            let n = match Self::window_arg(arg_results, Some(1)) {
                Ok(n) => n,
                Err(_) => return Self::nan_output(len),
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
            let n = match Self::window_arg(arg_results, Some(1)) {
                Ok(n) => n,
                Err(_) => return Self::nan_output(len),
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
        self.rolling_with(arg_results, None, |w| {
            w.iter().copied().sum::<f64>() / w.len() as f64
        })
    }

    fn eval_rolling_sum(&self, arg_results: &[Vec<f64>]) -> Vec<f64> {
        self.rolling_with(arg_results, None, |w| w.iter().copied().sum())
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
            // Handle poisoned mutex by recovering the inner data
            let mut guard = self
                .scratch
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let tmp = &mut guard.tmp;
            tmp.truncate(0);
            tmp.extend_from_slice(data);
            // NaN values compare as Equal to maintain stable sort
            tmp.sort_unstable_by(|a, b| {
                a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
            });
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
        self.rolling_with(arg_results, None, |w| {
            let m = w.iter().copied().sum::<f64>() / (w.len() as f64);
            let var = w
                .iter()
                .map(|v| {
                    let dv = *v - m;
                    dv * dv
                })
                .sum::<f64>()
                / (w.len() as f64);
            var.sqrt()
        })
    }

    fn eval_rolling_var(&self, arg_results: &[Vec<f64>]) -> Vec<f64> {
        self.rolling_with(arg_results, None, |w| {
            let m = w.iter().copied().sum::<f64>() / (w.len() as f64);
            w.iter()
                .map(|v| {
                    let dv = *v - m;
                    dv * dv
                })
                .sum::<f64>()
                / (w.len() as f64)
        })
    }

    fn eval_rolling_median(&self, arg_results: &[Vec<f64>]) -> Vec<f64> {
        let len = arg_results.first().map(|a| a.len()).unwrap_or(0);
        if arg_results.is_empty() || len == 0 {
            return Vec::with_capacity(len);
        }
        let base = &arg_results[0];
        let win = match Self::window_arg(arg_results, None) {
            Ok(win) => win,
            Err(_) => return Self::nan_output(len),
        };
        let mut out = vec![0.0; len];
        // Use scratch arena to avoid per-window allocations.
        // Handle poisoned mutex by recovering the inner data
        let mut guard = self
            .scratch
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let wbuf = &mut guard.window;
        for i in 0..len {
            if i + 1 < win {
                out[i] = f64::NAN;
            } else {
                let start = i + 1 - win;
                let slice = &base[start..=i];
                wbuf.truncate(0);
                wbuf.extend_from_slice(slice);
                // NaN values compare as Equal to maintain stable sort
                wbuf.sort_unstable_by(|a, b| {
                    a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
                });
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

    fn eval_abs(&self, arg_results: &[Vec<f64>]) -> Vec<f64> {
        if let Some(base) = arg_results.first() {
            base.iter().map(|v| v.abs()).collect()
        } else {
            Vec::new()
        }
    }

    fn eval_sign(&self, arg_results: &[Vec<f64>]) -> Vec<f64> {
        if let Some(base) = arg_results.first() {
            base.iter()
                .map(|v| {
                    if v.is_nan() {
                        f64::NAN
                    } else if *v > 0.0 {
                        1.0
                    } else if *v < 0.0 {
                        -1.0
                    } else {
                        0.0
                    }
                })
                .collect()
        } else {
            Vec::new()
        }
    }

    fn eval_rank(&self, arg_results: &[Vec<f64>]) -> Vec<f64> {
        let len = arg_results.first().map(|a| a.len()).unwrap_or(0);
        if !arg_results.is_empty() {
            let base = &arg_results[0];
            let mut indexed: Vec<(f64, usize)> =
                base.iter().enumerate().map(|(i, &v)| (v, i)).collect();
            // NaN values compare as Equal to maintain stable sort
            indexed.sort_unstable_by(|a, b| {
                a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal)
            });
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
                // NaN values filtered above, but use unwrap_or for safety
                valid_values.sort_unstable_by(|a, b| {
                    a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
                });
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
        if arg_results.is_empty() || len == 0 {
            return Vec::with_capacity(len);
        }
        let base = &arg_results[0];
        let win = match Self::window_arg(arg_results, None) {
            Ok(win) => win,
            Err(_) => return Self::nan_output(len),
        };
        let mut out = vec![0.0; len];
        Self::rolling_apply_into(base, win, &mut out, &mut |w| {
            w.iter()
                .copied()
                .filter(|x| !x.is_nan())
                // NaN values filtered above, but use unwrap_or for safety
                .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                .unwrap_or(f64::NAN)
        });
        out
    }

    fn eval_rolling_max(&self, arg_results: &[Vec<f64>]) -> Vec<f64> {
        let len = arg_results.first().map(|a| a.len()).unwrap_or(0);
        if arg_results.is_empty() || len == 0 {
            return Vec::with_capacity(len);
        }
        let base = &arg_results[0];
        let win = match Self::window_arg(arg_results, None) {
            Ok(win) => win,
            Err(_) => return Self::nan_output(len),
        };
        let mut out = vec![0.0; len];
        Self::rolling_apply_into(base, win, &mut out, &mut |w| {
            w.iter()
                .copied()
                .filter(|x| !x.is_nan())
                // NaN values filtered above, but use unwrap_or for safety
                .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                .unwrap_or(f64::NAN)
        });
        out
    }

    fn eval_rolling_count(&self, arg_results: &[Vec<f64>]) -> Vec<f64> {
        let len = arg_results.first().map(|a| a.len()).unwrap_or(0);
        if arg_results.is_empty() || len == 0 {
            return Vec::with_capacity(len);
        }
        let base = &arg_results[0];
        let win = match Self::window_arg(arg_results, None) {
            Ok(win) => win,
            Err(_) => return Self::nan_output(len),
        };
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

    /// Evaluate a binary operation element-wise into a provided output slice.
    #[inline]
    fn eval_bin_op_into(op: super::ast::BinOp, left: &[f64], right: &[f64], out: &mut [f64]) {
        use super::ast::BinOp;
        let len = out.len();

        for (i, out_val) in out.iter_mut().enumerate().take(len) {
            let l = *left.get(i).unwrap_or(&0.0);
            let r = *right.get(i).unwrap_or(&0.0);

            *out_val = match op {
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
        }
    }

    /// Evaluate a binary operation element-wise.
    #[inline]
    fn eval_unary_op_into(op: super::ast::UnaryOp, operand: &[f64], out: &mut [f64]) {
        use super::ast::UnaryOp;
        let len = out.len().min(operand.len());
        for i in 0..len {
            out[i] = match op {
                UnaryOp::Neg => -operand[i],
                UnaryOp::Not => {
                    if operand[i] == 0.0 {
                        1.0
                    } else {
                        0.0
                    }
                }
            };
        }
    }

    /// Evaluate if-then-else element-wise into a provided output slice.
    #[inline]
    fn eval_if_then_else_into(
        condition: &[f64],
        then_vals: &[f64],
        else_vals: &[f64],
        out: &mut [f64],
    ) {
        let len = out.len();
        for (i, out_val) in out.iter_mut().enumerate().take(len) {
            let cond = *condition.get(i).unwrap_or(&0.0);
            let then_val = *then_vals.get(i).unwrap_or(&0.0);
            let else_val = *else_vals.get(i).unwrap_or(&0.0);
            *out_val = if cond != 0.0 { then_val } else { else_val };
        }
    }

    fn eval_function_core<C: ExpressionContext>(
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
            Function::Abs => self.eval_abs(arg_results),
            Function::Sign => self.eval_sign(arg_results),
            // Custom financial functions (should be evaluated at the statements layer)
            // Programming error: these should never reach core evaluator
            Function::Sum
            | Function::Mean
            | Function::Ttm
            | Function::Ytd
            | Function::Qtd
            | Function::FiscalYtd
            | Function::Annualize
            | Function::AnnualizeRate
            | Function::Coalesce
            | Function::GrowthRate => {
                // Debug builds panic to catch programming errors early
                debug_assert!(
                    false,
                    "Custom financial functions should be evaluated in the statements layer, not in core"
                );
                // Release builds return NaN (safe fallback)
                vec![f64::NAN; arg_results.first().map(|v| v.len()).unwrap_or(1)]
            }
        }
    }

    /// Evaluate a function with given argument results (slices from arena).
    fn eval_function_into<C: ExpressionContext>(
        &self,
        fun: Function,
        arg_slices: &[&[f64]],
        _ctx: &C,
        _cols: &[&[f64]],
        out: &mut [f64],
    ) {
        // Convert slices to Vec for existing function implementations
        // TODO: optimize individual functions to work with slices
        let arg_results: Vec<Vec<f64>> = arg_slices.iter().map(|&s| s.to_vec()).collect();
        let result = self.eval_function_core(fun, &arg_results, _ctx, _cols);
        let copy_len = out.len().min(result.len());
        out[..copy_len].copy_from_slice(&result[..copy_len]);
    }
}
#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use crate::config::FinstackConfig;
    use crate::expr::{BinOp, Expr, Function, SimpleContext, UnaryOp};

    fn sample_context() -> (SimpleContext, Vec<Vec<f64>>) {
        let ctx = SimpleContext::new(["x", "y"]);
        let data = vec![vec![0.2, 0.5, 3.0, 4.0], vec![0.5, 1.5, 2.5, 3.5]];
        (ctx, data)
    }

    #[test]
    fn eval_auto_builds_plan_for_if_binop_and_unary_nodes() {
        let (ctx, data) = sample_context();
        let cols: Vec<&[f64]> = data.iter().map(|v| v.as_slice()).collect();

        let condition = Expr::bin_op(BinOp::Gt, Expr::column("x"), Expr::column("y"));
        let then_branch = Expr::column("x");
        let else_branch = Expr::unary_op(UnaryOp::Neg, Expr::column("y"));
        let expr = Expr::if_then_else(condition, then_branch, else_branch);

        let compiled = CompiledExpr::new(expr);
        let result = compiled.eval(&ctx, &cols, EvalOpts::default()).values;

        assert_eq!(result.len(), 4);
        assert!((result[0] + 0.5).abs() < 1e-12);
        assert!((result[1] + 1.5).abs() < 1e-12);
        assert!((result[2] - 3.0).abs() < 1e-12);
        assert!((result[3] - 4.0).abs() < 1e-12);
    }

    #[test]
    fn eval_with_plan_and_cache_executes_rolling_functions() {
        let (ctx, data) = sample_context();
        let cols: Vec<&[f64]> = data.iter().map(|v| v.as_slice()).collect();

        let expr = Expr::call(
            Function::RollingSum,
            vec![Expr::column("x"), Expr::literal(2.0)],
        );
        let meta = crate::config::results_meta(&FinstackConfig::default());
        let compiled = CompiledExpr::with_planning(expr, meta).with_cache(1);

        let result = compiled
            .eval(
                &ctx,
                &cols,
                EvalOpts {
                    plan: None,
                    cache_budget_mb: Some(1),
                },
            )
            .values;

        assert!(result[0].is_nan());
        assert!((result[1] - 0.7).abs() < 1e-12);
        assert!((result[2] - 3.5).abs() < 1e-12);
        assert!((result[3] - 7.0).abs() < 1e-12);
    }

    #[test]
    fn eval_allows_external_plan_override() {
        let (ctx, data) = sample_context();
        let cols: Vec<&[f64]> = data.iter().map(|v| v.as_slice()).collect();
        let expr = Expr::call(Function::Diff, vec![Expr::column("x"), Expr::literal(1.0)]);
        let meta = crate::config::results_meta(&FinstackConfig::default());
        let compiled = CompiledExpr::with_planning(expr, meta);
        let external_plan = compiled.plan.clone();

        let result = compiled
            .eval(
                &ctx,
                &cols,
                EvalOpts {
                    plan: external_plan,
                    cache_budget_mb: None,
                },
            )
            .values;

        assert!(result[0].is_nan());
        assert!((result[1] - 0.3).abs() < 1e-12);
        assert!((result[2] - 2.5).abs() < 1e-12);
        assert!((result[3] - 1.0).abs() < 1e-12);
    }
}
