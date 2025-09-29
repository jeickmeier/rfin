use crate::valuations::common::{instrument_type_label, InstrumentTypeArg};
use finstack_valuations::metrics::{standard_registry, MetricId, MetricRegistry};
use pyo3::basic::CompareOp;
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule, PyType};
use pyo3::{Bound, PyObject, PyRef};
use std::fmt;

/// Strongly-typed metric identifier mirroring `finstack_valuations::metrics::MetricId`.
#[pyclass(module = "finstack.valuations.metrics", name = "MetricId", frozen)]
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
    fn from_name(_cls: &Bound<'_, PyType>, name: &str) -> Self {
        Self::new(name.parse().unwrap())
    }

    /// Snake-case name of the metric.
    #[getter]
    fn name(&self) -> &str {
        self.inner.as_str()
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls)")]
    /// List of all standard metric identifiers bundled with finstack.
    fn standard_names(_cls: &Bound<'_, PyType>, py: Python<'_>) -> PyResult<PyObject> {
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
    ) -> PyResult<PyObject> {
        let rhs = if let Ok(value) = other.extract::<PyRef<Self>>() {
            Some(value.inner.clone())
        } else {
            None
        };
        crate::core::common::pycmp::richcmp_eq_ne(py, &self.inner, rhs, op)
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
            return Ok(MetricIdArg(name.parse().unwrap()));
        }
        Err(PyTypeError::new_err(
            "Expected MetricId or snake-case metric identifier",
        ))
    }
}

/// Registry of metric calculators with applicability filtering.
#[pyclass(
    module = "finstack.valuations.metrics",
    name = "MetricRegistry",
    frozen
)]
#[derive(Clone, Default)]
pub struct PyMetricRegistry {
    pub(crate) inner: MetricRegistry,
}

impl PyMetricRegistry {
    pub(crate) fn new(inner: MetricRegistry) -> Self {
        Self { inner }
    }

    fn metric_ids_to_list<'py>(&self, py: Python<'py>, ids: Vec<MetricId>) -> PyResult<PyObject> {
        let wrapped: Vec<PyMetricId> = ids.into_iter().map(PyMetricId::new).collect();
        Ok(PyList::new(py, wrapped)?.into())
    }
}

#[pymethods]
impl PyMetricRegistry {
    #[new]
    #[pyo3(text_signature = "()")]
    /// Create an empty registry; use :meth:`standard` to include built-in metrics.
    fn ctor() -> Self {
        Self::new(MetricRegistry::new())
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls)")]
    /// Registry populated with all finstack standard metrics.
    fn standard(_cls: &Bound<'_, PyType>) -> Self {
        Self::new(standard_registry())
    }

    #[pyo3(text_signature = "(self)")]
    /// All metric identifiers currently registered.
    fn available_metrics(&self, py: Python<'_>) -> PyResult<PyObject> {
        let ids = self.inner.available_metrics();
        self.metric_ids_to_list(py, ids)
    }

    #[pyo3(text_signature = "(self, instrument_type)")]
    /// Metrics applicable to the supplied instrument type.
    fn metrics_for_instrument(
        &self,
        py: Python<'_>,
        instrument_type: Bound<'_, PyAny>,
    ) -> PyResult<PyObject> {
        let InstrumentTypeArg(inst) = instrument_type.extract()?;
        let metrics = self
            .inner
            .metrics_for_instrument(instrument_type_label(inst));
        self.metric_ids_to_list(py, metrics)
    }

    #[pyo3(text_signature = "(self, metric, instrument_type)")]
    /// Test whether ``metric`` applies to the provided instrument type.
    fn is_applicable(
        &self,
        metric: Bound<'_, PyAny>,
        instrument_type: Bound<'_, PyAny>,
    ) -> PyResult<bool> {
        let MetricIdArg(metric_id) = metric.extract()?;
        let InstrumentTypeArg(inst) = instrument_type.extract()?;
        Ok(self
            .inner
            .is_applicable(&metric_id, instrument_type_label(inst)))
    }

    #[pyo3(text_signature = "(self, metric)")]
    /// Return ``True`` when the registry contains ``metric``.
    fn has_metric(&self, metric: Bound<'_, PyAny>) -> PyResult<bool> {
        let MetricIdArg(metric_id) = metric.extract()?;
        Ok(self.inner.has_metric(metric_id))
    }

    #[pyo3(text_signature = "(self)")]
    /// Clone the registry for experimentation without mutating the original.
    fn clone(&self) -> Self {
        // Underlying registry derives Clone; return a shallow clone wrapper
        Self::new(self.inner.clone())
    }
}

impl fmt::Display for PyMetricRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "MetricRegistry(metrics={})",
            self.inner.available_metrics().len()
        )
    }
}

pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "metrics")?;
    module.setattr(
        "__doc__",
        "Metric identifiers and registry helpers for finstack valuations.",
    )?;
    module.add_class::<PyMetricId>()?;
    module.add_class::<PyMetricRegistry>()?;
    let exports = ["MetricId", "MetricRegistry"];
    module.setattr("__all__", PyList::new(py, &exports)?)?;
    parent.add_submodule(&module)?;
    Ok(exports.to_vec())
}
