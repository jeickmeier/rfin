use crate::core::common::pycmp::richcmp_eq_ne;
use finstack_valuations::metrics::MetricId;
use pyo3::basic::CompareOp;
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule, PyType};
use pyo3::{Bound, Py, PyAny, PyRef};
use std::fmt;

/// Strongly typed metric identifier mirroring ``finstack_valuations::metrics::MetricId``.
///
/// Examples:
///     >>> MetricId.from_name("pv")
///     MetricId('pv')
#[pyclass(module = "finstack.valuations.metrics.ids", name = "MetricId", frozen)]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PyMetricId {
    pub(crate) inner: MetricId,
}

impl PyMetricId {
    pub(crate) fn new(inner: MetricId) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyMetricId {
    #[classmethod]
    #[pyo3(text_signature = "(cls, name)")]
    /// Parse a metric identifier, falling back to a custom metric when unknown.
    ///
    /// .. warning::
    ///     This method accepts unknown metric names and creates custom metrics.
    ///     For strict validation of user inputs, use :meth:`parse_strict` instead.
    ///
    /// Args:
    ///     name: Metric label such as ``"pv"`` or ``"dv01"``.
    ///
    /// Returns:
    ///     MetricId: Identifier corresponding to ``name``.
    ///
    /// Examples:
    ///     >>> MetricId.from_name("dv01").name
    ///     'dv01'
    ///     >>> # Unknown names create custom metrics (permissive):
    ///     >>> MetricId.from_name("my_custom_metric").name
    ///     'my_custom_metric'
    fn from_name(_cls: &Bound<'_, PyType>, name: &str) -> PyResult<Self> {
        let metric: MetricId = name.parse().unwrap_or_else(|_| unreachable!());
        Ok(Self::new(metric))
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, name)")]
    /// Parse a metric identifier strictly, rejecting unknown metric names.
    ///
    /// This method validates that the metric name is a known standard metric.
    /// Use this for user inputs, configuration files, and external APIs where
    /// unknown metrics should be rejected with a clear error.
    ///
    /// Args:
    ///     name: Metric label such as ``"pv"`` or ``"dv01"``.
    ///
    /// Returns:
    ///     MetricId: Identifier corresponding to ``name``.
    ///
    /// Raises:
    ///     ValueError: If the metric name is not a known standard metric.
    ///         The error includes a list of all available metrics.
    ///
    /// Examples:
    ///     >>> # Known metrics parse successfully:
    ///     >>> MetricId.parse_strict("dv01").name
    ///     'dv01'
    ///
    ///     >>> # Unknown metrics raise ValueError:
    ///     >>> try:
    ///     ...     MetricId.parse_strict("unknown_metric")
    ///     ... except ValueError as e:
    ///     ...     print("Caught error:", str(e))
    ///     Caught error: Unknown metric 'unknown_metric'...
    ///
    ///     >>> # Migration from from_name:
    ///     >>> # OLD (permissive):
    ///     >>> metric = MetricId.from_name(user_input)
    ///     >>> # NEW (strict - recommended for user inputs):
    ///     >>> metric = MetricId.parse_strict(user_input)
    fn parse_strict(_cls: &Bound<'_, PyType>, name: &str) -> PyResult<Self> {
        MetricId::parse_strict(name)
            .map(Self::new)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    /// Snake-case name of the metric.
    ///
    /// Returns:
    ///     str: Metric label, e.g., ``"pv"``.
    #[getter]
    fn name(&self) -> &str {
        self.inner.as_str()
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls)")]
    /// List of all standard metric identifiers bundled with finstack.
    ///
    /// Returns:
    ///     list[str]: Collection of built-in metric labels.
    ///
    /// Examples:
    ///     >>> "pv" in MetricId.standard_names()
    ///     True
    fn standard_names(_cls: &Bound<'_, PyType>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let names: Vec<&str> = MetricId::ALL_STANDARD
            .iter()
            .map(MetricId::as_str)
            .collect();
        Ok(PyList::new(py, names)?.into())
    }

    fn __repr__(&self) -> String {
        format!("MetricId('{}')", self.inner.as_str())
    }

    fn __str__(&self) -> &str {
        self.inner.as_str()
    }

    fn __hash__(&self) -> isize {
        use std::hash::{Hash, Hasher};
        let mut state = std::collections::hash_map::DefaultHasher::new();
        self.inner.hash(&mut state);
        state.finish() as isize
    }

    fn __richcmp__(
        &self,
        other: Bound<'_, PyAny>,
        op: CompareOp,
        py: Python<'_>,
    ) -> PyResult<Py<PyAny>> {
        let rhs = if let Ok(value) = other.extract::<PyRef<Self>>() {
            Some(value.inner.clone())
        } else {
            None
        };
        richcmp_eq_ne(py, &self.inner, rhs, op)
    }
}

impl fmt::Display for PyMetricId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.inner.as_str())
    }
}

/// Argument parser accepting :class:`MetricId` instances or snake-case strings.
#[derive(Clone, Debug)]
pub(crate) struct MetricIdArg(pub MetricId);

impl<'py> FromPyObject<'py> for MetricIdArg {
    fn extract_bound(ob: &Bound<'py, PyAny>) -> PyResult<Self> {
        if let Ok(wrapper) = ob.extract::<PyRef<PyMetricId>>() {
            return Ok(MetricIdArg(wrapper.inner.clone()));
        }
        if let Ok(name) = ob.extract::<&str>() {
            let metric: MetricId = name.parse().unwrap_or_else(|_| unreachable!());
            return Ok(MetricIdArg(metric));
        }
        Err(PyTypeError::new_err(
            "Expected MetricId or snake-case metric identifier",
        ))
    }
}

pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "ids")?;
    module.setattr("__doc__", "Metric identifiers for finstack valuations.")?;
    module.add_class::<PyMetricId>()?;
    let exports = ["MetricId"];
    module.setattr("__all__", PyList::new(py, exports)?)?;
    parent.add_submodule(&module)?;
    Ok(exports.to_vec())
}
