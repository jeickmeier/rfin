//! Tree visualization utilities.

use crate::explain::DependencyTree;
use crate::evaluator::Results;
use finstack_core::dates::PeriodId;

/// Render dependency tree as ASCII art.
///
/// # Arguments
///
/// * `tree` - Dependency tree to render
///
/// # Returns
///
/// ASCII representation suitable for console output
///
/// # Examples
///
/// ```rust
/// # use finstack_statements::builder::ModelBuilder;
/// # use finstack_statements::evaluator::DependencyGraph;
/// # use finstack_statements::explain::{DependencyTracer, render_tree_ascii};
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # let model = ModelBuilder::new("demo")
/// #     .periods("2025Q1..Q2", None)?
/// #     .compute("revenue", "100000")?
/// #     .compute("cogs", "revenue * 0.4")?
/// #     .compute("gross_profit", "revenue - cogs")?
/// #     .build()?;
/// # let graph = DependencyGraph::from_model(&model)?;
/// let tracer = DependencyTracer::new(&model, &graph);
/// let tree = tracer.dependency_tree("gross_profit")?;
///
/// let ascii = render_tree_ascii(&tree);
/// println!("{}", ascii);
/// // Output:
/// // gross_profit
/// // ├── revenue
/// // └── cogs
/// //     └── revenue
/// # Ok(())
/// # }
/// ```
pub fn render_tree_ascii(tree: &DependencyTree) -> String {
    let mut output = String::new();
    render_tree_recursive(tree, &mut output, "", true);
    output
}

/// Render dependency tree with values from results.
///
/// # Arguments
///
/// * `tree` - Dependency tree to render
/// * `results` - Evaluation results containing node values
/// * `period` - Period to display values for
///
/// # Returns
///
/// ASCII tree with values
///
/// # Examples
///
/// ```rust
/// # use finstack_statements::builder::ModelBuilder;
/// # use finstack_statements::evaluator::{DependencyGraph, Evaluator};
/// # use finstack_statements::explain::{DependencyTracer, render_tree_detailed};
/// # use finstack_core::dates::PeriodId;
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # let model = ModelBuilder::new("demo")
/// #     .periods("2025Q1..Q2", None)?
/// #     .compute("revenue", "100000")?
/// #     .compute("cogs", "revenue * 0.4")?
/// #     .compute("gross_profit", "revenue - cogs")?
/// #     .build()?;
/// # let mut evaluator = Evaluator::new();
/// # let results = evaluator.evaluate(&model)?;
/// # let graph = DependencyGraph::from_model(&model)?;
/// let tracer = DependencyTracer::new(&model, &graph);
/// let tree = tracer.dependency_tree("gross_profit")?;
///
/// let period = PeriodId::quarter(2025, 1);
/// let detailed = render_tree_detailed(&tree, &results, &period);
/// println!("{}", detailed);
/// // Output:
/// // gross_profit = 60,000.00
/// // ├── revenue = 100,000.00
/// // └── cogs = 40,000.00
/// //     └── revenue = 100,000.00
/// # Ok(())
/// # }
/// ```
pub fn render_tree_detailed(
    tree: &DependencyTree,
    results: &Results,
    period: &PeriodId,
) -> String {
    let mut output = String::new();
    render_tree_with_values(tree, results, period, &mut output, "", true);
    output
}

// Recursive helper for ASCII rendering
fn render_tree_recursive(
    tree: &DependencyTree,
    output: &mut String,
    prefix: &str,
    is_last: bool,
) {
    // Render current node
    let connector = if is_last { "└── " } else { "├── " };
    let node_name = if prefix.is_empty() {
        // Root node
        tree.node_id.clone()
    } else {
        format!("{}{}", connector, tree.node_id)
    };

    output.push_str(&node_name);
    if let Some(formula) = &tree.formula {
        output.push_str(&format!(" ({})", formula));
    }
    output.push('\n');

    // Render children
    let child_count = tree.children.len();
    for (i, child) in tree.children.iter().enumerate() {
        let is_last_child = i == child_count - 1;
        let new_prefix = if prefix.is_empty() {
            String::new()
        } else {
            format!(
                "{}{}",
                prefix,
                if is_last { "    " } else { "│   " }
            )
        };

        render_tree_recursive(child, output, &new_prefix, is_last_child);
    }
}

