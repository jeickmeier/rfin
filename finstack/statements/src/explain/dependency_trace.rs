//! Dependency tracing for financial statement nodes.

use crate::error::{Error, Result};
use crate::evaluator::DependencyGraph;
use crate::types::FinancialModelSpec;
use indexmap::IndexSet;
use serde::{Deserialize, Serialize};

/// Traces dependencies between nodes in a financial model.
///
/// The tracer uses the dependency graph to identify which nodes a given node
/// depends on (direct and transitive) and which nodes depend on it.
///
/// # Examples
///
/// ```rust
/// # use finstack_statements::builder::ModelBuilder;
/// # use finstack_statements::evaluator::DependencyGraph;
/// # use finstack_statements::explain::DependencyTracer;
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let model = ModelBuilder::new("demo")
///     .periods("2025Q1..Q2", None)?
///     .compute("revenue", "100000")?
///     .compute("cogs", "revenue * 0.4")?
///     .compute("gross_profit", "revenue - cogs")?
///     .build()?;
///
/// let graph = DependencyGraph::from_model(&model)?;
/// let tracer = DependencyTracer::new(&model, &graph);
///
/// // Get direct dependencies
/// let deps = tracer.direct_dependencies("gross_profit")?;
/// assert_eq!(deps.len(), 2);
/// assert!(deps.contains(&"revenue"));
/// assert!(deps.contains(&"cogs"));
/// # Ok(())
/// # }
/// ```
pub struct DependencyTracer<'a> {
    model: &'a FinancialModelSpec,
    graph: &'a DependencyGraph,
}

impl<'a> DependencyTracer<'a> {
    /// Create a new dependency tracer.
    ///
    /// # Arguments
    ///
    /// * `model` - Financial model specification
    /// * `graph` - Pre-built dependency graph
    pub fn new(model: &'a FinancialModelSpec, graph: &'a DependencyGraph) -> Self {
        Self { model, graph }
    }

    /// Get all direct dependencies for a node.
    ///
    /// Returns the nodes that this node directly references in its formula.
    ///
    /// # Arguments
    ///
    /// * `node_id` - Node identifier to inspect
    ///
    /// # Returns
    ///
    /// Vector of node IDs that are direct dependencies
    pub fn direct_dependencies(&self, node_id: &str) -> Result<Vec<&str>> {
        let deps = self
            .graph
            .get_dependencies(node_id)
            .ok_or_else(|| Error::invalid_input(format!("Node '{}' not found", node_id)))?;

        Ok(deps.iter().map(|s| s.as_str()).collect())
    }

    /// Get all transitive dependencies (recursive).
    ///
    /// Returns all nodes that this node depends on, directly or indirectly.
    ///
    /// # Arguments
    ///
    /// * `node_id` - Node identifier to inspect
    ///
    /// # Returns
    ///
    /// Vector of node IDs in dependency order (dependencies before dependents)
    pub fn all_dependencies(&self, node_id: &str) -> Result<Vec<String>> {
        let mut all_deps = IndexSet::new();
        let mut visited = IndexSet::new();
        self.collect_transitive_deps_owned(node_id, &mut all_deps, &mut visited)?;
        Ok(all_deps.into_iter().collect())
    }

    /// Get dependency tree as hierarchical structure.
    ///
    /// Builds a tree showing the complete dependency hierarchy for a node.
    ///
    /// # Arguments
    ///
    /// * `node_id` - Root node for the dependency tree
    ///
    /// # Returns
    ///
    /// Dependency tree structure suitable for visualization
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use finstack_statements::builder::ModelBuilder;
    /// # use finstack_statements::evaluator::DependencyGraph;
    /// # use finstack_statements::explain::DependencyTracer;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let model = ModelBuilder::new("demo")
    /// #     .periods("2025Q1..Q2", None)?
    /// #     .compute("a", "10")?
    /// #     .compute("b", "a * 2")?
    /// #     .compute("c", "a + b")?
    /// #     .build()?;
    /// # let graph = DependencyGraph::from_model(&model)?;
    /// let tracer = DependencyTracer::new(&model, &graph);
    /// let tree = tracer.dependency_tree("c")?;
    ///
    /// assert_eq!(tree.node_id, "c");
    /// assert_eq!(tree.children.len(), 2);
    /// assert_eq!(tree.depth(), 2);
    /// # Ok(())
    /// # }
    /// ```
    pub fn dependency_tree(&self, node_id: &str) -> Result<DependencyTree> {
        let mut visited = IndexSet::new();
        self.build_tree(node_id, &mut visited)
    }

