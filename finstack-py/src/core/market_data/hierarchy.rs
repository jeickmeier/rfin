//! Python bindings for the market data hierarchy tree.
//!
//! Provides `HierarchyNode`, `MarketDataHierarchy`, `CompletenessReport`, and
//! `SubtreeCoverage` wrappers that expose the Rust hierarchy API to Python.

use crate::errors::core_to_py;
use finstack_core::market_data::hierarchy::{
    CompletenessReport, HierarchyNode, MarketDataHierarchy, SubtreeCoverage,
};
use finstack_core::types::CurveId;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyModule};
use pyo3::Bound;

// ---------------------------------------------------------------------------
// PyHierarchyNode
// ---------------------------------------------------------------------------

/// A single node in the market data hierarchy tree.
///
/// Nodes form a tree: each has a name, optional key-value tags for cross-cutting
/// queries, ordered children, and leaf CurveId references.
///
/// Parameters
/// ----------
/// name : str
///     Node display name.
#[pyclass(
    module = "finstack.core.market_data.hierarchy",
    name = "HierarchyNode",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyHierarchyNode {
    pub(crate) inner: HierarchyNode,
}

#[pymethods]
impl PyHierarchyNode {
    /// Create a new hierarchy node with the given name.
    ///
    /// Parameters
    /// ----------
    /// name : str
    ///     Display name for the node.
    #[new]
    #[pyo3(text_signature = "(name)")]
    fn new(name: &str) -> Self {
        Self {
            inner: HierarchyNode::new(name),
        }
    }

    /// Node display name.
    #[getter]
    fn name(&self) -> &str {
        self.inner.name()
    }

    /// Tags attached to this node (key-value metadata).
    ///
    /// Returns
    /// -------
    /// dict[str, str]
    #[getter]
    fn tags(&self, py: Python<'_>) -> PyResult<Py<PyDict>> {
        let dict = PyDict::new(py);
        for (k, v) in self.inner.tags() {
            dict.set_item(k, v)?;
        }
        Ok(dict.into())
    }

    /// Ordered child nodes.
    ///
    /// Returns
    /// -------
    /// dict[str, HierarchyNode]
    #[getter]
    fn children(&self) -> PyResult<std::collections::HashMap<String, PyHierarchyNode>> {
        let mut map = std::collections::HashMap::new();
        for (name, node) in self.inner.children() {
            map.insert(
                name.clone(),
                PyHierarchyNode {
                    inner: node.clone(),
                },
            );
        }
        Ok(map)
    }

    /// Leaf CurveId references at this node.
    ///
    /// Returns
    /// -------
    /// list[str]
    #[getter]
    fn curve_ids(&self) -> Vec<String> {
        self.inner
            .curve_ids()
            .iter()
            .map(|id| id.as_str().to_string())
            .collect()
    }

    /// Collect all CurveIds in this subtree (this node + all descendants).
    ///
    /// Returns
    /// -------
    /// list[str]
    #[pyo3(text_signature = "($self)")]
    fn all_curve_ids(&self) -> Vec<String> {
        self.inner
            .all_curve_ids()
            .into_iter()
            .map(|id| id.as_str().to_string())
            .collect()
    }

    fn __repr__(&self) -> String {
        let n_children = self.inner.children().len();
        let n_curves = self.inner.curve_ids().len();
        format!(
            "HierarchyNode(name='{}', children={}, curves={})",
            self.inner.name(),
            n_children,
            n_curves
        )
    }
}

// ---------------------------------------------------------------------------
// PyMarketDataHierarchy
// ---------------------------------------------------------------------------

/// The top-level market data hierarchy containing root nodes.
///
/// Each root represents a major asset class or category (e.g., "Rates", "Credit",
/// "FX", "Equity", "Volatility"). The hierarchy supports insertion, removal, and
/// validation of curve references.
#[pyclass(
    module = "finstack.core.market_data.hierarchy",
    name = "MarketDataHierarchy",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyMarketDataHierarchy {
    pub(crate) inner: MarketDataHierarchy,
}

