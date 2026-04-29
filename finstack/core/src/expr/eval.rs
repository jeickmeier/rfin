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
    context::SimpleContext,
    dag::{DagBuilder, ExecutionPlan},
};
use crate::collections::HashMap;
use smallvec::SmallVec;
use std::sync::{Mutex, OnceLock};
use std::vec::Vec;

/// Options controlling expression evaluation strategy and caching.
///
/// Allows callers to override the execution plan and cache budget for a single
/// evaluation. Useful for scenario analysis where different cache sizes or
/// plans may be beneficial.
///
/// # Fields
///
/// - `plan`: Internal optional pre-built execution plan, exposed through
///   [`EvalOpts::has_plan`].
/// - `cache_budget_mb`: Optional cache size in megabytes
/// - `max_arena_bytes`: Maximum scratch arena allocation in bytes
///
/// # Examples
///
/// ```rust
/// use finstack_core::expr::{CompiledExpr, Expr, SimpleContext, EvalOpts};
///
/// let ctx = SimpleContext::new(["x"]).expect("unique columns");
/// let x = vec![1.0, 2.0, 3.0];
/// let cols: [&[f64]; 1] = [&x];
/// let expr = CompiledExpr::new(Expr::column("x"));
///
/// // Evaluate with custom cache
/// let mut opts = EvalOpts::default();
/// opts.cache_budget_mb = Some(16);
/// let out = expr.eval(&ctx, &cols, opts).expect("column lookup should succeed");
/// assert_eq!(out.values, vec![1.0, 2.0, 3.0]);
/// ```
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct EvalOpts {
    /// Optional pre-built execution plan to follow. If not provided, the
    /// evaluator will either use the internal plan (if present) or fallback to
    /// a minimal evaluation path for the expression.
    pub(crate) plan: Option<ExecutionPlan>,
    /// Optional cache budget in megabytes. When provided, a cache will be
    /// instantiated (and sized for the plan when available) and cache stats
    /// will be embedded in the returned metadata.
    pub cache_budget_mb: Option<usize>,
    /// Maximum arena allocation in bytes. Defaults to 1 GB.
    /// Set to 0 to disable the check.
    #[serde(default = "default_max_arena_bytes")]
    pub max_arena_bytes: usize,
}

fn default_max_arena_bytes() -> usize {
    1_073_741_824
}

impl Default for EvalOpts {
    fn default() -> Self {
        Self {
            plan: None,
            cache_budget_mb: None,
            max_arena_bytes: default_max_arena_bytes(),
        }
    }
}

impl EvalOpts {
    /// Return whether an explicit execution plan is attached.
    pub fn has_plan(&self) -> bool {
        self.plan.is_some()
    }
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
/// `CompiledExpr` is both `Send` and `Sync`. Internal scratch buffers and
/// caches are protected by `Mutex`. For parallel evaluation, either share a
/// single instance (concurrent `eval()` calls will serialize on the scratch
/// `Mutex`) or clone for independent scratch buffers per thread.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct CompiledExpr {
    /// Underlying expression AST.
    pub ast: Expr,
    /// Optional execution plan for complex expressions.
    pub(crate) plan: Option<ExecutionPlan>,
    /// Cache manager for intermediate results.
    #[serde(skip)]
    pub(crate) cache: Option<CacheManager>,
    /// Small scratch arena to reuse temporary buffers within hot paths.
    #[serde(skip, default = "default_scratch")]
    pub(super) scratch: Mutex<ScratchArena>,
    /// Lazily-built fallback plan, populated on first `eval()` when `plan` is None.
    /// Prevents rebuilding the DAG on every call for expressions created via `new()`.
    #[serde(skip)]
    lazy_plan: OnceLock<ExecutionPlan>,
}

fn default_scratch() -> Mutex<ScratchArena> {
    Mutex::new(ScratchArena::default())
}

