//! Graph utilities shared across the statements crate.

use indexmap::{IndexMap, IndexSet};

/// Perform a topological sort using Kahn's algorithm.
///
/// The `dependencies` map should contain an entry for every node id we want to
/// sort, where each entry lists the other node ids it depends on. Dependencies
/// that are not present as keys are ignored (they are assumed to be external or
/// already satisfied).
///
/// Returns `Ok(order)` when every node can be scheduled, or `Err(remaining)`
/// containing the ids that participate in (at least) one cycle when the graph
/// is not acyclic.
pub fn toposort_ids(
    dependencies: &IndexMap<String, IndexSet<String>>,
) -> Result<Vec<String>, Vec<String>> {
    // Initialize in-degree for every known node.
    let mut in_degree: IndexMap<String, usize> =
        dependencies.keys().map(|id| (id.clone(), 0usize)).collect();

    // Track dependents so we can decrement in-degree when removing a node.
    let mut dependents: IndexMap<String, IndexSet<String>> = dependencies
        .keys()
        .map(|id| (id.clone(), IndexSet::new()))
        .collect();

    for (node, deps) in dependencies {
        for dep in deps {
            if let Some(degree) = in_degree.get_mut(node) {
                *degree += 1;
            }
            if let Some(children) = dependents.get_mut(dep) {
                children.insert(node.clone());
            }
        }
    }

    // Collect nodes that have no inbound edges. We mimic the previous behavior
    // (Vec + pop) to keep ordering identical.
    let mut stack: Vec<String> = in_degree
        .iter()
        .filter(|(_, &degree)| degree == 0)
        .map(|(id, _)| id.clone())
        .collect();

    let mut order = Vec::with_capacity(in_degree.len());

    while let Some(node) = stack.pop() {
        order.push(node.clone());

        if let Some(children) = dependents.get(&node) {
            for dependent in children {
                if let Some(degree) = in_degree.get_mut(dependent) {
                    *degree = degree.saturating_sub(1);
                    if *degree == 0 {
                        stack.push(dependent.clone());
                    }
                }
            }
        }
    }

    if order.len() == in_degree.len() {
        Ok(order)
    } else {
        let remaining: Vec<String> = in_degree
            .keys()
            .filter(|id| !order.contains(id))
            .cloned()
            .collect();
        Err(remaining)
    }
}
