//! Schema types for metric definitions.

use crate::statements::utils::json_to_py;
use finstack_statements::registry::{MetricDefinition, MetricRegistry, UnitType};
use indexmap::IndexMap;
use pyo3::basic::CompareOp;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict, PyModule, PyType};
use pyo3::{Bound, IntoPyObjectExt};

/// Unit type for metric values.
#[pyclass(module = "finstack.statements.registry", name = "UnitType", frozen)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PyUnitType {
    pub(crate) inner: UnitType,
}

impl PyUnitType {
    pub(crate) fn new(inner: UnitType) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyUnitType {
    #[classattr]
    const PERCENTAGE: Self = Self {
        inner: UnitType::Percentage,
    };
    #[classattr]
    const CURRENCY: Self = Self {
        inner: UnitType::Currency,
    };
    #[classattr]
    const RATIO: Self = Self {
        inner: UnitType::Ratio,
    };
    #[classattr]
    const COUNT: Self = Self {
        inner: UnitType::Count,
    };
    #[classattr]
    const TIME_PERIOD: Self = Self {
        inner: UnitType::TimePeriod,
    };

    fn __repr__(&self) -> String {
        format!("UnitType.{:?}", self.inner)
    }

    fn __richcmp__(
        &self,
        other: PyRef<'_, Self>,
        op: CompareOp,
        py: Python<'_>,
    ) -> PyResult<Py<PyAny>> {
        let result = match op {
            CompareOp::Eq => self.inner == other.inner,
            CompareOp::Ne => self.inner != other.inner,
            _ => return Err(PyValueError::new_err("Unsupported comparison")),
        };
        let py_bool = result.into_bound_py_any(py)?;
        Ok(py_bool.into())
    }
}

/// Individual metric definition.
#[pyclass(module = "finstack.statements.registry", name = "MetricDefinition")]
#[derive(Clone, Debug)]
pub struct PyMetricDefinition {
    pub(crate) inner: MetricDefinition,
}

impl PyMetricDefinition {
    pub(crate) fn new(inner: MetricDefinition) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyMetricDefinition {
    #[new]
    #[pyo3(
        text_signature = "(id, name, formula, description=None, category=None, unit_type=None, requires=None, tags=None)",
        signature = (
            id,
            name,
            formula,
            description = None,
            category = None,
            unit_type = None,
            requires = None,
            tags = None
        )
    )]
    /// Create a metric definition.
    ///
    /// Parameters
    /// ----------
    /// id : str
    ///     Unique identifier within namespace
    /// name : str
    ///     Human-readable name
    /// formula : str
    ///     Formula text in statement DSL
    /// description : str, optional
    ///     Description of what this metric represents
    /// category : str, optional
    ///     Category for grouping (e.g., "margins", "returns")
    /// unit_type : UnitType, optional
    ///     Unit type (percentage, currency, ratio, etc.)
    /// requires : list[str], optional
    ///     List of required node dependencies
    /// tags : list[str], optional
    ///     Tags for filtering/searching
    ///
    /// Returns
    /// -------
    /// MetricDefinition
    ///     Metric definition
    fn new_py(
        id: String,
        name: String,
        formula: String,
        description: Option<String>,
        category: Option<String>,
        unit_type: Option<PyUnitType>,
        requires: Option<Vec<String>>,
        tags: Option<Vec<String>>,
    ) -> Self {
        Self::new(MetricDefinition {
            id,
            name,
            formula,
            description,
            category,
            unit_type: unit_type.map(|u| u.inner),
            requires: requires.unwrap_or_default(),
            tags: tags.unwrap_or_default(),
            meta: IndexMap::new(),
        })
    }

    #[getter]
    fn id(&self) -> String {
        self.inner.id.clone()
    }

    #[getter]
    fn name(&self) -> String {
        self.inner.name.clone()
    }

    #[getter]
    fn formula(&self) -> String {
        self.inner.formula.clone()
    }

    #[getter]
    fn description(&self) -> Option<String> {
        self.inner.description.clone()
    }

    #[getter]
    fn category(&self) -> Option<String> {
        self.inner.category.clone()
    }

    #[getter]
    fn unit_type(&self) -> Option<PyUnitType> {
        self.inner.unit_type.map(PyUnitType::new)
    }

    #[getter]
    fn requires(&self) -> Vec<String> {
        self.inner.requires.clone()
    }