#[pymethods]
impl PyMarketDataHierarchy {
    /// Create an empty hierarchy.
    #[new]
    #[pyo3(text_signature = "()")]
    fn new() -> Self {
        Self {
            inner: MarketDataHierarchy::new(),
        }
    }

    /// Insert a curve at a ``/``-separated path, creating intermediate nodes as needed.
    ///
    /// Parameters
    /// ----------
    /// path : str
    ///     Slash-separated path (e.g., ``"Rates/USD/OIS"``).
    /// curve_id : str
    ///     The curve identifier to insert at the leaf node.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If the path is malformed or the curve_id already exists elsewhere.
    #[pyo3(text_signature = "($self, path, curve_id)")]
    fn insert_curve(&mut self, path: &str, curve_id: &str) -> PyResult<()> {
        self.inner
            .insert_curve(path, CurveId::new(curve_id))
            .map_err(core_to_py)
    }

    /// Remove a curve from wherever it sits in the tree.
    ///
    /// Parameters
    /// ----------
    /// curve_id : str
    ///     The curve identifier to remove.
    ///
    /// Returns
    /// -------
    /// bool
    ///     ``True`` if the curve was found and removed.
    #[pyo3(text_signature = "($self, curve_id)")]
    fn remove_curve(&mut self, curve_id: &str) -> bool {
        self.inner.remove_curve(&CurveId::new(curve_id))
    }

    /// Find the path from root to a specific curve.
    ///
    /// Parameters
    /// ----------
    /// curve_id : str
    ///     The curve identifier to locate.
    ///
    /// Returns
    /// -------
    /// list[str] | None
    ///     Path segments from root to the node containing the curve, or ``None``
    ///     if not found.
    #[pyo3(text_signature = "($self, curve_id)")]
    fn path_for_curve(&self, curve_id: &str) -> Option<Vec<String>> {
        self.inner.path_for_curve(&CurveId::new(curve_id))
    }

    /// Collect all CurveIds across the entire hierarchy.
    ///
    /// Returns
    /// -------
    /// list[str]
    #[pyo3(text_signature = "($self)")]
    fn all_curve_ids(&self) -> Vec<String> {
        self.inner
            .all_curve_ids()
            .into_iter()
            .map(|id| id.as_str().to_string())
            .collect()
    }

    /// Validate structural hierarchy invariants.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If node names are empty, mismatched, or duplicate CurveIds exist.
    #[pyo3(text_signature = "($self)")]
    fn validate(&self) -> PyResult<()> {
        self.inner.validate().map_err(core_to_py)
    }

    /// Root nodes of the hierarchy.
    ///
    /// Returns
    /// -------
    /// dict[str, HierarchyNode]
    #[getter]
    fn roots(&self) -> PyResult<std::collections::HashMap<String, PyHierarchyNode>> {
        let mut map = std::collections::HashMap::new();
        for (name, node) in self.inner.roots() {
            map.insert(
                name.clone(),
                PyHierarchyNode {
                    inner: node.clone(),
                },
            );
        }
        Ok(map)
    }

    fn __repr__(&self) -> String {
        let n_roots = self.inner.roots().len();
        let n_curves = self.inner.all_curve_ids().len();
        format!(
            "MarketDataHierarchy(roots={}, total_curves={})",
            n_roots, n_curves
        )
    }
}

// ---------------------------------------------------------------------------
// PySubtreeCoverage
// ---------------------------------------------------------------------------

/// Coverage statistics for a single subtree.
///
/// Attributes
/// ----------
/// path : list[str]
///     Path to the subtree root.
/// total_expected : int
///     Number of CurveIds declared in this subtree.
/// total_present : int
///     Number of those CurveIds that are present in MarketContext.
/// percent : float
///     Coverage percentage (0.0 -- 100.0).
#[pyclass(
    module = "finstack.core.market_data.hierarchy",
    name = "SubtreeCoverage",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PySubtreeCoverage {
    pub(crate) inner: SubtreeCoverage,
}

