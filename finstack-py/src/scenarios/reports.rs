//! Report types from scenario execution.

use crate::core::dates::utils::date_to_py;
use finstack_core::currency::Currency;
use finstack_core::money::Money;
use finstack_scenarios::adapters::RollForwardReport;
use finstack_scenarios::engine::ApplicationReport;
use indexmap::IndexMap;
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule, PyType};
use time::macros::date;

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
#[pyclass(
    module = "finstack.scenarios",
    name = "ApplicationReport",
    frozen,
    from_py_object
)]
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
/// instrument_carry : list[tuple[str, list[tuple[str, float]]]]
///     Per-instrument carry accrual by currency as
///     ``[(instrument_id, [(currency_code, amount), ...]), ...]``.
/// total_carry : dict[str, float]
///     Total P&L from carry by currency (ISO 4217 code -> amount).
#[pyclass(
    module = "finstack.scenarios",
    name = "RollForwardReport",
    frozen,
    from_py_object
)]
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
    #[classmethod]
    fn example(_cls: &Bound<'_, PyType>) -> PyResult<Self> {
        let mut total_carry: IndexMap<Currency, Money> = IndexMap::new();
        total_carry.insert(Currency::USD, Money::new(25_000.0, Currency::USD));

        Ok(Self::new(RollForwardReport {
            old_date: date!(2025 - 01 - 01),
            new_date: date!(2025 - 02 - 01),
            days: 31,
            instrument_carry: Vec::new(),
            total_carry,
            failed_instruments: Vec::new(),
        }))
    }

    #[getter]
    /// Original as-of date.
    ///
    /// Returns
    /// -------
    /// date
    ///     Date before roll.
    fn old_date(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.old_date)
    }

    #[getter]
    /// New as-of date after roll.
    ///
    /// Returns
    /// -------
    /// date
    ///     Date after roll.
    fn new_date(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
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
    /// list[tuple[str, list[tuple[str, float]]]]
    ///     List of (instrument_id, [(currency_code, amount)]) pairs.
    fn instrument_carry(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let outer = PyList::empty(py);
        for (id, per_ccy) in &self.inner.instrument_carry {
            let inner = PyList::empty(py);
            for (ccy, money) in per_ccy {
                let code = format!("{}", ccy);
                inner.append((code, money.amount()))?;
            }
            outer.append((id.clone(), inner))?;
        }
        Ok(outer.into())
    }

    #[getter]
    /// Total P&L from carry.
    ///
    /// Returns
    /// -------
    /// dict[str, float]
    ///     Total carry by currency code.
    fn total_carry(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let dict = pyo3::types::PyDict::new(py);
        for (ccy, money) in &self.inner.total_carry {
            let code = format!("{}", ccy);
            dict.set_item(code, money.amount())?;
        }
        Ok(dict.into())
    }

    #[getter]
    /// Instruments whose carry calculation failed without aborting the roll.
    ///
    /// Returns
    /// -------
    /// list[tuple[str, str]]
    ///     List of ``(instrument_id, error_message)`` pairs.
    fn failed_instruments(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let failures = PyList::empty(py);
        for (instrument_id, message) in &self.inner.failed_instruments {
            failures.append((instrument_id.clone(), message.clone()))?;
        }
        Ok(failures.into())
    }

    fn __repr__(&self) -> String {
        format!(
            "RollForwardReport(days={}, currencies={})",
            self.inner.days,
            self.inner.total_carry.len()
        )
    }
}

/// Register report types with the scenarios module.
pub(crate) fn register(
    _py: Python<'_>,
    module: &Bound<'_, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyApplicationReport>()?;
    module.add_class::<PyRollForwardReport>()?;

    Ok(vec!["ApplicationReport", "RollForwardReport"])
}