    #[getter]
    fn tags(&self) -> Vec<String> {
        self.inner.tags.clone()
    }

    #[getter]
    fn meta(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let dict = PyDict::new(py);
        for (key, value) in &self.inner.meta {
            dict.set_item(key, json_to_py(py, value)?)?;
        }
        Ok(dict.into())
    }

    /// Convert to JSON string.
    ///
    /// Returns
    /// -------
    /// str
    ///     JSON representation
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner)
            .map_err(|e| PyValueError::new_err(format!("Serialization error: {}", e)))
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, json_str)")]
    /// Create from JSON string.
    ///
    /// Parameters
    /// ----------
    /// json_str : str
    ///     JSON string
    ///
    /// Returns
    /// -------
    /// MetricDefinition
    ///     Deserialized metric definition
    fn from_json(_cls: &Bound<'_, PyType>, json_str: &str) -> PyResult<Self> {
        serde_json::from_str(json_str)
            .map(Self::new)
            .map_err(|e| PyValueError::new_err(format!("Deserialization error: {}", e)))
    }

    fn __repr__(&self) -> String {
        format!(
            "MetricDefinition(id='{}', name='{}')",
            self.inner.id, self.inner.name
        )
    }
}

/// Top-level metric registry schema.
#[pyclass(module = "finstack.statements.registry", name = "MetricRegistry")]
#[derive(Clone, Debug)]
pub struct PyMetricRegistry {
    pub(crate) inner: MetricRegistry,
}

impl PyMetricRegistry {
    pub(crate) fn new(inner: MetricRegistry) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyMetricRegistry {
    #[new]
    #[pyo3(text_signature = "(namespace, metrics, schema_version=1)")]
    /// Create a metric registry.
    ///
    /// Parameters
    /// ----------
    /// namespace : str
    ///     Namespace for all metrics (e.g., "fin", "custom")
    /// metrics : list[MetricDefinition]
    ///     List of metric definitions
    /// schema_version : int, optional
    ///     Schema version (default: 1)
    ///
    /// Returns
    /// -------
    /// MetricRegistry
    ///     Registry
    fn new_py(
        namespace: String,
        metrics: Vec<PyMetricDefinition>,
        schema_version: Option<u32>,
    ) -> Self {
        Self::new(MetricRegistry {
            namespace,
            schema_version: schema_version.unwrap_or(1),
            metrics: metrics.into_iter().map(|m| m.inner).collect(),
            meta: IndexMap::new(),
        })
    }

    #[getter]
    fn namespace(&self) -> String {
        self.inner.namespace.clone()
    }

    #[getter]
    fn schema_version(&self) -> u32 {
        self.inner.schema_version
    }

    #[getter]
    fn metrics(&self) -> Vec<PyMetricDefinition> {
        self.inner
            .metrics
            .iter()
            .map(|m| PyMetricDefinition::new(m.clone()))
            .collect()
    }

    #[getter]
    fn meta(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let dict = PyDict::new(py);
        for (key, value) in &self.inner.meta {
            dict.set_item(key, json_to_py(py, value)?)?;
        }
        Ok(dict.into())
    }

    /// Convert to JSON string.
    ///
    /// Returns
    /// -------
    /// str
    ///     JSON representation
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner)
            .map_err(|e| PyValueError::new_err(format!("Serialization error: {}", e)))
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, json_str)")]
    /// Create from JSON string.
    ///
    /// Parameters
    /// ----------
    /// json_str : str
    ///     JSON string
    ///
    /// Returns
    /// -------
    /// MetricRegistry
    ///     Deserialized registry
    fn from_json(_cls: &Bound<'_, PyType>, json_str: &str) -> PyResult<Self> {
        serde_json::from_str(json_str)
            .map(Self::new)
            .map_err(|e| PyValueError::new_err(format!("Deserialization error: {}", e)))
    }

    fn __repr__(&self) -> String {
        format!(
            "MetricRegistry(namespace='{}', metrics={})",
            self.inner.namespace,
            self.inner.metrics.len()
        )
    }
}

pub(crate) fn register<'py>(_py: Python<'py>, module: &Bound<'py, PyModule>) -> PyResult<()> {
    module.add_class::<PyUnitType>()?;
    module.add_class::<PyMetricDefinition>()?;
    module.add_class::<PyMetricRegistry>()?;
    Ok(())
}