#[pymethods]
impl PySubtreeCoverage {
    /// Path to the subtree root.
    #[getter]
    fn path(&self) -> Vec<String> {
        self.inner.path.clone()
    }

    /// Number of CurveIds declared in this subtree.
    #[getter]
    fn total_expected(&self) -> usize {
        self.inner.total_expected
    }

    /// Number of those CurveIds present in MarketContext.
    #[getter]
    fn total_present(&self) -> usize {
        self.inner.total_present
    }

    /// Coverage percentage (0.0 -- 100.0).
    #[getter]
    fn percent(&self) -> f64 {
        self.inner.percent
    }

    fn __repr__(&self) -> String {
        format!(
            "SubtreeCoverage(path={:?}, expected={}, present={}, percent={:.1}%)",
            self.inner.path,
            self.inner.total_expected,
            self.inner.total_present,
            self.inner.percent
        )
    }
}

// ---------------------------------------------------------------------------
// PyCompletenessReport
// ---------------------------------------------------------------------------

/// Report comparing hierarchy-declared CurveIds against what exists in MarketContext.
///
/// Attributes
/// ----------
/// missing : list[tuple[list[str], str]]
///     CurveIds declared in hierarchy but missing from MarketContext.
/// unclassified : list[str]
///     CurveIds in MarketContext that are not in any hierarchy node.
/// coverage : list[SubtreeCoverage]
///     Per-subtree coverage statistics.
#[pyclass(
    module = "finstack.core.market_data.hierarchy",
    name = "CompletenessReport",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyCompletenessReport {
    pub(crate) inner: CompletenessReport,
}

#[pymethods]
impl PyCompletenessReport {
    /// CurveIds declared in hierarchy but missing from MarketContext.
    ///
    /// Returns
    /// -------
    /// list[tuple[list[str], str]]
    #[getter]
    fn missing(&self) -> Vec<(Vec<String>, String)> {
        self.inner
            .missing
            .iter()
            .map(|(path, curve_id)| (path.clone(), curve_id.as_str().to_string()))
            .collect()
    }

    /// CurveIds in MarketContext that are not in any hierarchy node.
    ///
    /// Returns
    /// -------
    /// list[str]
    #[getter]
    fn unclassified(&self) -> Vec<String> {
        self.inner
            .unclassified
            .iter()
            .map(|id| id.as_str().to_string())
            .collect()
    }

    /// Per-subtree coverage statistics.
    ///
    /// Returns
    /// -------
    /// list[SubtreeCoverage]
    #[getter]
    fn coverage(&self) -> Vec<PySubtreeCoverage> {
        self.inner
            .coverage
            .iter()
            .map(|c| PySubtreeCoverage { inner: c.clone() })
            .collect()
    }

    fn __repr__(&self) -> String {
        format!(
            "CompletenessReport(missing={}, unclassified={}, coverage_entries={})",
            self.inner.missing.len(),
            self.inner.unclassified.len(),
            self.inner.coverage.len()
        )
    }
}

// ---------------------------------------------------------------------------
// Module registration
// ---------------------------------------------------------------------------

pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "hierarchy")?;
    module.setattr(
        "__doc__",
        "Market data hierarchy for organizational grouping of curves.",
    )?;
    module.add_class::<PyHierarchyNode>()?;
    module.add_class::<PyMarketDataHierarchy>()?;
    module.add_class::<PyCompletenessReport>()?;
    module.add_class::<PySubtreeCoverage>()?;

    let exports = [
        "HierarchyNode",
        "MarketDataHierarchy",
        "CompletenessReport",
        "SubtreeCoverage",
    ];
    module.setattr("__all__", PyList::new(py, exports)?)?;
    parent.add_submodule(&module)?;
    let _ = py;
    Ok(exports.to_vec())
}
