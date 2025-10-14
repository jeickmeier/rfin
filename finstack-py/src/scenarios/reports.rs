//! Report types from scenario execution.

use crate::core::utils::date_to_py;
use finstack_scenarios::adapters::RollForwardReport;
use finstack_scenarios::engine::ApplicationReport;
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};

/// Report describing what happened during scenario application.
///
/// Attributes
/// ----------
/// operations_applied : int
///     Number of operations successfully applied.
/// warnings : list[str]
///     Warnings generated during application (non-fatal).
/// rounding_context : str | None
///     Rounding context stamp (for determinism tracking).
#[pyclass(module = "finstack.scenarios", name = "ApplicationReport", frozen)]
#[derive(Clone, Debug)]
pub struct PyApplicationReport {
    pub(crate) inner: ApplicationReport,
}

impl PyApplicationReport {
    pub fn new(inner: ApplicationReport) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyApplicationReport {
    #[getter]
    /// Number of operations successfully applied.
    ///
    /// Returns
    /// -------
    /// int
    ///     Count of applied operations.
    fn operations_applied(&self) -> usize {
        self.inner.operations_applied
    }

    #[getter]
    /// Warnings generated during application (non-fatal).
    ///
    /// Returns
    /// -------
    /// list[str]
    ///     List of warning messages.
    fn warnings(&self) -> Vec<String> {
        self.inner.warnings.clone()
    }

    #[getter]
    /// Rounding context stamp (for determinism tracking).
    ///
    /// Returns
    /// -------
    /// str | None
    ///     Rounding context identifier if available.
    fn rounding_context(&self) -> Option<String> {
        self.inner.rounding_context.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "ApplicationReport(operations_applied={}, warnings={}, rounding_context={:?})",
            self.inner.operations_applied,
            self.inner.warnings.len(),
            self.inner.rounding_context
        )
    }
}

/// Report from time roll-forward operation.
///
/// Attributes
/// ----------
/// old_date : date
///     Original as-of date.
/// new_date : date
///     New as-of date after roll.
/// days : int
///     Number of days rolled forward.
/// instrument_carry : list[tuple[str, float]]
///     Per-instrument carry accrual.
/// instrument_mv_change : list[tuple[str, float]]
///     Per-instrument market value change.
/// total_carry : float
///     Total P&L from carry.
/// total_mv_change : float
///     Total P&L from market value changes.
#[pyclass(module = "finstack.scenarios", name = "RollForwardReport", frozen)]
#[derive(Clone, Debug)]
pub struct PyRollForwardReport {
    pub(crate) inner: RollForwardReport,
}

impl PyRollForwardReport {
    pub fn new(inner: RollForwardReport) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyRollForwardReport {
    #[getter]
    /// Original as-of date.
    ///
    /// Returns
    /// -------
    /// date
    ///     Date before roll.
    fn old_date(&self, py: Python<'_>) -> PyResult<PyObject> {
        date_to_py(py, self.inner.old_date)
    }

    #[getter]
    /// New as-of date after roll.
    ///
    /// Returns
    /// -------
    /// date
    ///     Date after roll.
    fn new_date(&self, py: Python<'_>) -> PyResult<PyObject> {
        date_to_py(py, self.inner.new_date)
    }

    #[getter]
    /// Number of days rolled forward.
    ///
    /// Returns
    /// -------
    /// int
    ///     Day count.
    fn days(&self) -> i64 {
        self.inner.days
    }

    #[getter]
    /// Per-instrument carry accrual.
    ///
    /// Returns
    /// -------
    /// list[tuple[str, float]]
    ///     List of (instrument_id, carry) pairs.
    fn instrument_carry(&self, py: Python<'_>) -> PyResult<PyObject> {
        let list = PyList::new(
            py,
            self.inner
                .instrument_carry
                .iter()
                .map(|(id, value)| (id.clone(), *value)),
        )?;
        Ok(list.into())
    }

    #[getter]
    /// Per-instrument market value change.
    ///
    /// Returns
    /// -------
    /// list[tuple[str, float]]
    ///     List of (instrument_id, mv_change) pairs.
    fn instrument_mv_change(&self, py: Python<'_>) -> PyResult<PyObject> {
        let list = PyList::new(
            py,
            self.inner
                .instrument_mv_change
                .iter()
                .map(|(id, value)| (id.clone(), *value)),
        )?;
        Ok(list.into())
    }

    #[getter]
    /// Total P&L from carry.
    ///
    /// Returns
    /// -------
    /// float
    ///     Total carry.
    fn total_carry(&self) -> f64 {
        self.inner.total_carry
    }

    #[getter]
    /// Total P&L from market value changes.
    ///
    /// Returns
    /// -------
    /// float
    ///     Total market value change.
    fn total_mv_change(&self) -> f64 {
        self.inner.total_mv_change
    }

    fn __repr__(&self) -> String {
        format!(
            "RollForwardReport(days={}, total_carry={:.2}, total_mv_change={:.2})",
            self.inner.days, self.inner.total_carry, self.inner.total_mv_change
        )
    }
}

/// Register report types with the scenarios module.
pub(crate) fn register(_py: Python<'_>, module: &Bound<'_, PyModule>) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyApplicationReport>()?;
    module.add_class::<PyRollForwardReport>()?;

    Ok(vec!["ApplicationReport", "RollForwardReport"])
}

