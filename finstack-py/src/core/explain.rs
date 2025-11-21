//! Explainability bindings: options and traces.
//!
//! This module exposes the explanation infrastructure:
//! - `ExplainOpts`: Configuration for opting into execution tracing
//! - `ExplanationTrace`: Container for execution trace entries
//! - `TraceEntry`: Execution events (available as dicts via pythonize)

use finstack_core::explain::{ExplainOpts, ExplanationTrace};
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule, PyType};
use pyo3::{Bound, PyResult};
use pythonize::pythonize;

/// Opt-in configuration for generating explanation traces.
///
/// Controls whether detailed execution traces are captured during computation.
///
/// Parameters
/// ----------
/// enabled : bool
///     Whether explanation tracing is enabled.
/// max_entries : int, optional
///     Maximum number of trace entries (caps memory usage).
#[pyclass(name = "ExplainOpts", module = "finstack.core.explain", frozen)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PyExplainOpts {
    pub(crate) inner: ExplainOpts,
}

impl PyExplainOpts {
    pub(crate) fn new(inner: ExplainOpts) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyExplainOpts {
    #[new]
    #[pyo3(signature = (enabled=false, max_entries=None))]
    #[pyo3(text_signature = "(enabled=False, max_entries=None)")]
    fn ctor(enabled: bool, max_entries: Option<usize>) -> Self {
        Self::new(ExplainOpts {
            enabled,
            max_entries,
        })
    }

    #[classattr]
    const DISABLED: Self = Self {
        inner: ExplainOpts {
            enabled: false,
            max_entries: None,
        },
    };

    #[classmethod]
    #[pyo3(text_signature = "(cls)")]
    fn enabled(_cls: &Bound<'_, PyType>) -> Self {
        Self::new(ExplainOpts::enabled())
    }

    #[getter]
    fn is_enabled(&self) -> bool {
        self.inner.enabled
    }

    #[getter]
    fn max_entries(&self) -> Option<usize> {
        self.inner.max_entries
    }

    fn __repr__(&self) -> String {
        format!(
            "ExplainOpts(enabled={}, max_entries={:?})",
            self.inner.enabled, self.inner.max_entries
        )
    }
}

/// Container for detailed execution traces of financial computations.
///
/// Traces are organized by type (calibration, pricing, waterfall) and contain
/// a sequence of domain-specific entries.
#[pyclass(name = "ExplanationTrace", module = "finstack.core.explain", frozen)]
#[derive(Clone, Debug)]
pub struct PyExplanationTrace {
    pub(crate) inner: ExplanationTrace,
}

impl From<ExplanationTrace> for PyExplanationTrace {
    fn from(inner: ExplanationTrace) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyExplanationTrace {
    #[getter]
    /// Type of trace (e.g., "calibration", "pricing").
    fn trace_type(&self) -> &str {
        &self.inner.trace_type
    }

    #[getter]
    /// Whether the trace was truncated due to size limits.
    fn truncated(&self) -> bool {
        self.inner.is_truncated()
    }

    #[getter]
    /// List of trace entries as dictionaries.
    fn entries(&self, py: Python<'_>) -> PyResult<PyObject> {
        pythonize(py, &self.inner.entries)
            .map(|b| b.unbind())
            .map_err(|e| {
                pyo3::exceptions::PyValueError::new_err(format!("Failed to pythonize entries: {e}"))
            })
    }

    /// Serialize the full trace to a JSON string (pretty-printed).
    #[pyo3(text_signature = "(self)")]
    fn to_json(&self) -> PyResult<String> {
        self.inner
            .to_json_pretty()
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "ExplanationTrace(type='{}', entries={}, truncated={})",
            self.inner.trace_type,
            self.inner.entries.len(),
            self.inner.is_truncated()
        )
    }
}

pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "explain")?;
    module.setattr(
        "__doc__",
        "Explainability components: options and trace containers.",
    )?;

    module.add_class::<PyExplainOpts>()?;
    module.add_class::<PyExplanationTrace>()?;

    let exports = ["ExplainOpts", "ExplanationTrace"];
    module.setattr("__all__", PyList::new(py, &exports)?)?;
    parent.add_submodule(&module)?;
    Ok(exports.to_vec())
}