    /// Get nodes that depend on this node (reverse dependencies).
    ///
    /// # Arguments
    ///
    /// * `node_id` - Node identifier to inspect
    ///
    /// # Returns
    ///
    /// Vector of node IDs that depend on this node
    pub fn dependents(&self, node_id: &str) -> Result<Vec<&str>> {
        let deps = self
            .graph
            .dependents
            .get(node_id)
            .ok_or_else(|| Error::invalid_input(format!("Node '{}' not found", node_id)))?;

        Ok(deps.iter().map(|s| s.as_str()).collect())
    }

    // Internal helper to collect transitive dependencies (owned strings)
    fn collect_transitive_deps_owned(
        &self,
        node_id: &str,
        all_deps: &mut IndexSet<String>,
        visited: &mut IndexSet<String>,
    ) -> Result<()> {
        // Avoid infinite loops
        if visited.contains(node_id) {
            return Ok(());
        }
        visited.insert(node_id.to_string());

        // Get direct dependencies
        let direct_deps = self.direct_dependencies(node_id)?;

        // Recursively process each dependency
        for dep_id in direct_deps {
            self.collect_transitive_deps_owned(dep_id, all_deps, visited)?;
            all_deps.insert(dep_id.to_string());
        }

        Ok(())
    }

    // Internal helper to build dependency tree
    fn build_tree(&self, node_id: &str, visited: &mut IndexSet<String>) -> Result<DependencyTree> {
        // Get node spec for formula
        let node_spec = self
            .model
            .nodes
            .get(node_id)
            .ok_or_else(|| Error::invalid_input(format!("Node '{}' not found", node_id)))?;

        let formula = node_spec.formula_text.clone();

        // Get direct dependencies
        let deps = self.direct_dependencies(node_id)?;

        // Build children (but avoid cycles)
        let mut children = Vec::new();
        for dep_id in deps {
            // Skip if we've already visited this node (prevents infinite recursion)
            if visited.contains(dep_id) {
                // Add a marker node indicating cycle
                children.push(DependencyTree {
                    node_id: format!("{} (cycle)", dep_id),
                    formula: None,
                    children: Vec::new(),
                });
            } else {
                visited.insert(dep_id.to_string());
                children.push(self.build_tree(dep_id, visited)?);
                visited.shift_remove(dep_id);
            }
        }

        Ok(DependencyTree {
            node_id: node_id.to_string(),
            formula,
            children,
        })
    }
}

/// Hierarchical dependency tree structure.
///
/// Represents the complete dependency hierarchy for a node, suitable for
/// visualization and analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyTree {
    /// Node identifier
    pub node_id: String,

    /// Formula text (if node is calculated)
    pub formula: Option<String>,

    /// Child dependencies
    pub children: Vec<DependencyTree>,
}

impl DependencyTree {
    /// Get the maximum depth of the tree.
    ///
    /// # Returns
    ///
    /// Maximum depth (0 for a leaf node, 1 for a node with children, etc.)
    pub fn depth(&self) -> usize {
        if self.children.is_empty() {
            0
        } else {
            1 + self.children.iter().map(|c| c.depth()).max().unwrap_or(0)
        }
    }

    /// Count total number of nodes in the tree.
    ///
    /// # Returns
    ///
    /// Total node count including this node and all descendants
    pub fn node_count(&self) -> usize {
        1 + self.children.iter().map(|c| c.node_count()).sum::<usize>()
    }

