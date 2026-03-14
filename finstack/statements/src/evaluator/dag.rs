//! Dependency graph construction and topological sorting.

use crate::error::{Error, Result};
use crate::types::FinancialModelSpec;
use indexmap::{IndexMap, IndexSet};

/// Dependency graph for nodes in a financial model.
///
/// The graph stores both incoming and outgoing edges so that consumers can
/// traverse dependencies and dependents efficiently. It is primarily used by
/// the evaluator to derive a topological execution order and detect cycles.
#[derive(Debug)]
pub struct DependencyGraph {
    /// Map of node_id → set of dependencies (nodes it depends on)
    pub dependencies: IndexMap<String, IndexSet<String>>,

    /// Map of node_id → set of dependents (nodes that depend on it)
    pub dependents: IndexMap<String, IndexSet<String>>,
}

impl DependencyGraph {
    /// Build a dependency graph from a model specification.
    ///
    /// # Arguments
    /// * `model` - Fully configured [`FinancialModelSpec`](crate::types::FinancialModelSpec)
    ///
    /// # Example
    ///
    /// ```rust
    /// # use finstack_statements::builder::ModelBuilder;
    /// # use finstack_statements::evaluator::DependencyGraph;
    /// let model = ModelBuilder::new("demo")
    ///     .periods("2025Q1..Q2", None)?
    ///     .compute("a", "10")?
    ///     .compute("b", "a * 2")?
    ///     .build()?;
    ///
    /// let graph = DependencyGraph::from_model(&model)?;
    /// assert!(graph.dependencies["b"].contains("a"));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn from_model(model: &FinancialModelSpec) -> Result<Self> {
        // Validate all formula references before building graph
        Self::validate_formula_references(model)?;

        let mut dependencies = IndexMap::new();
        let mut dependents = IndexMap::new();

        // Initialize empty sets for all nodes
        for node_id in model.nodes.keys() {
            dependencies.insert(node_id.as_str().to_string(), IndexSet::new());
            dependents.insert(node_id.as_str().to_string(), IndexSet::new());
        }

        let all_node_ids: IndexSet<String> = model
            .nodes
            .keys()
            .map(|id| id.as_str().to_string())
            .collect();

        // Extract dependencies from formulas and where clauses
        for (node_id, node_spec) in &model.nodes {
            if let Some(formula) = &node_spec.formula_text {
                let node_deps = extract_dependencies(formula, &all_node_ids)?;
                add_dependency_edges(
                    node_id.as_str(),
                    &node_deps,
                    &mut dependencies,
                    &mut dependents,
                );
            }

            if let Some(where_clause) = &node_spec.where_text {
                let node_deps = extract_dependencies(where_clause, &all_node_ids)?;
                add_dependency_edges(
                    node_id.as_str(),
                    &node_deps,
                    &mut dependencies,
                    &mut dependents,
                );
            }
        }

        Ok(Self {
            dependencies,
            dependents,
        })
    }

    /// Validate that all identifier references in formulas exist in the model.
    ///
    /// This catches typos and unknown references at build time instead of runtime.
    fn validate_formula_references(model: &FinancialModelSpec) -> Result<()> {
        // Create set of all valid identifiers (all node IDs in the model)
        let valid_identifiers: IndexSet<String> = model
            .nodes
            .keys()
            .map(|id| id.as_str().to_string())
            .collect();

        // Check each formula
        for (node_id, node_spec) in &model.nodes {
            if let Some(formula) = &node_spec.formula_text {
                // Extract all identifiers from the formula
                let all_identifiers = crate::utils::formula::extract_all_identifiers(formula)?;

                // Check each identifier
                for identifier in &all_identifiers {
                    // Skip cs.* references (capital structure - validated at runtime)
                    if identifier.starts_with("cs.") {
                        continue;
                    }

                    // Check if identifier exists in model nodes
                    if !valid_identifiers.contains(identifier) {
                        return Err(Error::eval(format!(
                            "Unknown identifier '{}' in formula for node '{}'. \
                             Formula: '{}'. \
                             This identifier does not exist in the model. \
                             Did you mean one of: {}?",
                            identifier,
                            node_id,
                            formula,
                            suggest_similar_identifiers(identifier, &valid_identifiers)
                        )));
                    }
                }
            }

            // Also validate where clauses
            if let Some(where_clause) = &node_spec.where_text {
                let all_identifiers = crate::utils::formula::extract_all_identifiers(where_clause)?;

                for identifier in &all_identifiers {
                    if identifier.starts_with("cs.") {
                        continue;
                    }

                    if !valid_identifiers.contains(identifier) {
                        return Err(Error::eval(format!(
                            "Unknown identifier '{}' in where clause for node '{}'. \
                             Where clause: '{}'. \
                             This identifier does not exist in the model. \
                             Did you mean one of: {}?",
                            identifier,
                            node_id,
                            where_clause,
                            suggest_similar_identifiers(identifier, &valid_identifiers)
                        )));
                    }
                }
            }
        }

        Ok(())
    }

    /// Get dependencies for a node.
    ///
    /// # Arguments
    /// * `node_id` - Node identifier to inspect
    ///
    /// # Returns
    /// Either an [`IndexSet`] of upstream dependencies or `None` if the node
    /// does not exist.
    pub fn get_dependencies(&self, node_id: &str) -> Option<&IndexSet<String>> {
        self.dependencies.get(node_id)
    }

    /// Check for circular dependencies.
    ///
    /// Performs a depth-first search to surface a representative cycle when one
    /// exists.
    pub fn detect_cycles(&self) -> Result<()> {
        for node_id in self.dependencies.keys() {
            if let Some(cycle) = self.find_cycle_from(node_id) {
                return Err(Error::circular_dependency(cycle));
            }
        }
        Ok(())
    }

    /// Find a cycle starting from a given node (DFS).
    fn find_cycle_from(&self, start: &str) -> Option<Vec<String>> {
        let mut visited = IndexSet::new();
        let mut path = Vec::new();
        self.dfs_cycle(start, &mut visited, &mut path)
    }

    fn dfs_cycle(
        &self,
        node: &str,
        visited: &mut IndexSet<String>,
        path: &mut Vec<String>,
    ) -> Option<Vec<String>> {
        // If we've seen this node in the current path, we have a cycle
        if let Some(cycle_start) = path.iter().position(|n| n == node) {
            let mut cycle = path[cycle_start..].to_vec();
            cycle.push(node.to_string());
            return Some(cycle);
        }

        // If we've fully explored this node before, skip it
        if visited.contains(node) {
            return None;
        }

        path.push(node.to_string());

        if let Some(deps) = self.dependencies.get(node) {
            for dep in deps {
                if let Some(cycle) = self.dfs_cycle(dep, visited, path) {
                    return Some(cycle);
                }
            }
        }

        path.pop();
        visited.insert(node.to_string());
        None
    }
}