/// Tiny reusable scratch buffers for hot evaluation paths.
#[derive(Default, Debug)]
pub(super) struct ScratchArena {
    /// Generic temporary buffer for algorithms (e.g., median, sorts).
    pub(super) tmp: Vec<f64>,
    /// Window buffer for rolling operations that need a writable copy.
    pub(super) window: Vec<f64>,
}

impl Clone for CompiledExpr {
    fn clone(&self) -> Self {
        Self {
            ast: self.ast.clone(),
            plan: self.plan.clone(),
            cache: self.cache.clone(),
            // Fresh scratch and lazy_plan for clones; per-instance reuse only.
            scratch: Mutex::new(ScratchArena::default()),
            lazy_plan: OnceLock::new(),
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
            lazy_plan: OnceLock::new(),
        }
    }

    /// Construct with DAG planning enabled.
    pub fn with_planning(ast: Expr, meta: crate::config::ResultsMeta) -> crate::Result<Self> {
        let mut builder = DagBuilder::new();
        let plan = builder.build_plan(vec![ast.clone()], meta)?;
        let cache = CacheManager::for_plan(&plan, 100); // 100MB default

        Ok(Self {
            ast,
            plan: Some(plan),
            cache: Some(cache),
            scratch: Mutex::new(ScratchArena::default()),
            lazy_plan: OnceLock::new(),
        })
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

    /// Return whether this compiled expression currently has an attached cache.
    pub fn has_cache(&self) -> bool {
        self.cache.is_some()
    }

    /// Return whether this compiled expression has a pre-built execution plan.
    pub fn has_plan(&self) -> bool {
        self.plan.is_some()
    }

    /// Unified evaluation entrypoint returning values with execution metadata.
    ///
    /// Uses scalar implementations for all functions, with optional DAG planning
    /// and caching for complex expressions.
    pub fn eval(
        &self,
        ctx: &SimpleContext,
        cols: &[&[f64]],
        opts: EvalOpts,
    ) -> crate::Result<EvaluationResult> {
        // Decide on execution plan preference: opts > self > lazy-cached auto-build.
        // Use references to avoid cloning ExecutionPlan (which contains Vec<DagNode>
        // with recursive Expr trees). Only build a new owned plan when none exists.
        let owned_plan;
        let plan_to_use: &ExecutionPlan = if let Some(ref plan) = opts.plan {
            plan
        } else if let Some(ref plan) = self.plan {
            plan
        } else if let Some(plan) = self.lazy_plan.get() {
            plan
        } else {
            let mut builder = DagBuilder::new();
            let meta = crate::config::results_meta(&crate::config::FinstackConfig::default());
            let plan = builder.build_plan(vec![self.ast.clone()], meta)?;
            // Try to cache for future calls; if a racing thread beat us, use theirs.
            match self.lazy_plan.set(plan) {
                Ok(()) => self
                    .lazy_plan
                    .get()
                    .ok_or(crate::Error::from(crate::InputError::Invalid))?,
                Err(plan) => {
                    // Race: another thread set it first. Use theirs (already cached).
                    // Keep our plan alive for this call as a fallback.
                    owned_plan = plan;
                    self.lazy_plan.get().unwrap_or(&owned_plan)
                }
            }
        };

        // Decide on cache to use for this evaluation
        let eval_cache: Option<CacheManager> = if let Some(budget) = opts.cache_budget_mb {
            Some(CacheManager::for_plan(plan_to_use, budget))
        } else {
            self.cache.clone()
        };

        tracing::debug!(
            row_count = cols.first().map(|c| c.len()).unwrap_or(0),
            plan_nodes = plan_to_use.nodes.len(),
            cache_enabled = eval_cache.is_some(),
            "evaluating compiled expression"
        );

        // Compute values using the chosen strategy
        let values: Vec<f64> = {
            // Execute nodes in topological order using arena allocation
            let len = cols.first().map(|c| c.len()).unwrap_or(0);
            let node_count = plan_to_use.nodes.len();
            let arena_elements = len.checked_mul(node_count).ok_or_else(|| {
                crate::Error::from(crate::InputError::TooLarge {
                    what: "expression arena".into(),
                    requested_bytes: usize::MAX,
                    limit_bytes: opts.max_arena_bytes,
                })
            })?;
            let arena_bytes = arena_elements.saturating_mul(std::mem::size_of::<f64>());
            if opts.max_arena_bytes > 0 && arena_bytes > opts.max_arena_bytes {
                return Err(crate::InputError::TooLarge {
                    what: "expression arena".into(),
                    requested_bytes: arena_bytes,
                    limit_bytes: opts.max_arena_bytes,
                }
                .into());
            }

            // Pre-allocate arena for all node results to avoid per-node Vec allocations
            let mut arena = vec![0.0; arena_elements];
            let mut offsets: HashMap<u64, (usize, usize)> = HashMap::default();
            let mut cursor = 0;

            for node in &plan_to_use.nodes {
                // Cache lookup
                if let Some(ref cache) = eval_cache {
                    if let Some(cached) = cache.get(node.id, len) {
                        let scalar_result = cached.as_scalar_slice();
                        // Copy cached result into arena without materializing an
                        // intermediate Vec on every cache hit.
                        debug_assert_eq!(scalar_result.len(), len);
                        let start = cursor;
                        let end = cursor + len;
                        arena[start..end].copy_from_slice(&scalar_result[..len]);
                        offsets.insert(node.id, (start, end));
                        cursor = end;
                        continue;
                    }
                }

                // Allocate space in arena for this node's result
                let start = cursor;
                let end = cursor + len;

                // Evaluate node directly into arena slice
                // Split the arena to avoid borrow conflicts
                let (arena_deps, arena_out) = arena.split_at_mut(start);
                let out_slice = &mut arena_out[..len];
                self.eval_node_into(ctx, cols, node, arena_deps, &offsets, out_slice)?;

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
            plan_to_use
                .roots
                .first()
                .and_then(|&root_id| offsets.get(&root_id))
                .map(|&(start, end)| arena[start..end].to_vec())
                .unwrap_or_default()
        };

        // Stamp minimal metadata only at IO boundaries; evaluator does not record timings/cache/parallel
        let meta = crate::config::results_meta(&crate::config::FinstackConfig::default());

        Ok(EvaluationResult {
            values,
            metadata: meta,
        })
    }

    /// Evaluate a single DAG node directly into a provided output slice (arena-based).
    fn eval_node_into(
        &self,
        ctx: &SimpleContext,
        cols: &[&[f64]],
        node: &super::dag::DagNode,
        arena: &[f64],
        offsets: &HashMap<u64, (usize, usize)>,
        out: &mut [f64],
    ) -> crate::Result<()> {
        match &node.expr.node {
            ExprNode::Column(name) => {
                let Some(idx) = ctx.index_of(name) else {
                    return Err(crate::error::InputError::NotFound {
                        id: format!("expr column:{name}"),
                    }
                    .into());
                };
                let Some(col_data) = cols.get(idx) else {
                    return Err(crate::Error::Validation(format!(
                        "Expression context resolved column '{name}' to index {idx}, but only {} data columns were provided",
                        cols.len()
                    )));
                };
                let len = out.len().min(col_data.len());
                out[..len].copy_from_slice(&col_data[..len]);
                out[len..].fill(f64::NAN);
            }
            ExprNode::CSRef { .. } => {
                return Err(crate::Error::Validation(
                    "capital-structure references require the statements evaluator".to_string(),
                ));
            }
            ExprNode::Literal(val) => {
                out.fill(*val);
            }
            ExprNode::Call(func, _args) => {
                // Get argument results from dependencies (slices from arena)
                let arg_slices: SmallVec<[&[f64]; 4]> = node
                    .dependencies
                    .iter()
                    .filter_map(|&dep_id| {
                        offsets.get(&dep_id).map(|&(start, end)| &arena[start..end])
                    })
                    .collect();

                if arg_slices.len() != node.dependencies.len() {
                    return Err(crate::Error::Validation(format!(
                        "Expression DAG node {} is missing {} dependency results",
                        node.id,
                        node.dependencies.len() - arg_slices.len()
                    )));
                }
                self.eval_function_into(*func, &arg_slices, ctx, cols, out)?;
            }
            ExprNode::BinOp { op, .. } => {
                // Binary operations should have exactly 2 dependencies
                if node.dependencies.len() < 2 {
                    return Err(crate::Error::Validation(format!(
                        "Binary expression node {} is missing operands",
                        node.id
                    )));
                }
                let left = offsets
                    .get(&node.dependencies[0])
                    .map(|&(start, end)| &arena[start..end])
                    .ok_or_else(|| {
                        crate::Error::Validation(format!(
                            "Binary expression node {} is missing its left dependency result",
                            node.id
                        ))
                    })?;
                let right = offsets
                    .get(&node.dependencies[1])
                    .map(|&(start, end)| &arena[start..end])
                    .ok_or_else(|| {
                        crate::Error::Validation(format!(
                            "Binary expression node {} is missing its right dependency result",
                            node.id
                        ))
                    })?;
                Self::eval_bin_op_into(*op, left, right, out);
            }
            ExprNode::UnaryOp { op, .. } => {
                // Unary operations should have exactly 1 dependency
                if node.dependencies.is_empty() {
                    return Err(crate::Error::Validation(format!(
                        "Unary expression node {} is missing its operand",
                        node.id
                    )));
                }
                let operand = offsets
                    .get(&node.dependencies[0])
                    .map(|&(start, end)| &arena[start..end])
                    .ok_or_else(|| {
                        crate::Error::Validation(format!(
                            "Unary expression node {} is missing its operand result",
                            node.id
                        ))
                    })?;
                Self::eval_unary_op_into(*op, operand, out);
            }
            ExprNode::IfThenElse { .. } => {
                // If-then-else should have exactly 3 dependencies
                if node.dependencies.len() < 3 {
                    return Err(crate::Error::Validation(format!(
                        "If-then-else expression node {} is missing one or more branch dependencies",
                        node.id
                    )));
                }
                let condition = offsets
                    .get(&node.dependencies[0])
                    .map(|&(start, end)| &arena[start..end])
                    .ok_or_else(|| {
                        crate::Error::Validation(format!(
                            "If-then-else node {} is missing its condition result",
                            node.id
                        ))
                    })?;
                let then_vals = offsets
                    .get(&node.dependencies[1])
                    .map(|&(start, end)| &arena[start..end])
                    .ok_or_else(|| {
                        crate::Error::Validation(format!(
                            "If-then-else node {} is missing its then-branch result",
                            node.id
                        ))
                    })?;
                let else_vals = offsets
                    .get(&node.dependencies[2])
                    .map(|&(start, end)| &arena[start..end])
                    .ok_or_else(|| {
                        crate::Error::Validation(format!(
                            "If-then-else node {} is missing its else-branch result",
                            node.id
                        ))
                    })?;
                Self::eval_if_then_else_into(condition, then_vals, else_vals, out);
            }
        }
        Ok(())
    }

    /// Evaluate a binary operation element-wise into a provided output slice.
    #[inline]
    fn eval_bin_op_into(op: super::ast::BinOp, left: &[f64], right: &[f64], out: &mut [f64]) {
        use super::ast::BinOp;
        let len = out.len();

        for (i, out_val) in out.iter_mut().enumerate().take(len) {
            let (Some(&l), Some(&r)) = (left.get(i), right.get(i)) else {
                *out_val = f64::NAN;
                continue;
            };

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
                // Exact equality semantics for expression-language operators.
                #[allow(clippy::float_cmp)]
                BinOp::Eq => {
                    if l == r {
                        1.0
                    } else {
                        0.0
                    }
                }
                #[allow(clippy::float_cmp)]
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
        out[len..].fill(f64::NAN);
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
            let (Some(&cond), Some(&then_val), Some(&else_val)) =
                (condition.get(i), then_vals.get(i), else_vals.get(i))
            else {
                *out_val = f64::NAN;
                continue;
            };
            *out_val = if cond != 0.0 { then_val } else { else_val };
        }
    }

    /// Evaluate a function with given argument results (slices from arena).
    fn eval_function_into(
        &self,
        fun: Function,
        arg_slices: &[&[f64]],
        _ctx: &SimpleContext,
        _cols: &[&[f64]],
        out: &mut [f64],
    ) -> crate::Result<()> {
        let result = self.eval_function_core(fun, arg_slices, _ctx, _cols)?;
        let copy_len = out.len().min(result.len());
        out[..copy_len].copy_from_slice(&result[..copy_len]);
        if copy_len < out.len() {
            out[copy_len..].fill(f64::NAN);
        }
        Ok(())
    }
}
#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing
)]
mod tests {
    use super::*;
    use crate::config::FinstackConfig;
    use crate::expr::{BinOp, Expr, Function, SimpleContext, UnaryOp};

