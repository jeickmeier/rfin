//! Explain module bindings for financial models.

use crate::statements::error::stmt_to_py;
use crate::statements::evaluator::{PyDependencyGraph, PyResults};
use crate::statements::types::model::PyFinancialModelSpec;
use crate::statements::types::node::PyNodeType;
use finstack_statements::explain::{
    render_tree_ascii, render_tree_detailed, DependencyTracer, DependencyTree, Explanation,
    ExplanationStep, FormulaExplainer,
};
use pyo3::prelude::*;
use pyo3::types::PyModule;
use pyo3::Bound;

/// Step in a formula calculation breakdown.
#[pyclass(
    module = "finstack.statements.explain",
    name = "ExplanationStep",
    frozen
)]
#[derive(Clone)]
pub struct PyExplanationStep {
    inner: ExplanationStep,
}

#[pymethods]
impl PyExplanationStep {
    #[new]
    #[pyo3(signature = (component, value, operation=None))]
    /// Create an explanation step.
    ///
    /// Parameters
    /// ----------
    /// component : str
    ///     Component identifier (e.g., "revenue")
    /// value : float
    ///     Value of the component
    /// operation : str, optional
    ///     Operation applied (e.g., "+", "-", "*", "/")
    ///
    /// Returns
    /// -------
    /// ExplanationStep
    ///     Explanation step
    fn new(component: String, value: f64, operation: Option<String>) -> Self {
        Self {
            inner: ExplanationStep {
                component,
                value,
                operation,
            },
        }
    }

    #[getter]
    fn component(&self) -> String {
        self.inner.component.clone()
    }

    #[getter]
    fn value(&self) -> f64 {
        self.inner.value
    }

    #[getter]
    fn operation(&self) -> Option<String> {
        self.inner.operation.clone()
    }

    fn __repr__(&self) -> String {
        if let Some(op) = &self.inner.operation {
            format!(
                "ExplanationStep('{}', {:.2}, op='{}')",
                self.inner.component, self.inner.value, op
            )
        } else {
            format!(
                "ExplanationStep('{}', {:.2})",
                self.inner.component, self.inner.value
            )
        }
    }
}

/// Detailed explanation of a node's calculation.
#[pyclass(
    module = "finstack.statements.explain",
    name = "Explanation",
    frozen
)]
#[derive(Clone)]
pub struct PyExplanation {
    inner: Explanation,
}

#[pymethods]
impl PyExplanation {
    #[getter]
    fn node_id(&self) -> String {
        self.inner.node_id.clone()
    }

    #[getter]
    fn period_id(&self) -> crate::core::dates::periods::PyPeriodId {
        crate::core::dates::periods::PyPeriodId::new(self.inner.period_id)
    }

    #[getter]
    fn final_value(&self) -> f64 {
        self.inner.final_value
    }

    #[getter]
    fn node_type(&self) -> PyNodeType {
        PyNodeType::new(self.inner.node_type)
    }

    #[getter]
    fn formula_text(&self) -> Option<String> {
        self.inner.formula_text.clone()
    }

    #[getter]
    fn breakdown(&self) -> Vec<PyExplanationStep> {
        self.inner
            .breakdown
            .iter()
            .map(|step| PyExplanationStep {
                inner: step.clone(),
            })
            .collect()
    }

    #[pyo3(signature = ())]
    /// Convert explanation to detailed string format.
    ///
    /// Returns
    /// -------
    /// str
    ///     Human-readable explanation of the calculation
    fn to_string_detailed(&self) -> String {
        self.inner.to_string_detailed()
    }

    #[pyo3(signature = ())]
    /// Convert explanation to compact string format.
    ///
    /// Returns
    /// -------
    /// str
    ///     Compact single-line summary
    fn to_string_compact(&self) -> String {
        self.inner.to_string_compact()
    }

    fn __repr__(&self) -> String {
        format!(
            "Explanation(node='{}', period={}, value={:.2})",
            self.inner.node_id, self.inner.period_id, self.inner.final_value
        )
    }

    fn __str__(&self) -> String {
        self.inner.to_string_detailed()
    }
}

/// Formula explainer for financial models.
#[pyclass(
    module = "finstack.statements.explain",
    name = "FormulaExplainer",
    unsendable
)]
pub struct PyFormulaExplainer {
    model: PyFinancialModelSpec,
    results: PyResults,
}

