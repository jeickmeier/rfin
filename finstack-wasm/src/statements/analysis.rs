//! WASM bindings for statement analysis tools.
//!
//! Provides tools for dependency tracing, goal seeking, and other analysis.

use crate::core::error::js_error;
use crate::statements::types::JsFinancialModelSpec;
use crate::utils::json::to_js_value;
use finstack_statements::evaluator::DependencyGraph;
use finstack_statements_analytics::analysis::{DependencyTracer, DependencyTree};
use wasm_bindgen::prelude::*;

// =============================================================================
// DependencyTree
// =============================================================================

/// Hierarchical dependency tree structure.
#[wasm_bindgen(js_name = DependencyTree)]
pub struct JsDependencyTree {
    inner: DependencyTree,
}

#[wasm_bindgen(js_class = DependencyTree)]
impl JsDependencyTree {
    /// Node identifier.
    #[wasm_bindgen(getter, js_name = nodeId)]
    pub fn node_id(&self) -> String {
        self.inner.node_id.clone()
    }

    /// Formula text (if node is calculated).
    #[wasm_bindgen(getter)]
    pub fn formula(&self) -> Option<String> {
        self.inner.formula.clone()
    }

    /// Get children as JS array.
    #[wasm_bindgen(getter)]
    pub fn children(&self) -> js_sys::Array {
        let arr = js_sys::Array::new();
        for child in &self.inner.children {
            arr.push(
                &JsDependencyTree {
                    inner: child.clone(),
                }
                .into(),
            );
        }
        arr
    }

    /// Get the maximum depth of the tree.
    #[wasm_bindgen]
    pub fn depth(&self) -> usize {
        self.inner.depth()
    }

    /// Count total number of nodes in the tree.
    #[wasm_bindgen(js_name = nodeCount)]
    pub fn node_count(&self) -> usize {
        self.inner.node_count()
    }

    /// Convert tree to ASCII representation.
    #[wasm_bindgen(js_name = toAscii)]
    pub fn to_ascii(&self) -> String {
        self.inner.to_string_ascii()
    }

    /// Serialize to JavaScript object.
    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }

    /// Serialize to JSON string.
    #[wasm_bindgen(js_name = toJsonString)]
    pub fn to_json_string(&self) -> Result<String, JsValue> {
        serde_json::to_string_pretty(&self.inner).map_err(|e| js_error(e.to_string()))
    }
}

// =============================================================================
// Dependency Analysis Functions
// =============================================================================

/// Get the direct dependencies for a node.
///
/// # Arguments
/// * `model` - Financial model specification
/// * `node_id` - Node identifier to inspect
///
/// # Returns
/// Array of node IDs that are direct dependencies.
#[wasm_bindgen(js_name = directDependencies)]
pub fn direct_dependencies(
    model: &JsFinancialModelSpec,
    node_id: &str,
) -> Result<js_sys::Array, JsValue> {
    let spec = model.inner();
    let graph =
        DependencyGraph::from_model(spec).map_err(|e| js_error(format!("Graph error: {}", e)))?;
    let tracer = DependencyTracer::new(spec, &graph);

    let deps = tracer
        .direct_dependencies(node_id)
        .map_err(|e| js_error(e.to_string()))?;

    let arr = js_sys::Array::new();
    for dep in deps {
        arr.push(&JsValue::from_str(dep));
    }
    Ok(arr)
}

/// Get all transitive dependencies for a node.
///
/// # Arguments
/// * `model` - Financial model specification
/// * `node_id` - Node identifier to inspect
///
/// # Returns
/// Array of node IDs in dependency order (dependencies before dependents).
#[wasm_bindgen(js_name = allDependencies)]
pub fn all_dependencies(
    model: &JsFinancialModelSpec,
    node_id: &str,
) -> Result<js_sys::Array, JsValue> {
    let spec = model.inner();
    let graph =
        DependencyGraph::from_model(spec).map_err(|e| js_error(format!("Graph error: {}", e)))?;
    let tracer = DependencyTracer::new(spec, &graph);

    let deps = tracer
        .all_dependencies(node_id)
        .map_err(|e| js_error(e.to_string()))?;

    let arr = js_sys::Array::new();
    for dep in deps {
        arr.push(&JsValue::from_str(&dep));
    }
    Ok(arr)
}

/// Build a dependency tree for a node.
///
/// # Arguments
/// * `model` - Financial model specification
/// * `node_id` - Root node for the dependency tree
///
/// # Returns
/// Dependency tree structure suitable for visualization.
#[wasm_bindgen(js_name = dependencyTree)]
pub fn dependency_tree(
    model: &JsFinancialModelSpec,
    node_id: &str,
) -> Result<JsDependencyTree, JsValue> {
    let spec = model.inner();
    let graph =
        DependencyGraph::from_model(spec).map_err(|e| js_error(format!("Graph error: {}", e)))?;
    let tracer = DependencyTracer::new(spec, &graph);

    let tree = tracer
        .dependency_tree(node_id)
        .map_err(|e| js_error(e.to_string()))?;

    Ok(JsDependencyTree { inner: tree })
}

/// Get nodes that depend on a given node (reverse dependencies).
///
/// # Arguments
/// * `model` - Financial model specification
/// * `node_id` - Node identifier to inspect
///
/// # Returns
/// Array of node IDs that depend on this node.
#[wasm_bindgen(js_name = dependents)]
pub fn dependents(model: &JsFinancialModelSpec, node_id: &str) -> Result<js_sys::Array, JsValue> {
    let spec = model.inner();
    let graph =
        DependencyGraph::from_model(spec).map_err(|e| js_error(format!("Graph error: {}", e)))?;
    let tracer = DependencyTracer::new(spec, &graph);

    let deps = tracer
        .dependents(node_id)
        .map_err(|e| js_error(e.to_string()))?;

    let arr = js_sys::Array::new();
    for dep in deps {
        arr.push(&JsValue::from_str(dep));
    }
    Ok(arr)
}

/// Render a dependency tree as ASCII text.
///
/// # Arguments
/// * `model` - Financial model specification
/// * `node_id` - Root node for the dependency tree
///
/// # Returns
/// ASCII tree string suitable for console output.
#[wasm_bindgen(js_name = renderDependencyTreeAscii)]
pub fn render_dependency_tree_ascii(
    model: &JsFinancialModelSpec,
    node_id: &str,
) -> Result<String, JsValue> {
    let tree = dependency_tree(model, node_id)?;
    Ok(tree.to_ascii())
}
