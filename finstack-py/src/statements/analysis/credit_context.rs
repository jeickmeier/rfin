//! Credit context metrics bindings.

use crate::core::dates::periods::{PyPeriod, PyPeriodId};
use crate::statements::capital_structure::PyCapitalStructureCashflows;
use crate::statements::evaluator::PyStatementResult;
use finstack_statements::analysis::credit_context::{
    compute_credit_context as rs_compute_credit_context, CreditContextMetrics,
};
use pyo3::prelude::*;
use pyo3::types::PyModule;
use pyo3::{wrap_pyfunction, Bound};

/// Per-instrument credit context metrics.
#[pyclass(
    module = "finstack.statements.analysis",
    name = "CreditContextMetrics",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyCreditContextMetrics {
    pub(crate) inner: CreditContextMetrics,
}

impl PyCreditContextMetrics {
    pub(crate) fn new(inner: CreditContextMetrics) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyCreditContextMetrics {
    #[getter]
    /// DSCR by period.
    fn dscr(&self) -> Vec<(PyPeriodId, f64)> {
        self.inner
            .dscr
            .iter()
            .map(|(p, v)| (PyPeriodId::new(*p), *v))
            .collect()
    }

    #[getter]
    /// Interest coverage by period.
    fn interest_coverage(&self) -> Vec<(PyPeriodId, f64)> {
        self.inner
            .interest_coverage
            .iter()
            .map(|(p, v)| (PyPeriodId::new(*p), *v))
            .collect()
    }

    #[getter]
    /// LTV by period.
    fn ltv(&self) -> Vec<(PyPeriodId, f64)> {
        self.inner
            .ltv
            .iter()
            .map(|(p, v)| (PyPeriodId::new(*p), *v))
            .collect()
    }

    #[getter]
    /// Minimum DSCR across all periods.
    fn dscr_min(&self) -> Option<f64> {
        self.inner.dscr_min
    }

    #[getter]
    /// Minimum interest coverage across all periods.
    fn interest_coverage_min(&self) -> Option<f64> {
        self.inner.interest_coverage_min
    }

    fn __repr__(&self) -> String {
        format!(
            "CreditContextMetrics(dscr_min={:?}, icr_min={:?})",
            self.inner.dscr_min, self.inner.interest_coverage_min
        )
    }
}

#[pyfunction]
#[pyo3(
    signature = (statement, cs_cashflows, instrument_id, coverage_node, periods, reference_value=None),
    name = "compute_credit_context"
)]
/// Compute credit context metrics for a specific instrument.
///
/// Parameters
/// ----------
/// statement : StatementResult
///     Evaluated statement results
/// cs_cashflows : CapitalStructureCashflows
///     Capital structure cashflows
/// instrument_id : str
///     Instrument to compute metrics for
/// coverage_node : str
///     Statement node for coverage numerator (e.g. "ebitda")
/// periods : list[Period]
///     Periods over which to compute metrics
/// reference_value : float | None
///     Optional reference value for LTV (e.g. enterprise value)
///
/// Returns
/// -------
/// CreditContextMetrics
///     Credit metrics (DSCR, interest coverage, LTV)
fn py_compute_credit_context(
    statement: &PyStatementResult,
    cs_cashflows: &PyCapitalStructureCashflows,
    instrument_id: &str,
    coverage_node: &str,
    periods: Vec<PyPeriod>,
    reference_value: Option<f64>,
) -> PyCreditContextMetrics {
    let rs_periods: Vec<finstack_core::dates::Period> =
        periods.iter().map(|p| p.inner.clone()).collect();
    let metrics = rs_compute_credit_context(
        &statement.inner,
        &cs_cashflows.inner,
        instrument_id,
        coverage_node,
        &rs_periods,
        reference_value,
    );
    PyCreditContextMetrics::new(metrics)
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyCreditContextMetrics>()?;
    module.add_function(wrap_pyfunction!(py_compute_credit_context, module)?)?;
    Ok(vec!["CreditContextMetrics", "compute_credit_context"])
}