#[pymethods]
impl PyFormulaExplainer {
    #[new]
    #[pyo3(signature = (model, results))]
    /// Create a new formula explainer.
    ///
    /// Parameters
    /// ----------
    /// model : FinancialModelSpec
    ///     Financial model specification
    /// results : Results
    ///     Evaluation results
    ///
    /// Returns
    /// -------
    /// FormulaExplainer
    ///     Explainer instance
    fn new(model: &PyFinancialModelSpec, results: &PyResults) -> Self {
        Self {
            model: model.clone(),
            results: results.clone(),
        }
    }

    #[pyo3(signature = (node_id, period))]
    /// Explain how a node's value was calculated for a specific period.
    ///
    /// Parameters
    /// ----------
    /// node_id : str
    ///     Node identifier
    /// period : PeriodId
    ///     Period to explain
    ///
    /// Returns
    /// -------
    /// Explanation
    ///     Detailed explanation of the calculation
    fn explain(
        &self,
        node_id: &str,
        period: &crate::core::dates::periods::PyPeriodId,
    ) -> PyResult<PyExplanation> {
        let explainer = FormulaExplainer::new(&self.model.inner, &self.results.inner);
        let explanation = explainer.explain(node_id, &period.inner).map_err(stmt_to_py)?;
        Ok(PyExplanation { inner: explanation })
    }

    fn __repr__(&self) -> String {
        format!(
            "FormulaExplainer(model='{}', nodes={})",
            self.model.inner.id,
            self.results.inner.nodes.len()
        )
    }
}

/// Hierarchical dependency tree structure.
#[pyclass(
    module = "finstack.statements.explain",
    name = "DependencyTree",
    frozen
)]
#[derive(Clone)]
pub struct PyDependencyTree {
    inner: DependencyTree,
}

#[pymethods]
impl PyDependencyTree {
    #[getter]
    fn node_id(&self) -> String {
        self.inner.node_id.clone()
    }

    #[getter]
    fn formula(&self) -> Option<String> {
        self.inner.formula.clone()
    }

    #[getter]
    fn children(&self) -> Vec<PyDependencyTree> {
        self.inner
            .children
            .iter()
            .map(|child| PyDependencyTree {
                inner: child.clone(),
            })
            .collect()
    }

    #[pyo3(signature = ())]
    /// Get the maximum depth of the tree.
    ///
    /// Returns
    /// -------
    /// int
    ///     Maximum depth (0 for a leaf node, 1 for a node with children, etc.)
    fn depth(&self) -> usize {
        self.inner.depth()
    }

    #[pyo3(signature = ())]
    /// Convert tree to ASCII string representation.
    ///
    /// Returns
    /// -------
    /// str
    ///     ASCII tree visualization
    fn to_string_ascii(&self) -> String {
        render_tree_ascii(&self.inner)
    }

    fn __repr__(&self) -> String {
        format!(
            "DependencyTree(node='{}', children={}, depth={})",
            self.inner.node_id,
            self.inner.children.len(),
            self.inner.depth()
        )
    }

    fn __str__(&self) -> String {
        self.to_string_ascii()
    }
}

/// Dependency tracer for financial models.
#[pyclass(
    module = "finstack.statements.explain",
    name = "DependencyTracer",
    unsendable
)]
pub struct PyDependencyTracer {
    model: PyFinancialModelSpec,
    graph: Py<PyDependencyGraph>,
}

#[pymethods]
impl PyDependencyTracer {
    #[new]
    #[pyo3(signature = (model, graph))]
    /// Create a new dependency tracer.
    ///
    /// Parameters
    /// ----------
    /// model : FinancialModelSpec
    ///     Financial model specification
    /// graph : DependencyGraph
    ///     Pre-built dependency graph
    ///
    /// Returns
    /// -------
    /// DependencyTracer
    ///     Tracer instance
    fn new(model: &PyFinancialModelSpec, graph: &Bound<'_, PyDependencyGraph>) -> Self {
        Self {
            model: model.clone(),
            graph: graph.clone().unbind(),
        }
    }

    #[pyo3(signature = (node_id))]
    /// Get all direct dependencies for a node.
    ///
    /// Parameters
    /// ----------
    /// node_id : str
    ///     Node identifier to inspect
    ///
    /// Returns
    /// -------
    /// list[str]
    ///     Node IDs that are direct dependencies
    fn direct_dependencies(&self, py: Python<'_>, node_id: &str) -> PyResult<Vec<String>> {
        let graph_ref = self.graph.borrow(py);
        let tracer = DependencyTracer::new(&self.model.inner, &graph_ref.inner);
        tracer
            .direct_dependencies(node_id)
            .map(|deps| deps.iter().map(|s| s.to_string()).collect())
            .map_err(stmt_to_py)
    }

