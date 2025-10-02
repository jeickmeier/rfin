//! Dependency graph construction and topological sorting.

use crate::error::{Error, Result};
use crate::types::FinancialModelSpec;
use indexmap::{IndexMap, IndexSet};

/// Dependency graph for nodes in a financial model.
#[derive(Debug)]
pub struct DependencyGraph {
    /// Map of node_id → set of dependencies (nodes it depends on)
    pub dependencies: IndexMap<String, IndexSet<String>>,

    /// Map of node_id → set of dependents (nodes that depend on it)
    pub dependents: IndexMap<String, IndexSet<String>>,
}

impl DependencyGraph {
    /// Build a dependency graph from a model specification.
    pub fn from_model(model: &FinancialModelSpec) -> Result<Self> {
        let mut dependencies = IndexMap::new();
        let mut dependents = IndexMap::new();

        // Initialize empty sets for all nodes
        for node_id in model.nodes.keys() {
            dependencies.insert(node_id.clone(), IndexSet::new());
            dependents.insert(node_id.clone(), IndexSet::new());
        }

        // Extract dependencies from formulas
        for (node_id, node_spec) in &model.nodes {
            if let Some(formula) = &node_spec.formula_text {
                let node_deps =
                    extract_dependencies(formula, &model.nodes.keys().cloned().collect());

                for dep in &node_deps {
                    // Add to this node's dependencies
                    dependencies.get_mut(node_id).unwrap().insert(dep.clone());

                    // Add this node to the dependent's dependents list
                    dependents.get_mut(dep).unwrap().insert(node_id.clone());
                }
            }
        }

        Ok(Self {
            dependencies,
            dependents,
        })
    }

    /// Get dependencies for a node.
    pub fn get_dependencies(&self, node_id: &str) -> Option<&IndexSet<String>> {
        self.dependencies.get(node_id)
    }

    /// Check for circular dependencies.
    pub fn detect_cycles(&self) -> Result<()> {
        for node_id in self.dependencies.keys() {
            if let Some(cycle) = self.find_cycle_from(node_id) {
                return Err(Error::CircularDependency { path: cycle });
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
        if path.contains(&node.to_string()) {
            let cycle_start = path.iter().position(|n| n == node).unwrap();
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

/// Compute topological sort order for evaluation.
///
/// Returns nodes in an order where all dependencies are evaluated before dependents.
pub fn evaluate_order(graph: &DependencyGraph) -> Result<Vec<String>> {
    // Kahn's algorithm for topological sort
    let mut in_degree = IndexMap::new();

    // Initialize in-degrees
    for node_id in graph.dependencies.keys() {
        let degree = graph.dependencies[node_id].len();
        in_degree.insert(node_id.clone(), degree);
    }

    // Queue nodes with no dependencies
    let mut queue: Vec<String> = in_degree
        .iter()
        .filter(|(_, &degree)| degree == 0)
        .map(|(node, _)| node.clone())
        .collect();

    let mut result = Vec::new();

    while let Some(node) = queue.pop() {
        result.push(node.clone());

        // Reduce in-degree of dependents
        if let Some(deps) = graph.dependents.get(&node) {
            for dependent in deps {
                let degree = in_degree.get_mut(dependent).unwrap();
                *degree -= 1;
                if *degree == 0 {
                    queue.push(dependent.clone());
                }
            }
        }
    }

    // If we haven't processed all nodes, there's a cycle
    if result.len() != graph.dependencies.len() {
        return Err(Error::eval("Circular dependency detected in model"));
    }

    Ok(result)
}

/// Extract node dependencies from a formula string.
///
/// This is a simple implementation that looks for known node IDs in the formula.
/// It's not perfect but sufficient for Phase 3.
fn extract_dependencies(formula: &str, all_node_ids: &IndexSet<String>) -> IndexSet<String> {
    let mut deps = IndexSet::new();

    // Simple approach: check if any node ID appears as a whole word in the formula
    for node_id in all_node_ids {
        // Check for node_id as a standalone identifier
        // This is a simple heuristic - in a more robust implementation, we'd parse the AST
        if formula.contains(node_id.as_str()) {
            // Verify it's not part of a larger identifier
            let is_standalone = formula.match_indices(node_id.as_str()).any(|(idx, _)| {
                let before = if idx > 0 {
                    formula.chars().nth(idx - 1)
                } else {
                    None
                };
                let after = formula.chars().nth(idx + node_id.len());

                let before_ok = before.map_or(true, |c| !c.is_alphanumeric() && c != '_');
                let after_ok = after.map_or(true, |c| !c.is_alphanumeric() && c != '_');

                before_ok && after_ok
            });

            if is_standalone {
                deps.insert(node_id.clone());
            }
        }
    }

    deps
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::ModelBuilder;

    #[test]
    fn test_simple_dag() {
        let model = ModelBuilder::new("test")
            .periods("2025Q1..Q2", None)
            .unwrap()
            .compute("a", "10")
            .unwrap()
            .compute("b", "a * 2")
            .unwrap()
            .compute("c", "b + a")
            .unwrap()
            .build()
            .unwrap();

        let graph = DependencyGraph::from_model(&model).unwrap();

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
            .unwrap()
            .compute("a", "10")
            .unwrap()
            .compute("b", "a * 2")
            .unwrap()
            .compute("c", "b + a")
            .unwrap()
            .build()
            .unwrap();

        let graph = DependencyGraph::from_model(&model).unwrap();
        let order = evaluate_order(&graph).unwrap();

        // 'a' should come before 'b' and 'c'
        let a_pos = order.iter().position(|n| n == "a").unwrap();
        let b_pos = order.iter().position(|n| n == "b").unwrap();
        let c_pos = order.iter().position(|n| n == "c").unwrap();

        assert!(a_pos < b_pos);
        assert!(b_pos < c_pos);
    }

    #[test]
    fn test_cycle_detection() {
        let model = ModelBuilder::new("test")
            .periods("2025Q1..Q2", None)
            .unwrap()
            .compute("a", "b + 1")
            .unwrap()
            .compute("b", "c + 1")
            .unwrap()
            .compute("c", "a + 1")
            .unwrap()
            .build()
            .unwrap();

        let graph = DependencyGraph::from_model(&model).unwrap();

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

        let deps = extract_dependencies("revenue - cogs", &all_nodes);
        assert_eq!(deps.len(), 2);
        assert!(deps.contains("revenue"));
        assert!(deps.contains("cogs"));
    }
}
