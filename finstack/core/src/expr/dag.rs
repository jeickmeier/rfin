//! DAG (Directed Acyclic Graph) planning for expression execution.
//!
//! This module provides shared sub-expression detection, execution planning,
//! and optimization for complex expression trees. It builds an execution
//! DAG that minimizes recomputation and maximizes cache hits.

use super::ast::*;
use std::collections::{HashMap, HashSet};
use std::vec::Vec;

/// A node in the execution DAG.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DagNode {
    /// Unique identifier for this node.
    pub id: u64,
    /// The expression this node represents.
    pub expr: Expr,
    /// Dependencies (other DAG nodes this depends on).
    pub dependencies: Vec<u64>,
    /// Reference count (how many other nodes depend on this).
    pub ref_count: usize,
    /// Whether this node can be executed in Polars.
    pub polars_eligible: bool,
    /// Estimated cost of computing this node.
    pub cost: usize,
}

/// Execution plan for a DAG of expressions.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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

        // Determine Polars eligibility in topological order so that
        // dependency flags are available when evaluating parents.
        self.determine_polars_eligibility_topo(&ordered_nodes);

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
        };

        // Estimate cost
        let cost = self.estimate_cost(&expr);

        // Create DAG node
        let node = DagNode {
            id,
            expr: expr.clone(),
            dependencies,
            ref_count: 0,           // Will be calculated later
            polars_eligible: false, // Will be determined later
            cost,
        };

        // Store node and cache expression
        self.nodes.insert(id, node);
        self.expr_cache.insert(expr, id);

        id
    }

    /// Calculate reference counts for all nodes.
    fn calculate_ref_counts(&mut self, root_ids: &[u64]) {
        let mut ref_counts: HashMap<u64, usize> = HashMap::new();
        let mut visited = HashSet::new();

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

    /// Determine which nodes are eligible for Polars execution using a
    /// topological sweep so dependencies are processed first.
    fn determine_polars_eligibility_topo(&mut self, ordered_nodes: &[DagNode]) {
        // Local map to avoid borrowing issues while mutating self.nodes
        let mut elig: HashMap<u64, bool> = HashMap::with_capacity(self.nodes.len());
        for n in ordered_nodes {
            let is_eligible = match &n.expr.node {
                ExprNode::Column(_) | ExprNode::Literal(_) => true,
                ExprNode::Call(func, _args) => {
                    let func_ok = self.function_supports_polars(*func);
                    let deps_ok = n
                        .dependencies
                        .iter()
                        .all(|d| elig.get(d).copied().unwrap_or(false));
                    func_ok && deps_ok
                }
            };
            elig.insert(n.id, is_eligible);
            if let Some(node_mut) = self.nodes.get_mut(&n.id) {
                node_mut.polars_eligible = is_eligible;
            }
        }
    }

    /// Check if a function supports Polars lowering.
    fn function_supports_polars(&self, func: Function) -> bool {
        match func {
            Function::Lag | Function::Lead | Function::Diff | Function::PctChange => true,
            Function::RollingMean | Function::RollingSum => true,

            // Cumulative functions use scalar implementation for determinism
            Function::CumSum | Function::CumProd | Function::CumMin | Function::CumMax => false,
            // Statistical functions now support Polars lowering
            Function::Std | Function::Var | Function::Median => true,
            // Rolling statistical functions use scalar fallback for now
            Function::RollingStd | Function::RollingVar | Function::RollingMedian => false,
            // Complex EWM functions still use scalar fallback
            Function::EwmMean => false,

            // New functions
            Function::Shift => true,
            // Keep rolling min/max/count as scalar/unsupported until mapped
            Function::RollingMin | Function::RollingMax => false,
            Function::Rank
            | Function::Quantile
            | Function::RollingCount
            | Function::EwmStd
            | Function::EwmVar => false,
        }
    }

    /// Estimate the computational cost of an expression.
    fn estimate_cost(&self, expr: &Expr) -> usize {
        match &expr.node {
            ExprNode::Column(_) => 1,
            ExprNode::Literal(_) => 1,
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
                };
                base_cost + args.len() * 5
            }
        }
    }

    /// Build topological ordering of nodes.
    fn topological_sort(&self, root_ids: &[u64]) -> Vec<DagNode> {
        let mut visited = HashSet::new();
        let mut result = Vec::new();
        let mut visiting = HashSet::new();

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
                panic!("Cycle detected in expression DAG at node {}", node_id);
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
        let mut cache_nodes = HashSet::new();
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

/// Pushdown boundary detection - determines optimal Polars/scalar split points.
pub struct PushdownAnalyzer;

impl PushdownAnalyzer {
    /// Analyze an execution plan and determine optimal pushdown boundaries.
    pub fn analyze_boundaries(plan: &ExecutionPlan) -> PushdownBoundaries {
        let polars_subtrees = Vec::new();
        let mut scalar_nodes = HashSet::new();
        let mut boundaries = Vec::new();

        // Find contiguous Polars-eligible subtrees
        for node in &plan.nodes {
            if !node.polars_eligible {
                scalar_nodes.insert(node.id);
            }
        }

        // Identify boundary points where we switch from Polars to scalar
        for node in &plan.nodes {
            if node.polars_eligible {
                // Check if any dependency requires scalar execution
                let has_scalar_deps = node
                    .dependencies
                    .iter()
                    .any(|&dep_id| scalar_nodes.contains(&dep_id));

                if has_scalar_deps {
                    boundaries.push(PushdownBoundary {
                        node_id: node.id,
                        boundary_type: BoundaryType::PolarsTScalar,
                        materialization_required: true,
                    });
                }
            }
        }

        let speedup = Self::estimate_speedup(&plan.nodes, &boundaries);
        PushdownBoundaries {
            boundaries,
            polars_subtrees,
            estimated_speedup: speedup,
        }
    }

    fn estimate_speedup(nodes: &[DagNode], boundaries: &[PushdownBoundary]) -> f64 {
        let total_cost: usize = nodes.iter().map(|n| n.cost).sum();
        let polars_cost: usize = nodes
            .iter()
            .filter(|n| n.polars_eligible)
            .map(|n| n.cost / 3) // Assume Polars is ~3x faster
            .sum();
        let scalar_cost: usize = nodes
            .iter()
            .filter(|n| !n.polars_eligible)
            .map(|n| n.cost)
            .sum();

        let boundary_cost: usize = boundaries.len() * 10; // Materialization overhead

        if total_cost > 0 {
            (total_cost as f64) / ((polars_cost + scalar_cost + boundary_cost) as f64)
        } else {
            1.0
        }
    }
}

/// Analysis of pushdown boundaries in an execution plan.
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PushdownBoundaries {
    /// Specific boundary points.
    pub boundaries: Vec<PushdownBoundary>,
    /// Polars-eligible subtrees.
    pub polars_subtrees: Vec<Vec<u64>>,
    /// Estimated speedup from pushdown.
    pub estimated_speedup: f64,
}

/// A specific boundary point in the execution.
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PushdownBoundary {
    /// Node ID where boundary occurs.
    pub node_id: u64,
    /// Type of boundary.
    pub boundary_type: BoundaryType,
    /// Whether materialization is required at this boundary.
    pub materialization_required: bool,
}

/// Types of pushdown boundaries.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BoundaryType {
    /// Transition from Polars-eligible to scalar-only.
    PolarsTScalar,
    /// Transition from scalar back to Polars-eligible.
    ScalarToPolars,
}
