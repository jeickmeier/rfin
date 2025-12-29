//! DAG planning and execution optimization for expressions.
//!
//! Detects shared sub-expressions in complex expression trees and builds an
//! optimized execution plan that evaluates each unique sub-expression only
//! once. Critical for performance in financial statement models with hundreds
//! of interdependent formulas.

use super::ast::*;
use crate::collections::{HashMap, HashSet};
use std::vec::Vec;

/// A node in the execution DAG.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DagNode {
    /// Unique identifier for this node.
    pub id: u64,
    /// The expression this node represents.
    pub expr: Expr,
    /// Dependencies (other DAG nodes this depends on).
    pub dependencies: Vec<u64>,
    /// Reference count (how many other nodes depend on this).
    pub ref_count: usize,
    /// Estimated cost of computing this node.
    pub cost: usize,
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use crate::config::ResultsMeta;

    fn meta() -> ResultsMeta {
        crate::config::results_meta(&crate::config::FinstackConfig::default())
    }

    #[test]
    fn dag_builder_deduplicates_structurally_identical_nodes() {
        let mut builder = DagBuilder::new();
        let a = Expr::call(
            Function::RollingMean,
            vec![Expr::column("x"), Expr::literal(3.0)],
        )
        .with_id(7);
        let b = Expr::call(
            Function::RollingMean,
            vec![Expr::column("x"), Expr::literal(3.0)],
        )
        .with_id(42);

        let plan = builder.build_plan(vec![a, b], meta());
        assert_eq!(plan.nodes.len(), 3, "column, literal, rolling mean");
        assert_eq!(plan.roots.len(), 2);
        assert_eq!(plan.roots[0], plan.roots[1]);
    }

    #[test]
    fn dag_builder_orders_dependencies_topologically() {
        let mut builder = DagBuilder::new();
        let col = Expr::column("x");
        let lit = Expr::literal(2.0);
        let sum = Expr::call(Function::RollingSum, vec![col.clone(), lit.clone()]);
        let mean = Expr::call(Function::RollingMean, vec![col, lit]);

        let plan = builder.build_plan(vec![sum, mean], meta());
        assert_eq!(plan.nodes.len(), 4);

        // Ensure dependencies come before dependents in node order.
        for node in &plan.nodes {
            for &dep in &node.dependencies {
                let dep_index = plan
                    .nodes
                    .iter()
                    .position(|n| n.id == dep)
                    .expect("dependency must exist");
                let node_index = plan
                    .nodes
                    .iter()
                    .position(|n| n.id == node.id)
                    .expect("node must exist in plan");
                assert!(dep_index < node_index);
            }
        }
    }
}

/// Execution plan for a DAG of expressions.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExecutionPlan {
    /// All nodes in topological order.
    pub nodes: Vec<DagNode>,
    /// Root node IDs (final outputs).
    pub roots: Vec<u64>,
    /// Execution metadata.
    pub meta: crate::config::ResultsMeta,
    /// Cache strategy recommendations.
    pub cache_strategy: CacheStrategy,
}

/// Cache strategy for the execution plan.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CacheStrategy {
    /// Nodes that should be cached (high ref count or expensive).
    pub cache_nodes: HashSet<u64>,
    /// Expected cache hit rate.
    pub expected_hit_rate: f64,
    /// Memory budget estimate (arbitrary units).
    pub memory_budget: usize,
}

/// DAG builder that detects shared sub-expressions and builds optimized execution plans.
#[derive(Default)]
pub struct DagBuilder {
    /// Expression cache for deduplication.
    expr_cache: HashMap<Expr, u64>,
    /// Node storage.
    nodes: HashMap<u64, DagNode>,
    /// Next available node ID.
    next_id: u64,
}

impl DagBuilder {
    /// Create a new DAG builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Build an execution plan from a list of root expressions.
    pub fn build_plan(
        &mut self,
        exprs: Vec<Expr>,
        meta: crate::config::ResultsMeta,
    ) -> ExecutionPlan {
        // Clear state
        self.expr_cache.clear();
        self.nodes.clear();
        self.next_id = 0;

        // Process each root expression
        let mut root_ids = Vec::new();
        for expr in exprs {
            let id = self.process_expression(expr);
            root_ids.push(id);
        }

        // Calculate reference counts
        self.calculate_ref_counts(&root_ids);

        // Build topological order (dependencies first)
        let ordered_nodes = self.topological_sort(&root_ids);

        // Generate cache strategy
        let cache_strategy = self.generate_cache_strategy(&ordered_nodes);

        ExecutionPlan {
            nodes: ordered_nodes,
            roots: root_ids,
            meta,
            cache_strategy,
        }
    }