// Recursive helper for rendering with values
fn render_tree_with_values(
    tree: &DependencyTree,
    results: &Results,
    period: &PeriodId,
    output: &mut String,
    prefix: &str,
    is_last: bool,
) {
    // Get value for this node
    let value = results.get(&tree.node_id, period);

    // Render current node
    let connector = if is_last { "└── " } else { "├── " };
    let node_display = if prefix.is_empty() {
        // Root node
        if let Some(v) = value {
            format!("{} = {:.2}", tree.node_id, v)
        } else {
            tree.node_id.clone()
        }
    } else {
        let base = format!("{}{}", connector, tree.node_id);
        if let Some(v) = value {
            format!("{} = {:.2}", base, v)
        } else {
            base
        }
    };

    output.push_str(&node_display);
    output.push('\n');

    // Render children
    let child_count = tree.children.len();
    for (i, child) in tree.children.iter().enumerate() {
        let is_last_child = i == child_count - 1;
        let new_prefix = if prefix.is_empty() {
            String::new()
        } else {
            format!(
                "{}{}",
                prefix,
                if is_last { "    " } else { "│   " }
            )
        };

        render_tree_with_values(child, results, period, output, &new_prefix, is_last_child);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::ModelBuilder;
    use crate::evaluator::{DependencyGraph, Evaluator};
    use crate::explain::DependencyTracer;

    #[test]
    fn test_render_tree_ascii() {
        let model = ModelBuilder::new("test")
            .periods("2025Q1..Q2", None)
            .unwrap()
            .compute("a", "10")
            .unwrap()
            .compute("b", "a * 2")
            .unwrap()
            .compute("c", "a + b")
            .unwrap()
            .build()
            .unwrap();

        let graph = DependencyGraph::from_model(&model).unwrap();
        let tracer = DependencyTracer::new(&model, &graph);
        let tree = tracer.dependency_tree("c").unwrap();

        let ascii = render_tree_ascii(&tree);
        println!("ASCII tree:\n{}", ascii);
        assert!(ascii.contains("c"));
        assert!(ascii.contains("a"));
        assert!(ascii.contains("b"));
        // The tree should have structure (root has 2 children)
        assert_eq!(tree.children.len(), 2);
    }

    #[test]
    fn test_render_tree_detailed() {
        let period = PeriodId::quarter(2025, 1);
        let model = ModelBuilder::new("test")
            .periods("2025Q1..Q2", None)
            .unwrap()
            .compute("revenue", "100000")
            .unwrap()
            .compute("cogs", "revenue * 0.4")
            .unwrap()
            .compute("gross_profit", "revenue - cogs")
            .unwrap()
            .build()
            .unwrap();

        let mut evaluator = Evaluator::new();
        let results = evaluator.evaluate(&model).unwrap();

        let graph = DependencyGraph::from_model(&model).unwrap();
        let tracer = DependencyTracer::new(&model, &graph);
        let tree = tracer.dependency_tree("gross_profit").unwrap();

        let detailed = render_tree_detailed(&tree, &results, &period);
        assert!(detailed.contains("gross_profit = 60000.00"));
        assert!(detailed.contains("revenue = 100000.00"));
        assert!(detailed.contains("cogs = 40000.00"));
    }

    #[test]
    fn test_render_empty_tree() {
        let model = ModelBuilder::new("test")
            .periods("2025Q1..Q2", None)
            .unwrap()
            .compute("a", "10")
            .unwrap()
            .build()
            .unwrap();

        let graph = DependencyGraph::from_model(&model).unwrap();
        let tracer = DependencyTracer::new(&model, &graph);
        let tree = tracer.dependency_tree("a").unwrap();

        let ascii = render_tree_ascii(&tree);
        assert!(ascii.contains("a"));
        assert_eq!(ascii.lines().count(), 1); // Just the root node
    }
}

