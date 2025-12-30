use crate::valuations::common::{instrument_type_label, InstrumentTypeArg};
use crate::valuations::metrics::ids::{MetricIdArg, PyMetricId};
use finstack_valuations::metrics::{standard_registry, MetricId, MetricRegistry};
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule, PyType};
use pyo3::{Bound, Py, PyAny};
use std::fmt;

/// Registry of metric calculators with applicability filtering.
///
/// Examples:
///     >>> registry = MetricRegistry.standard()
///     >>> registry.has_metric("pv")
///     True
#[pyclass(
    module = "finstack.valuations.metrics.registry",
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

    fn metric_ids_to_list<'py>(&self, py: Python<'py>, ids: Vec<MetricId>) -> PyResult<Py<PyAny>> {
        let wrapped: Vec<PyMetricId> = ids.into_iter().map(PyMetricId::new).collect();
        Ok(PyList::new(py, wrapped)?.into())
    }
}

#[pymethods]
impl PyMetricRegistry {
    #[new]
    #[pyo3(text_signature = "()")]
    /// Create an empty registry instance.
    ///
    /// Returns:
    ///     MetricRegistry: Registry without pre-registered metrics.
    ///
    /// Examples:
    ///     >>> custom = MetricRegistry()
    ///     >>> custom.available_metrics()
    ///     []
    fn ctor() -> Self {
        Self::new(MetricRegistry::new())
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls)")]
    /// Create a registry populated with all finstack standard metrics.
    ///
    /// Returns:
    ///     MetricRegistry: Registry containing the default metric set.
    ///
    /// Examples:
    ///     >>> MetricRegistry.standard().has_metric("pv")
    ///     True
    fn standard(_cls: &Bound<'_, PyType>) -> Self {
        Self::new(standard_registry().clone())
    }

    #[pyo3(text_signature = "(self)")]
    /// All metric identifiers currently registered.
    ///
    /// Returns:
    ///     list[MetricId]: Wrapped metric identifiers known to the registry.
    fn available_metrics(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let ids = self.inner.available_metrics();
        self.metric_ids_to_list(py, ids)
    }

    #[pyo3(text_signature = "(self, instrument_type)")]
    /// Metrics applicable to the supplied instrument type.
    ///
    /// Args:
    ///     instrument_type: Instrument type enumeration or label.
    ///
    /// Returns:
    ///     list[MetricId]: Metrics that can be computed for the instrument.
    ///
    /// Raises:
    ///     ValueError: If the instrument label cannot be parsed.
    fn metrics_for_instrument(
        &self,
        py: Python<'_>,
        instrument_type: Bound<'_, PyAny>,
    ) -> PyResult<Py<PyAny>> {
        let InstrumentTypeArg(inst) = instrument_type.extract()?;
        let _label = instrument_type_label(inst);
        let metrics = self.inner.metrics_for_instrument(inst);
        self.metric_ids_to_list(py, metrics)
    }

    #[pyo3(text_signature = "(self, metric, instrument_type)")]
    /// Test whether ``metric`` applies to the provided instrument type.
    ///
    /// Args:
    ///     metric: Metric identifier or label.
    ///     instrument_type: Instrument type enumeration or label.
    ///
    /// Returns:
    ///     bool: ``True`` when the metric supports the instrument type.
    ///
    /// Raises:
    ///     ValueError: If either argument cannot be parsed.
    fn is_applicable(
        &self,
        metric: Bound<'_, PyAny>,
        instrument_type: Bound<'_, PyAny>,
    ) -> PyResult<bool> {
        let MetricIdArg(metric_id) = metric.extract()?;
        let InstrumentTypeArg(inst) = instrument_type.extract()?;
        let _label = instrument_type_label(inst);
        Ok(self.inner.is_applicable(&metric_id, inst))
    }

    #[pyo3(text_signature = "(self, metric)")]
    /// Determine whether the registry contains ``metric``.
    ///
    /// Args:
    ///     metric: Metric identifier or snake-case label.
    ///
    /// Returns:
    ///     bool: ``True`` when the metric is registered.
    ///
    /// Raises:
    ///     ValueError: If the metric cannot be parsed.
    fn has_metric(&self, metric: Bound<'_, PyAny>) -> PyResult<bool> {
        let MetricIdArg(metric_id) = metric.extract()?;
        Ok(self.inner.has_metric(metric_id))
    }

    #[pyo3(text_signature = "(self)")]
    /// Clone the registry for experimentation without mutating the original.
    ///
    /// Returns:
    ///     MetricRegistry: Shallow clone of the current registry.
    ///
    /// Examples:
    ///     >>> cloned = MetricRegistry.standard().clone()
    ///     >>> cloned.has_metric("pv")
    ///     True
    fn clone(&self) -> Self {
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
    let module = PyModule::new(py, "registry")?;
    module.setattr(
        "__doc__",
        "Metric registry utilities for finstack valuations.",
    )?;
    module.add_class::<PyMetricRegistry>()?;
    let exports = ["MetricRegistry"];
    module.setattr("__all__", PyList::new(py, &exports)?)?;
    parent.add_submodule(&module)?;
    Ok(exports.to_vec())
}
