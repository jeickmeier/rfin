//! Python bindings for capital structure cashflow aggregation.
//!
//! Wraps `finstack_statements::capital_structure` types and exposes
//! high-level functions for aggregating instrument cashflows across
//! a capital structure specification.

use crate::core::dates::periods::{PyPeriod, PyPeriodId};
use crate::core::dates::utils::py_to_date;
use crate::core::market_data::context::PyMarketContext;
use crate::core::money::PyMoney;
use crate::statements::error::stmt_to_py;
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
use pyo3::Bound;

use finstack_statements::capital_structure::{self, CapitalStructureCashflows, CashflowBreakdown};

// ---------------------------------------------------------------------------
// CashflowBreakdown
// ---------------------------------------------------------------------------

/// Per-period cashflow breakdown for a single instrument or aggregate.
///
/// Attributes:
///     interest_expense_cash: Cash portion of interest expense.
///     interest_expense_pik: PIK (payment-in-kind) portion of interest expense.
///     principal_payment: Principal payment for the period.
///     fees: Fees for the period.
///     debt_balance: Outstanding debt balance at period end.
///     accrued_interest: Accrued but unpaid interest.
#[pyclass(
    module = "finstack.statements.capital_structure",
    name = "CashflowBreakdown",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub struct PyCashflowBreakdown {
    inner: CashflowBreakdown,
}

#[pymethods]
impl PyCashflowBreakdown {
    #[getter]
    fn interest_expense_cash(&self) -> PyMoney {
        PyMoney::new(self.inner.interest_expense_cash)
    }

    #[getter]
    fn interest_expense_pik(&self) -> PyMoney {
        PyMoney::new(self.inner.interest_expense_pik)
    }

    #[getter]
    fn interest_expense_total(&self) -> PyMoney {
        PyMoney::new(self.inner.interest_expense_total())
    }

    #[getter]
    fn principal_payment(&self) -> PyMoney {
        PyMoney::new(self.inner.principal_payment)
    }

    #[getter]
    fn fees(&self) -> PyMoney {
        PyMoney::new(self.inner.fees)
    }

    #[getter]
    fn debt_balance(&self) -> PyMoney {
        PyMoney::new(self.inner.debt_balance)
    }

    #[getter]
    fn accrued_interest(&self) -> PyMoney {
        PyMoney::new(self.inner.accrued_interest)
    }

    fn __repr__(&self) -> String {
        format!(
            "CashflowBreakdown(interest={}, principal={}, balance={})",
            self.inner.interest_expense_total(),
            self.inner.principal_payment,
            self.inner.debt_balance
        )
    }
}

// ---------------------------------------------------------------------------
// CapitalStructureCashflows
// ---------------------------------------------------------------------------

/// Aggregated capital structure cashflows across all instruments and periods.
///
/// Provides accessors for per-instrument and aggregate (total) flows,
/// with automatic cross-currency aggregation when a reporting currency
/// is specified.
#[pyclass(
    module = "finstack.statements.capital_structure",
    name = "CapitalStructureCashflows",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub struct PyCapitalStructureCashflows {
    inner: CapitalStructureCashflows,
}

#[pymethods]
impl PyCapitalStructureCashflows {
    /// List of instrument IDs in the capital structure.
    #[getter]
    fn instrument_ids(&self) -> Vec<String> {
        self.inner.by_instrument.keys().cloned().collect()
    }

    /// List of period IDs that have aggregate totals.
    #[getter]
    fn period_ids(&self) -> Vec<PyPeriodId> {
        self.inner
            .totals
            .keys()
            .map(|pid| PyPeriodId::new(*pid))
            .collect()
    }

    /// Get the interest expense for a specific instrument and period.
    fn get_interest(&self, instrument_id: &str, period_id: PyPeriodId) -> PyResult<f64> {
        self.inner
            .get_interest(instrument_id, &period_id.inner)
            .map_err(stmt_to_py)
    }

    /// Get the cash interest expense for a specific instrument and period.
    fn get_interest_cash(&self, instrument_id: &str, period_id: PyPeriodId) -> PyResult<f64> {
        self.inner
            .get_interest_cash(instrument_id, &period_id.inner)
            .map_err(stmt_to_py)
    }

    /// Get the PIK interest expense for a specific instrument and period.
    fn get_interest_pik(&self, instrument_id: &str, period_id: PyPeriodId) -> PyResult<f64> {
        self.inner
            .get_interest_pik(instrument_id, &period_id.inner)
            .map_err(stmt_to_py)
    }

    /// Get the principal payment for a specific instrument and period.
    fn get_principal(&self, instrument_id: &str, period_id: PyPeriodId) -> PyResult<f64> {
        self.inner
            .get_principal(instrument_id, &period_id.inner)
            .map_err(stmt_to_py)
    }

    /// Get the debt balance for a specific instrument and period.
    fn get_debt_balance(&self, instrument_id: &str, period_id: PyPeriodId) -> PyResult<f64> {
        self.inner
            .get_debt_balance(instrument_id, &period_id.inner)
            .map_err(stmt_to_py)
    }

    /// Get the accrued interest for a specific instrument and period.
    fn get_accrued_interest(&self, instrument_id: &str, period_id: PyPeriodId) -> PyResult<f64> {
        self.inner
            .get_accrued_interest(instrument_id, &period_id.inner)
            .map_err(stmt_to_py)
    }