    /// Process an expression tree, deduplicating shared sub-expressions.
    fn process_expression(&mut self, expr: Expr) -> u64 {
        // Check if we've already seen this expression
        if let Some(&existing_id) = self.expr_cache.get(&expr) {
            return existing_id;
        }

        // Generate new ID and process dependencies
        let id = self.next_id;
        self.next_id += 1;

        let dependencies = match &expr.node {
            ExprNode::Column(_) | ExprNode::Literal(_) => Vec::new(),
            ExprNode::Call(_, args) => args
                .iter()
                .map(|arg| self.process_expression(arg.clone()))
                .collect(),
            ExprNode::BinOp { left, right, .. } => {
                vec![
                    self.process_expression((**left).clone()),
                    self.process_expression((**right).clone()),
                ]
            }
            ExprNode::UnaryOp { operand, .. } => {
                vec![self.process_expression((**operand).clone())]
            }
            ExprNode::IfThenElse {
                condition,
                then_expr,
                else_expr,
            } => {
                vec![
                    self.process_expression((**condition).clone()),
                    self.process_expression((**then_expr).clone()),
                    self.process_expression((**else_expr).clone()),
                ]
            }
        };

        // Estimate cost
        let cost = self.estimate_cost(&expr);

        // Create DAG node
        let node = DagNode {
            id,
            expr: expr.clone(),
            dependencies,
            ref_count: 0, // Will be calculated later
            cost,
        };

        // Store node and cache expression
        self.nodes.insert(id, node);
        self.expr_cache.insert(expr, id);

        id
    }

    /// Calculate reference counts for all nodes.
    fn calculate_ref_counts(&mut self, root_ids: &[u64]) {
        let mut ref_counts: HashMap<u64, usize> = HashMap::default();
        let mut visited = HashSet::default();

        fn count_refs(
            node_id: u64,
            nodes: &HashMap<u64, DagNode>,
            ref_counts: &mut HashMap<u64, usize>,
            visited: &mut HashSet<u64>,
        ) {
            if visited.contains(&node_id) {
                return;
            }
            visited.insert(node_id);

            if let Some(node) = nodes.get(&node_id) {
                for &dep_id in &node.dependencies {
                    *ref_counts.entry(dep_id).or_insert(0) += 1;
                    count_refs(dep_id, nodes, ref_counts, visited);
                }
            }
        }

        for &root_id in root_ids {
            count_refs(root_id, &self.nodes, &mut ref_counts, &mut visited);
        }

        // Update nodes with reference counts
        for (id, count) in ref_counts {
            if let Some(node) = self.nodes.get_mut(&id) {
                node.ref_count = count;
            }
        }
    }

    /// Estimate the computational cost of an expression.
    fn estimate_cost(&self, expr: &Expr) -> usize {
        match &expr.node {
            ExprNode::Column(_) => 1,
            ExprNode::Literal(_) => 1,
            ExprNode::BinOp { .. } => 2, // Basic arithmetic/comparison/logical operations
            ExprNode::UnaryOp { .. } => 2,
            ExprNode::IfThenElse { .. } => 3, // Conditional evaluation
            ExprNode::Call(func, args) => {
                let base_cost = match func {
                    Function::Lag | Function::Lead => 5,
                    Function::Diff | Function::PctChange => 10,
                    Function::CumSum | Function::CumProd | Function::CumMin | Function::CumMax => {
                        20
                    }
                    Function::RollingMean | Function::RollingSum => 30,
                    Function::RollingStd | Function::RollingVar | Function::RollingMedian => 50,
                    Function::EwmMean => 25,
                    Function::Std | Function::Var => 40,
                    Function::Median => 60,

                    // New functions
                    Function::Shift => 5,
                    Function::Rank => 80,
                    Function::Quantile => 90,
                    Function::RollingMin | Function::RollingMax => 30,
                    Function::RollingCount => 20,
                    Function::EwmStd | Function::EwmVar => 45,
                    // Custom financial functions
                    Function::Sum | Function::Mean => 5,
                    Function::Annualize => 2,
                    Function::AnnualizeRate => 3, // Slightly more expensive due to powf
                    Function::Ttm | Function::Ytd | Function::Qtd | Function::FiscalYtd => 30, // Similar cost to rolling functions
                    Function::Coalesce => 3,
                    Function::Abs | Function::Sign => 2,
                    Function::GrowthRate => 35,
                };
                base_cost + args.len() * 5
            }
        }
    }