    #[pyo3(signature = (node_id))]
    /// Get all transitive dependencies (recursive).
    ///
    /// Parameters
    /// ----------
    /// node_id : str
    ///     Node identifier to inspect
    ///
    /// Returns
    /// -------
    /// list[str]
    ///     All node IDs in dependency order
    fn all_dependencies(&self, py: Python<'_>, node_id: &str) -> PyResult<Vec<String>> {
        let graph_ref = self.graph.borrow(py);
        let tracer = DependencyTracer::new(&self.model.inner, &graph_ref.inner);
        tracer.all_dependencies(node_id).map_err(stmt_to_py)
    }

    #[pyo3(signature = (node_id))]
    /// Get dependency tree as hierarchical structure.
    ///
    /// Parameters
    /// ----------
    /// node_id : str
    ///     Root node for the dependency tree
    ///
    /// Returns
    /// -------
    /// DependencyTree
    ///     Hierarchical dependency structure
    fn dependency_tree(&self, py: Python<'_>, node_id: &str) -> PyResult<PyDependencyTree> {
        let graph_ref = self.graph.borrow(py);
        let tracer = DependencyTracer::new(&self.model.inner, &graph_ref.inner);
        let tree = tracer.dependency_tree(node_id).map_err(stmt_to_py)?;
        Ok(PyDependencyTree { inner: tree })
    }

    #[pyo3(signature = (node_id))]
    /// Get nodes that depend on this node (reverse dependencies).
    ///
    /// Parameters
    /// ----------
    /// node_id : str
    ///     Node identifier to inspect
    ///
    /// Returns
    /// -------
    /// list[str]
    ///     Node IDs that depend on this node
    fn dependents(&self, py: Python<'_>, node_id: &str) -> PyResult<Vec<String>> {
        let graph_ref = self.graph.borrow(py);
        let tracer = DependencyTracer::new(&self.model.inner, &graph_ref.inner);
        tracer
            .dependents(node_id)
            .map(|deps| deps.iter().map(|s| s.to_string()).collect())
            .map_err(stmt_to_py)
    }

    fn __repr__(&self) -> String {
        format!(
            "DependencyTracer(model='{}', nodes={})",
            self.model.inner.id,
            self.model.inner.nodes.len()
        )
    }
}

#[pyfunction]
#[pyo3(signature = (tree))]
/// Render dependency tree as ASCII art.
///
/// Parameters
/// ----------
/// tree : DependencyTree
///     Dependency tree to render
///
/// Returns
/// -------
/// str
///     ASCII representation
fn py_render_tree_ascii(tree: &PyDependencyTree) -> String {
    render_tree_ascii(&tree.inner)
}

#[pyfunction]
#[pyo3(signature = (tree, results, period))]
/// Render dependency tree with values from results.
///
/// Parameters
/// ----------
/// tree : DependencyTree
///     Dependency tree to render
/// results : Results
///     Evaluation results containing node values
/// period : PeriodId
///     Period to display values for
///
/// Returns
/// -------
/// str
///     ASCII tree with values
fn py_render_tree_detailed(
    tree: &PyDependencyTree,
    results: &PyResults,
    period: &crate::core::dates::periods::PyPeriodId,
) -> String {
    render_tree_detailed(&tree.inner, &results.inner, &period.inner)
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(_py, "explain")?;
    module.setattr(
        "__doc__",
        concat!(
            "Node explanation and dependency tracing.\n\n",
            "This module provides tools for understanding how financial statement nodes ",
            "are calculated and what dependencies they have."
        ),
    )?;

    module.add_class::<PyExplanationStep>()?;
    module.add_class::<PyExplanation>()?;
    module.add_class::<PyFormulaExplainer>()?;
    module.add_class::<PyDependencyTree>()?;
    module.add_class::<PyDependencyTracer>()?;
    module.add_function(wrap_pyfunction!(py_render_tree_ascii, &module)?)?;
    module.add_function(wrap_pyfunction!(py_render_tree_detailed, &module)?)?;

    parent.add_submodule(&module)?;
    parent.setattr("explain", &module)?;

    Ok(vec![
        "ExplanationStep",
        "Explanation",
        "FormulaExplainer",
        "DependencyTree",
        "DependencyTracer",
        "render_tree_ascii",
        "render_tree_detailed",
    ])
}