    fn sample_context() -> (SimpleContext, Vec<Vec<f64>>) {
        let ctx = SimpleContext::new(["x", "y"]).expect("unique columns");
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
        let result = compiled
            .eval(&ctx, &cols, EvalOpts::default())
            .unwrap()
            .values;

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
        let compiled = CompiledExpr::with_planning(expr, meta)
            .unwrap()
            .with_cache(1);

        let result = compiled
            .eval(
                &ctx,
                &cols,
                EvalOpts {
                    plan: None,
                    cache_budget_mb: Some(1),
                    max_arena_bytes: default_max_arena_bytes(),
                },
            )
            .unwrap()
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
        let compiled = CompiledExpr::with_planning(expr, meta).unwrap();
        let external_plan = compiled.plan.clone();

        let result = compiled
            .eval(
                &ctx,
                &cols,
                EvalOpts {
                    plan: external_plan,
                    cache_budget_mb: None,
                    max_arena_bytes: default_max_arena_bytes(),
                },
            )
            .unwrap()
            .values;

        assert!(result[0].is_nan());
        assert!((result[1] - 0.3).abs() < 1e-12);
        assert!((result[2] - 2.5).abs() < 1e-12);
        assert!((result[3] - 1.0).abs() < 1e-12);
    }

    #[test]
    fn arena_rejects_oversized_allocation() {
        let ast = Expr::bin_op(BinOp::Add, Expr::column("x"), Expr::column("y"));
        let expr = CompiledExpr::new(ast);

        let col: Vec<f64> = vec![1.0; 1000];
        let cols: Vec<&[f64]> = vec![&col, &col];
        let ctx = SimpleContext::new(["x", "y"]).expect("unique columns");

        let opts = EvalOpts {
            max_arena_bytes: 100,
            ..EvalOpts::default()
        };
        let result = expr.eval(&ctx, &cols, opts);
        assert!(result.is_err());
        let err_str = result.unwrap_err().to_string();
        assert!(
            err_str.contains("too large") || err_str.contains("TooLarge"),
            "Expected TooLarge error, got: {err_str}"
        );
    }

    #[test]
    fn arena_accepts_normal_allocation() {
        let ast = Expr::column("x");
        let expr = CompiledExpr::new(ast);
        let col = vec![1.0, 2.0, 3.0];
        let cols: Vec<&[f64]> = vec![&col];
        let ctx = SimpleContext::new(["x"]).expect("unique columns");
        let opts = EvalOpts::default();
        let result = expr.eval(&ctx, &cols, opts);
        assert!(result.is_ok());
    }

    #[test]
    fn arena_check_disabled_when_zero() {
        let ast = Expr::column("x");
        let expr = CompiledExpr::new(ast);
        let col = vec![1.0, 2.0, 3.0];
        let cols: Vec<&[f64]> = vec![&col];
        let ctx = SimpleContext::new(["x"]).expect("unique columns");
        let opts = EvalOpts {
            max_arena_bytes: 0,
            ..EvalOpts::default()
        };
        let result = expr.eval(&ctx, &cols, opts);
        assert!(result.is_ok());
    }
}