fn add_dependency_edges(
    node_id: &str,
    node_deps: &IndexSet<String>,
    dependencies: &mut IndexMap<String, IndexSet<String>>,
    dependents: &mut IndexMap<String, IndexSet<String>>,
) {
    for dep in node_deps {
        if let Some(deps) = dependencies.get_mut(node_id) {
            deps.insert(dep.clone());
        }
        if let Some(dep_set) = dependents.get_mut(dep) {
            dep_set.insert(node_id.to_string());
        }
    }
}

/// Compute the topological evaluation order.
///
/// Nodes are returned in an order where all dependencies appear before the
/// nodes that depend on them. The function returns an error if a cycle is
/// present.
///
/// # Arguments
/// * `graph` - Dependency graph built from a [`FinancialModelSpec`](crate::types::FinancialModelSpec)
///
/// # Example
///
/// ```rust
/// # use finstack_statements::builder::ModelBuilder;
/// # use finstack_statements::evaluator::{DependencyGraph, evaluate_order};
/// let model = ModelBuilder::new("demo")
///     .periods("2025Q1..Q2", None)?
///     .compute("a", "10")?
///     .compute("b", "a * 2")?
///     .build()?;
///
/// let graph = DependencyGraph::from_model(&model)?;
/// let order = evaluate_order(&graph)?;
/// assert!(order.iter().position(|n| n == "a") < order.iter().position(|n| n == "b"));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn evaluate_order(graph: &DependencyGraph) -> Result<Vec<String>> {
    crate::utils::graph::toposort_ids(&graph.dependencies).map_err(|unprocessed| {
        Error::eval(format!(
            "Circular dependency detected in model. Affected nodes: {}",
            unprocessed.join(", ")
        ))
    })
}

/// Extract node dependencies from a formula string.
///
/// Uses shared formula utilities to find standalone identifier references.
/// This specifically uses `extract_direct_dependencies` which parses the AST
/// and ignores references inside `lag()` and `shift()` calls, allowing for
/// temporal cycles (like corkscrews) without blocking the DAG.
fn extract_dependencies(
    formula: &str,
    all_node_ids: &IndexSet<String>,
) -> Result<IndexSet<String>> {
    let direct_deps = crate::utils::formula::extract_direct_dependencies(formula).map_err(|e| {
        crate::error::Error::build(format!(
            "Failed to parse formula for dependency extraction: {e}"
        ))
    })?;
    Ok(direct_deps.intersection(all_node_ids).cloned().collect())
}

