//! Registry system for dynamic metrics.

mod schema;

pub use schema::{PyMetricDefinition, PyMetricRegistry};

use crate::statements::error::stmt_to_py;
use finstack_statements::registry::{AliasRegistry, Registry};
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule, PyType};
use pyo3::Bound;

/// Dynamic metric registry.
///
/// Allows loading reusable metric definitions from JSON files,
/// enabling analysts to define standard financial metrics without recompiling.
#[pyclass(module = "finstack.statements.registry", name = "Registry", unsendable)]
pub struct PyRegistry {
    inner: Registry,
}

#[pymethods]
impl PyRegistry {
    #[classmethod]
    #[pyo3(text_signature = "(cls)")]
    /// Create a new registry.
    ///
    /// Returns
    /// -------
    /// Registry
    ///     Registry instance
    fn new(_cls: &Bound<'_, PyType>) -> Self {
        Self {
            inner: Registry::new(),
        }
    }

    #[pyo3(text_signature = "(self)")]
    /// Load built-in metrics (fin.* namespace).
    ///
    /// Returns
    /// -------
    /// None
    fn load_builtins(&mut self) -> PyResult<()> {
        self.inner.load_builtins().map_err(stmt_to_py)
    }

    #[pyo3(text_signature = "(self, path)")]
    /// Load metrics from a JSON file.
    ///
    /// Parameters
    /// ----------
    /// path : str
    ///     Path to JSON registry file
    ///
    /// Returns
    /// -------
    /// None
    fn load_from_json(&mut self, path: &str) -> PyResult<()> {
        self.inner.load_from_json(path).map_err(stmt_to_py)
    }

    #[pyo3(text_signature = "(self, json_str)")]
    /// Load metrics from a JSON string.
    ///
    /// Parameters
    /// ----------
    /// json_str : str
    ///     JSON string containing metric registry
    ///
    /// Returns
    /// -------
    /// MetricRegistry
    ///     Loaded registry
    fn load_from_json_str(&mut self, json_str: &str) -> PyResult<PyMetricRegistry> {
        let registry = self
            .inner
            .load_from_json_str(json_str)
            .map_err(stmt_to_py)?;
        Ok(PyMetricRegistry::new(registry))
    }

    #[pyo3(text_signature = "(self, metric_id)")]
    /// Get a metric definition by ID.
    ///
    /// Parameters
    /// ----------
    /// metric_id : str
    ///     Metric identifier (e.g., "fin.gross_margin")
    ///
    /// Returns
    /// -------
    /// MetricDefinition
    ///     Metric definition
    fn get(&self, metric_id: &str) -> PyResult<PyMetricDefinition> {
        self.inner
            .get(metric_id)
            .map(|stored| PyMetricDefinition::new(stored.definition.clone()))
            .map_err(stmt_to_py)
    }

    #[pyo3(text_signature = "(self, namespace=None)")]
    /// List available metrics.
    ///
    /// Parameters
    /// ----------
    /// namespace : str, optional
    ///     Filter by namespace (e.g., "fin")
    ///
    /// Returns
    /// -------
    /// list[str]
    ///     List of metric IDs
    #[pyo3(signature = (namespace = None))]
    fn list_metrics(&self, namespace: Option<&str>) -> Vec<String> {
        if let Some(ns) = namespace {
            self.inner
                .namespace(ns)
                .map(|(id, _)| id.to_string())
                .collect()
        } else {
            self.inner
                .all_metrics()
                .map(|(id, _)| id.to_string())
                .collect()
        }
    }

    #[pyo3(text_signature = "(self, metric_id)")]
    /// Check if a metric exists.
    ///
    /// Parameters
    /// ----------
    /// metric_id : str
    ///     Metric identifier
    ///
    /// Returns
    /// -------
    /// bool
    ///     True if metric exists
    fn has_metric(&self, metric_id: &str) -> bool {
        self.inner.has(metric_id)
    }

    fn __repr__(&self) -> String {
        format!("Registry(metrics={})", self.inner.len())
    }
}

/// Alias registry for normalizing user-facing metric names.
#[pyclass(
    module = "finstack.statements.registry",
    name = "AliasRegistry",
    unsendable
)]
pub struct PyAliasRegistry {
    inner: AliasRegistry,
}

#[pymethods]
impl PyAliasRegistry {
    #[classmethod]
    fn new(_cls: &Bound<'_, PyType>) -> Self {
        Self {
            inner: AliasRegistry::new(),
        }
    }

    #[classmethod]
    #[pyo3(signature = (fuzzy_threshold))]
    fn with_threshold(_cls: &Bound<'_, PyType>, fuzzy_threshold: f64) -> Self {
        Self {
            inner: AliasRegistry::with_threshold(fuzzy_threshold),
        }
    }

    fn add_alias(&mut self, alias: &str, canonical: &str) {
        self.inner.add_alias(alias, canonical);
    }

    fn add_aliases(&mut self, canonical: &str, aliases: Vec<String>) {
        self.inner.add_aliases(canonical, aliases);
    }

    fn normalize(&self, input: &str) -> Option<String> {
        self.inner.normalize(input)
    }

    fn load_standard_aliases(&mut self) {
        self.inner.load_standard_aliases();
    }

    fn __len__(&self) -> usize {
        self.inner.len()
    }

    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    fn __repr__(&self) -> String {
        format!("AliasRegistry(aliases={})", self.inner.len())
    }
}

impl PyRegistry {
    /// Get a reference to the inner Registry (for internal use only)
    pub(crate) fn inner(&self) -> &Registry {
        &self.inner
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(_py, "registry")?;
    module.setattr("__doc__", "Dynamic metric registry system.")?;

    schema::register(_py, &module)?;
    module.add_class::<PyAliasRegistry>()?;
    module.add_class::<PyRegistry>()?;

    let exports = vec![
        "AliasRegistry",
        "Registry",
        "MetricDefinition",
        "MetricRegistry",
        "UnitType",
    ];
    module.setattr("__all__", PyList::new(_py, &exports)?)?;
    parent.add_submodule(&module)?;
    parent.setattr("registry", &module)?;

    Ok(exports)
}