    /// Build topological ordering of nodes.
    fn topological_sort(&self, root_ids: &[u64]) -> Vec<DagNode> {
        let mut visited = HashSet::default();
        let mut result = Vec::new();
        let mut visiting = HashSet::default();

        fn visit(
            node_id: u64,
            nodes: &HashMap<u64, DagNode>,
            visited: &mut HashSet<u64>,
            visiting: &mut HashSet<u64>,
            result: &mut Vec<DagNode>,
        ) {
            if visited.contains(&node_id) {
                return;
            }
            if visiting.contains(&node_id) {
                // Cycle detected - shouldn't happen in expression DAGs
                // Debug builds panic to catch programming errors early
                debug_assert!(
                    false,
                    "Cycle detected in expression DAG at node {}",
                    node_id
                );
                // Release builds: silently skip the cycle (safe but may produce incomplete results)
                return;
            }

            visiting.insert(node_id);

            if let Some(node) = nodes.get(&node_id) {
                for &dep_id in &node.dependencies {
                    visit(dep_id, nodes, visited, visiting, result);
                }
                result.push(node.clone());
            }

            visiting.remove(&node_id);
            visited.insert(node_id);
        }

        for &root_id in root_ids {
            visit(
                root_id,
                &self.nodes,
                &mut visited,
                &mut visiting,
                &mut result,
            );
        }

        result
    }

    /// Generate cache strategy based on node characteristics.
    fn generate_cache_strategy(&self, nodes: &[DagNode]) -> CacheStrategy {
        let mut cache_nodes = HashSet::default();
        let mut total_cost = 0;
        let mut cacheable_cost = 0;

        for node in nodes {
            total_cost += node.cost;

            // Cache nodes with high reference count or high cost
            let should_cache = node.ref_count > 1 && (node.cost > 30 || node.ref_count > 2);

            if should_cache {
                cache_nodes.insert(node.id);
                cacheable_cost += node.cost * (node.ref_count - 1);
            }
        }

        let expected_hit_rate = if total_cost > 0 {
            cacheable_cost as f64 / total_cost as f64
        } else {
            0.0
        };

        CacheStrategy {
            cache_nodes,
            expected_hit_rate,
            memory_budget: nodes.len() * 100, // Rough estimate
        }
    }
}

/// Execution analysis for optimization hints.
///
/// Note: With Polars removed, this provides basic execution statistics
/// without vectorization-specific boundary analysis.
pub struct PushdownAnalyzer;

impl PushdownAnalyzer {
    /// Analyze an execution plan and provide basic statistics.
    ///
    /// Returns a simplified analysis structure with execution cost estimates.
    pub fn analyze_boundaries(_plan: &ExecutionPlan) -> PushdownBoundaries {
        let optimized_subtrees = Vec::new();
        let boundaries = Vec::new();

        // All nodes use scalar execution
        let speedup = 1.0; // Scalar execution baseline

        PushdownBoundaries {
            boundaries,
            optimized_subtrees,
            estimated_speedup: speedup,
        }
    }
}

/// Analysis of pushdown boundaries in an execution plan.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct PushdownBoundaries {
    /// Specific boundary points.
    pub boundaries: Vec<PushdownBoundary>,
    /// Optimized execution subtrees.
    pub optimized_subtrees: Vec<Vec<u64>>,
    /// Estimated speedup from optimization.
    pub estimated_speedup: f64,
}

/// A specific boundary point in the execution.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct PushdownBoundary {
    /// Node ID where boundary occurs.
    pub node_id: u64,
    /// Type of boundary.
    pub boundary_type: BoundaryType,
    /// Whether materialization is required at this boundary.
    pub materialization_required: bool,
}

/// Types of execution boundaries.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum BoundaryType {
    /// Transition point between execution strategies.
    OptimizedToScalar,
    /// Transition from scalar back to optimized execution.
    ScalarToOptimized,
}