    /// Convert tree to ASCII representation.
    ///
    /// # Returns
    ///
    /// ASCII tree string suitable for console output
    pub fn to_string_ascii(&self) -> String {
        crate::explain::visualization::render_tree_ascii(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::ModelBuilder;

    #[test]
    fn test_direct_dependencies() {
        let model = ModelBuilder::new("test")
            .periods("2025Q1..Q2", None)
            .expect("valid operation")
            .compute("a", "10")
            .expect("valid operation")
            .compute("b", "a * 2")
            .expect("valid operation")
            .compute("c", "a + b")
            .expect("valid operation")
            .build()
            .expect("valid operation");

        let graph = DependencyGraph::from_model(&model).expect("should build dependency graph");
        let tracer = DependencyTracer::new(&model, &graph);

        let deps_a = tracer.direct_dependencies("a").expect("should get dependencies");
        assert_eq!(deps_a.len(), 0);

        let deps_b = tracer.direct_dependencies("b").expect("should get dependencies");
        assert_eq!(deps_b.len(), 1);
        assert!(deps_b.contains(&"a"));

        let deps_c = tracer.direct_dependencies("c").expect("should get dependencies");
        assert_eq!(deps_c.len(), 2);
        assert!(deps_c.contains(&"a"));
        assert!(deps_c.contains(&"b"));
    }

    #[test]
    fn test_all_dependencies() {
        let model = ModelBuilder::new("test")
            .periods("2025Q1..Q2", None)
            .expect("valid operation")
            .compute("a", "10")
            .expect("valid operation")
            .compute("b", "a * 2")
            .expect("valid operation")
            .compute("c", "b + 5")
            .expect("valid operation")
            .compute("d", "c - a")
            .expect("valid operation")
            .build()
            .expect("valid operation");

        let graph = DependencyGraph::from_model(&model).expect("should build dependency graph");
        let tracer = DependencyTracer::new(&model, &graph);

        // d depends on c, c depends on b, b depends on a, and d also depends on a
        let deps = tracer.all_dependencies("d").expect("should get all dependencies");
        assert_eq!(deps.len(), 3); // a, b, c
        assert!(deps.contains(&"a".to_string()));
        assert!(deps.contains(&"b".to_string()));
        assert!(deps.contains(&"c".to_string()));
    }

    #[test]
    fn test_dependency_tree() {
        let model = ModelBuilder::new("test")
            .periods("2025Q1..Q2", None)
            .expect("valid operation")
            .compute("revenue", "100000")
            .expect("valid operation")
            .compute("cogs", "revenue * 0.4")
            .expect("valid operation")
            .compute("gross_profit", "revenue - cogs")
            .expect("valid operation")
            .build()
            .expect("valid operation");

        let graph = DependencyGraph::from_model(&model).expect("should build dependency graph");
        let tracer = DependencyTracer::new(&model, &graph);

        let tree = tracer.dependency_tree("gross_profit").expect("should build dependency tree");
        assert_eq!(tree.node_id, "gross_profit");
        assert_eq!(tree.children.len(), 2);
        assert_eq!(tree.depth(), 2); // gross_profit -> cogs -> revenue (depth 2)
        assert_eq!(tree.node_count(), 4); // gross_profit, revenue, cogs, revenue (appears twice)
    }

    #[test]
    fn test_dependents() {
        let model = ModelBuilder::new("test")
            .periods("2025Q1..Q2", None)
            .expect("valid operation")
            .compute("a", "10")
            .expect("valid operation")
            .compute("b", "a * 2")
            .expect("valid operation")
            .compute("c", "a + 5")
            .expect("valid operation")
            .build()
            .expect("valid operation");

        let graph = DependencyGraph::from_model(&model).expect("should build dependency graph");
        let tracer = DependencyTracer::new(&model, &graph);

        // Both b and c depend on a
        let dependents = tracer.dependents("a").expect("should get dependents");
        assert_eq!(dependents.len(), 2);
        assert!(dependents.contains(&"b"));
        assert!(dependents.contains(&"c"));

        // Nothing depends on b
        let dependents_b = tracer.dependents("b").expect("should get dependents");
        assert_eq!(dependents_b.len(), 0);
    }

    #[test]
    fn test_node_not_found() {
        let model = ModelBuilder::new("test")
            .periods("2025Q1..Q2", None)
            .expect("valid operation")
            .compute("a", "10")
            .expect("valid operation")
            .build()
            .expect("valid operation");

        let graph = DependencyGraph::from_model(&model).expect("should build dependency graph");
        let tracer = DependencyTracer::new(&model, &graph);

        let result = tracer.direct_dependencies("nonexistent");
        assert!(result.is_err());
    }
}