/// Suggest similar identifiers for a typo using Levenshtein distance.
///
/// Returns a comma-separated list of up to 3 most similar identifiers.
fn suggest_similar_identifiers(typo: &str, valid: &IndexSet<String>) -> String {
    let mut similarities: Vec<(usize, &String)> = valid
        .iter()
        .map(|id| (levenshtein_distance(typo, id), id))
        .collect();

    // Sort by distance (closest first)
    similarities.sort_by_key(|(dist, _)| *dist);

    // Take top 3
    similarities
        .iter()
        .take(3)
        .map(|(_, id)| id.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

/// Calculate Levenshtein distance between two strings.
fn levenshtein_distance(s1: &str, s2: &str) -> usize {
    let len1 = s1.chars().count();
    let len2 = s2.chars().count();
    let mut matrix = vec![vec![0; len2 + 1]; len1 + 1];

    for (i, row) in matrix.iter_mut().enumerate().take(len1 + 1) {
        row[0] = i;
    }
    for (j, cell) in matrix[0].iter_mut().enumerate().take(len2 + 1) {
        *cell = j;
    }

    let s1_chars: Vec<char> = s1.chars().collect();
    let s2_chars: Vec<char> = s2.chars().collect();

    for (i, c1) in s1_chars.iter().enumerate() {
        for (j, c2) in s2_chars.iter().enumerate() {
            let cost = if c1 == c2 { 0 } else { 1 };
            matrix[i + 1][j + 1] = (matrix[i][j + 1] + 1) // deletion
                .min(matrix[i + 1][j] + 1) // insertion
                .min(matrix[i][j] + cost); // substitution
        }
    }

    matrix[len1][len2]
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::builder::ModelBuilder;

    #[test]
    fn test_simple_dag() {
        let model = ModelBuilder::new("test")
            .periods("2025Q1..Q2", None)
            .expect("test should succeed")
            .compute("a", "10")
            .expect("test should succeed")
            .compute("b", "a * 2")
            .expect("test should succeed")
            .compute("c", "b + a")
            .expect("test should succeed")
            .build()
            .expect("test should succeed");

        let graph = DependencyGraph::from_model(&model).expect("test should succeed");

        // Check dependencies
        assert_eq!(graph.dependencies["a"].len(), 0);
        assert!(graph.dependencies["b"].contains("a"));
        assert!(graph.dependencies["c"].contains("b"));
        assert!(graph.dependencies["c"].contains("a"));
    }

    #[test]
    fn test_topological_sort() {
        let model = ModelBuilder::new("test")
            .periods("2025Q1..Q2", None)
            .expect("test should succeed")
            .compute("a", "10")
            .expect("test should succeed")
            .compute("b", "a * 2")
            .expect("test should succeed")
            .compute("c", "b + a")
            .expect("test should succeed")
            .build()
            .expect("test should succeed");

        let graph = DependencyGraph::from_model(&model).expect("test should succeed");
        let order = evaluate_order(&graph).expect("test should succeed");

        // 'a' should come before 'b' and 'c'
        let a_pos = order
            .iter()
            .position(|n| n == "a")
            .expect("test should succeed");
        let b_pos = order
            .iter()
            .position(|n| n == "b")
            .expect("test should succeed");
        let c_pos = order
            .iter()
            .position(|n| n == "c")
            .expect("test should succeed");

        assert!(a_pos < b_pos);
        assert!(b_pos < c_pos);
    }

    #[test]
    fn test_cycle_detection() {
        let model = ModelBuilder::new("test")
            .periods("2025Q1..Q2", None)
            .expect("test should succeed")
            .compute("a", "b + 1")
            .expect("test should succeed")
            .compute("b", "c + 1")
            .expect("test should succeed")
            .compute("c", "a + 1")
            .expect("test should succeed")
            .build()
            .expect("test should succeed");

        let graph = DependencyGraph::from_model(&model).expect("test should succeed");

        // Should detect cycle
        let result = graph.detect_cycles();
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_dependencies() {
        let all_nodes: IndexSet<String> = ["revenue", "cogs", "gross_profit"]
            .iter()
            .map(|s| s.to_string())
            .collect();

        let deps = extract_dependencies("revenue - cogs", &all_nodes).unwrap();
        assert_eq!(deps.len(), 2);
        assert!(deps.contains("revenue"));
        assert!(deps.contains("cogs"));
    }

    #[test]
    fn test_lag_breaks_cycle() {
        let model = ModelBuilder::new("test")
            .periods("2025Q1..Q2", None)
            .expect("test should succeed")
            .compute("a", "lag(b, 1)") // a depends on b (lagged)
            .expect("test should succeed")
            .compute("b", "a + 1") // b depends on a (direct)
            .expect("test should succeed")
            .build()
            .expect("test should succeed");

        let graph = DependencyGraph::from_model(&model).expect("test should succeed");

        // Should NOT detect cycle because a's dependency on b is lagged
        let result = graph.detect_cycles();
        assert!(result.is_ok());

        // Order should be a then b (since b depends on a, and a depends on nothing in current period)
        let order = evaluate_order(&graph).expect("test should succeed");
        let a_pos = order
            .iter()
            .position(|n| n == "a")
            .expect("node a should exist");
        let b_pos = order
            .iter()
            .position(|n| n == "b")
            .expect("node b should exist");
        assert!(a_pos < b_pos);
    }

    #[test]
    fn test_where_clause_adds_dependencies() {
        let model = ModelBuilder::new("test")
            .periods("2025Q1..Q2", None)
            .expect("test should succeed")
            .value("revenue", &[])
            .mixed("margin")
            .formula("1.0")
            .expect("test should succeed")
            .build()
            .where_clause("revenue > 0.0")
            .build()
            .expect("test should succeed");

        let graph = DependencyGraph::from_model(&model).expect("test should succeed");
        assert!(graph.dependencies["margin"].contains("revenue"));
        assert!(graph.dependents["revenue"].contains("margin"));
    }
}
