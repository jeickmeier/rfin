//! Explainability bindings: options and traces.
//!
//! This module exposes the explanation infrastructure:
//! - `ExplainOpts`: Configuration for opting into execution tracing
//! - `ExplanationTrace`: Container for execution trace entries
//! - `TraceEntry`: Execution events (available as dicts via pythonize)

use finstack_core::explain::{ExplainOpts, ExplanationTrace, TraceEntry};
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule, PyType};
use pyo3::{Bound, PyResult};
use pythonize::{depythonize, pythonize};
use serde_json;

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
#[pyclass(name = "ExplanationTrace", module = "finstack.core.explain")]
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
    #[new]
    #[pyo3(text_signature = "(trace_type)")]
    /// Create a new empty trace of the given type.
    ///
    /// Parameters
    /// ----------
    /// trace_type : str
    ///     Type of trace (e.g., "calibration", "pricing", "waterfall").
    ///
    /// Returns
    /// -------
    /// ExplanationTrace
    ///     Empty trace ready to receive entries.
    fn ctor(trace_type: String) -> Self {
        Self {
            inner: ExplanationTrace::new(trace_type),
        }
    }

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
    fn entries(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
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

    /// Append a trace entry, respecting optional caps.
    #[pyo3(text_signature = "(self, entry, max_entries=None)")]
    fn push(&mut self, entry: PyRef<PyTraceEntry>, max_entries: Option<usize>) -> PyResult<()> {
        self.inner.push(entry.inner.clone(), max_entries);
        Ok(())
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

/// Domain-specific trace entry for explainability output.
#[pyclass(name = "TraceEntry", module = "finstack.core.explain", frozen)]
#[derive(Clone, Debug)]
pub struct PyTraceEntry {
    pub(crate) inner: TraceEntry,
}

impl PyTraceEntry {
    pub(crate) fn new(inner: TraceEntry) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyTraceEntry {
    #[classmethod]
    #[pyo3(text_signature = "(cls, iteration, residual, knots_updated, converged)")]
    fn calibration_iteration(
        _cls: &Bound<'_, PyType>,
        iteration: usize,
        residual: f64,
        knots_updated: Vec<String>,
        converged: bool,
    ) -> Self {
        Self::new(TraceEntry::CalibrationIteration {
            iteration,
            residual,
            knots_updated,
            converged,
        })
    }

    #[classmethod]
    #[pyo3(
        text_signature = "(cls, date, cashflow_amount, cashflow_currency, discount_factor, pv_amount, pv_currency, curve_id)"
    )]
    #[allow(clippy::too_many_arguments)]
    fn cashflow_pv(
        _cls: &Bound<'_, PyType>,
        date: String,
        cashflow_amount: f64,
        cashflow_currency: String,
        discount_factor: f64,
        pv_amount: f64,
        pv_currency: String,
        curve_id: String,
    ) -> Self {
        Self::new(TraceEntry::CashflowPV {
            date,
            cashflow_amount,
            cashflow_currency,
            discount_factor,
            pv_amount,
            pv_currency,
            curve_id,
        })
    }

    #[classmethod]
    #[pyo3(
        text_signature = "(cls, period, step_name, cash_in_amount, cash_in_currency, cash_out_amount, cash_out_currency, shortfall_amount=None, shortfall_currency=None)"
    )]
    #[allow(clippy::too_many_arguments)]
    fn waterfall_step(
        _cls: &Bound<'_, PyType>,
        period: usize,
        step_name: String,
        cash_in_amount: f64,
        cash_in_currency: String,
        cash_out_amount: f64,
        cash_out_currency: String,
        shortfall_amount: Option<f64>,
        shortfall_currency: Option<String>,
    ) -> Self {
        Self::new(TraceEntry::WaterfallStep {
            period,
            step_name,
            cash_in_amount,
            cash_in_currency,
            cash_out_amount,
            cash_out_currency,
            shortfall_amount,
            shortfall_currency,
        })
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, name, description, metadata=None)")]
    fn computation_step(
        _cls: &Bound<'_, PyType>,
        name: String,
        description: String,
        metadata: Option<Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        let meta = if let Some(value) = metadata {
            Some(depythonize::<serde_json::Value>(&value)?)
        } else {
            None
        };
        Ok(Self::new(TraceEntry::ComputationStep {
            name,
            description,
            metadata: meta,
        }))
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, row_labels, col_labels, sensitivity_matrix)")]
    fn jacobian(
        _cls: &Bound<'_, PyType>,
        row_labels: Vec<String>,
        col_labels: Vec<String>,
        sensitivity_matrix: Vec<Vec<f64>>,
    ) -> Self {
        Self::new(TraceEntry::Jacobian {
            row_labels,
            col_labels,
            sensitivity_matrix,
        })
    }

    #[getter]
    fn kind(&self) -> &'static str {
        match self.inner {
            TraceEntry::CalibrationIteration { .. } => "calibration_iteration",
            TraceEntry::CashflowPV { .. } => "cashflow_pv",
            TraceEntry::WaterfallStep { .. } => "waterfall_step",
            TraceEntry::ComputationStep { .. } => "computation_step",
            TraceEntry::Jacobian { .. } => "jacobian",
        }
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
    module.add_class::<PyTraceEntry>()?;

    let exports = ["ExplainOpts", "ExplanationTrace", "TraceEntry"];
    module.setattr("__all__", PyList::new(py, exports)?)?;
    parent.add_submodule(&module)?;
    Ok(exports.to_vec())
}