    /// Get the total interest expense across all instruments for a period.
    fn get_total_interest(&self, period_id: PyPeriodId) -> PyResult<f64> {
        self.inner
            .get_total_interest(&period_id.inner)
            .map_err(stmt_to_py)
    }

    /// Get the total cash interest expense across all instruments for a period.
    fn get_total_interest_cash(&self, period_id: PyPeriodId) -> PyResult<f64> {
        self.inner
            .get_total_interest_cash(&period_id.inner)
            .map_err(stmt_to_py)
    }

    /// Get the total PIK interest expense across all instruments for a period.
    fn get_total_interest_pik(&self, period_id: PyPeriodId) -> PyResult<f64> {
        self.inner
            .get_total_interest_pik(&period_id.inner)
            .map_err(stmt_to_py)
    }

    /// Get the total principal payment across all instruments for a period.
    fn get_total_principal(&self, period_id: PyPeriodId) -> PyResult<f64> {
        self.inner
            .get_total_principal(&period_id.inner)
            .map_err(stmt_to_py)
    }

    /// Get the total debt balance across all instruments for a period.
    fn get_total_debt_balance(&self, period_id: PyPeriodId) -> PyResult<f64> {
        self.inner
            .get_total_debt_balance(&period_id.inner)
            .map_err(stmt_to_py)
    }

    /// Get the total accrued interest across all instruments for a period.
    fn get_total_accrued_interest(&self, period_id: PyPeriodId) -> PyResult<f64> {
        self.inner
            .get_total_accrued_interest(&period_id.inner)
            .map_err(stmt_to_py)
    }

    /// Get the full breakdown for a given instrument and period.
    fn get_breakdown(
        &self,
        instrument_id: &str,
        period_id: PyPeriodId,
    ) -> PyResult<Option<PyCashflowBreakdown>> {
        Ok(self
            .inner
            .by_instrument
            .get(instrument_id)
            .and_then(|periods| periods.get(&period_id.inner))
            .map(|b| PyCashflowBreakdown { inner: b.clone() }))
    }

    /// Get the aggregate breakdown for a given period.
    fn get_total_breakdown(&self, period_id: PyPeriodId) -> Option<PyCashflowBreakdown> {
        self.inner
            .totals
            .get(&period_id.inner)
            .map(|b| PyCashflowBreakdown { inner: b.clone() })
    }

    fn __repr__(&self) -> String {
        format!(
            "CapitalStructureCashflows(instruments={}, periods={})",
            self.inner.by_instrument.len(),
            self.inner.totals.len()
        )
    }
}

// ---------------------------------------------------------------------------
// Functions
// ---------------------------------------------------------------------------

/// Aggregate cashflows across all instruments in a capital structure.
///
/// Builds instruments from the spec's debt instrument definitions,
/// then computes period-by-period cashflow breakdowns for each
/// instrument and rolls up aggregate totals.
///
/// Args:
///     spec: :class:`~finstack.statements.types.CapitalStructureSpec` defining
///         the capital structure.
///     periods: List of :class:`~finstack.core.dates.periods.Period` to evaluate.
///     market: :class:`~finstack.core.market_data.context.MarketContext` with
///         discount curves required by the instruments.
///     as_of: Valuation date.
///
/// Returns:
///     CapitalStructureCashflows: Per-instrument and aggregate cashflows.
///
/// Raises:
///     FinstackError: If instrument construction or cashflow generation fails.
#[pyfunction]
fn aggregate_instrument_cashflows<'py>(
    spec: &crate::statements::types::model::PyCapitalStructureSpec,
    periods: Vec<PyPeriod>,
    market: &PyMarketContext,
    as_of: &Bound<'py, PyAny>,
) -> PyResult<PyCapitalStructureCashflows> {
    let as_of_date = py_to_date(as_of)?;

    let core_periods: Vec<_> = periods.into_iter().map(|p| p.inner).collect();

    let mut instruments = indexmap::IndexMap::new();
    for debt_spec in &spec.inner.debt_instruments {
        let id = match debt_spec {
            finstack_statements::types::DebtInstrumentSpec::Bond { id, .. }
            | finstack_statements::types::DebtInstrumentSpec::Swap { id, .. }
            | finstack_statements::types::DebtInstrumentSpec::TermLoan { id, .. }
            | finstack_statements::types::DebtInstrumentSpec::Generic { id, .. } => id.clone(),
        };
        let inst =
            capital_structure::build_any_instrument_from_spec(debt_spec).map_err(stmt_to_py)?;
        instruments.insert(id, inst);
    }

    let result = capital_structure::aggregate_instrument_cashflows(
        &spec.inner,
        &instruments,
        &core_periods,
        &market.inner,
        as_of_date,
    )
    .map_err(stmt_to_py)?;

    Ok(PyCapitalStructureCashflows { inner: result })
}

// ---------------------------------------------------------------------------
// Module Registration
// ---------------------------------------------------------------------------

pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "capital_structure")?;
    module.setattr(
        "__doc__",
        "Capital structure cashflow aggregation: instrument-level and aggregate period flows.",
    )?;

    module.add_class::<PyCashflowBreakdown>()?;
    module.add_class::<PyCapitalStructureCashflows>()?;
    module.add_function(wrap_pyfunction!(aggregate_instrument_cashflows, &module)?)?;

    let exports = vec![
        "CashflowBreakdown",
        "CapitalStructureCashflows",
        "aggregate_instrument_cashflows",
    ];
    module.setattr("__all__", PyList::new(py, &exports)?)?;
    parent.add_submodule(&module)?;
    Ok(exports)
}
