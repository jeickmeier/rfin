//! Python bindings for structured credit result types.
//!
//! Wraps `TrancheCashflows`, `TrancheValuation`, `WaterfallDistribution`,
//! `PaymentRecord`, and coverage test `TestResult` from the core valuations library.

use crate::core::dates::utils::date_to_py;
use crate::core::money::PyMoney;
use finstack_valuations::instruments::fixed_income::structured_credit::{
    PaymentRecord as RustPaymentRecord, TestResult as RustTestResult,
    TrancheCashflows as RustTrancheCashflows, TrancheValuation as RustTrancheValuation,
    WaterfallDistribution as RustWaterfallDistribution,
};
use pyo3::prelude::*;
use std::collections::HashMap;

use super::utils::to_dict_via_serde;

// ============================================================================
// HELPERS
// ============================================================================

fn dated_flows_to_py(
    py: Python<'_>,
    flows: &[(time::Date, finstack_core::money::Money)],
) -> PyResult<Vec<(Py<PyAny>, PyMoney)>> {
    flows
        .iter()
        .map(|(date, money)| Ok((date_to_py(py, *date)?, PyMoney::new(*money))))
        .collect()
}

// ============================================================================
// PyTrancheCashflows
// ============================================================================

/// Result containing tranche-specific cashflows and metadata.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "TrancheCashflows",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyTrancheCashflows {
    pub(crate) inner: RustTrancheCashflows,
}

#[pymethods]
impl PyTrancheCashflows {
    #[getter]
    fn tranche_id(&self) -> &str {
        &self.inner.tranche_id
    }

    #[getter]
    fn final_balance(&self) -> PyMoney {
        PyMoney::new(self.inner.final_balance)
    }

    #[getter]
    fn total_interest(&self) -> PyMoney {
        PyMoney::new(self.inner.total_interest)
    }

    #[getter]
    fn total_principal(&self) -> PyMoney {
        PyMoney::new(self.inner.total_principal)
    }

    #[getter]
    fn total_pik(&self) -> PyMoney {
        PyMoney::new(self.inner.total_pik)
    }

    #[getter]
    fn total_writedown(&self) -> PyMoney {
        PyMoney::new(self.inner.total_writedown)
    }

    #[getter]
    fn cashflows(&self, py: Python<'_>) -> PyResult<Vec<(Py<PyAny>, PyMoney)>> {
        dated_flows_to_py(py, &self.inner.cashflows)
    }

    #[getter]
    fn interest_flows(&self, py: Python<'_>) -> PyResult<Vec<(Py<PyAny>, PyMoney)>> {
        dated_flows_to_py(py, &self.inner.interest_flows)
    }

    #[getter]
    fn principal_flows(&self, py: Python<'_>) -> PyResult<Vec<(Py<PyAny>, PyMoney)>> {
        dated_flows_to_py(py, &self.inner.principal_flows)
    }

    #[getter]
    fn pik_flows(&self, py: Python<'_>) -> PyResult<Vec<(Py<PyAny>, PyMoney)>> {
        dated_flows_to_py(py, &self.inner.pik_flows)
    }

    #[getter]
    fn writedown_flows(&self, py: Python<'_>) -> PyResult<Vec<(Py<PyAny>, PyMoney)>> {
        dated_flows_to_py(py, &self.inner.writedown_flows)
    }

    #[getter]
    fn detailed_flows(&self, py: Python<'_>) -> PyResult<Vec<Py<PyAny>>> {
        self.inner
            .detailed_flows
            .iter()
            .map(|cf| to_dict_via_serde(py, cf))
            .collect()
    }

    #[pyo3(text_signature = "($self)")]
    fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        to_dict_via_serde(py, &self.inner)
    }

    fn __repr__(&self) -> String {
        format!(
            "TrancheCashflows(tranche_id='{}', flows={}, final_balance={})",
            self.inner.tranche_id,
            self.inner.cashflows.len(),
            self.inner.final_balance,
        )
    }
}

// ============================================================================
// PyTrancheValuation
// ============================================================================

/// Tranche-specific valuation result with price, duration, and spread metrics.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "TrancheValuation",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyTrancheValuation {
    pub(crate) inner: RustTrancheValuation,
}

#[pymethods]
impl PyTrancheValuation {
    #[getter]
    fn tranche_id(&self) -> &str {
        &self.inner.tranche_id
    }

    #[getter]
    fn pv(&self) -> PyMoney {
        PyMoney::new(self.inner.pv)
    }

    #[getter]
    fn clean_price(&self) -> f64 {
        self.inner.clean_price
    }

    #[getter]
    fn dirty_price(&self) -> f64 {
        self.inner.dirty_price
    }

    #[getter]
    fn accrued(&self) -> PyMoney {
        PyMoney::new(self.inner.accrued)
    }

    #[getter]
    fn wal(&self) -> f64 {
        self.inner.wal
    }

    #[getter]
    fn modified_duration(&self) -> f64 {
        self.inner.modified_duration
    }

    #[getter]
    fn z_spread_bps(&self) -> f64 {
        self.inner.z_spread_bps
    }

    #[getter]
    fn cs01(&self) -> f64 {
        self.inner.cs01
    }

    #[getter]
    fn ytm(&self) -> f64 {
        self.inner.ytm
    }

    #[getter]
    fn metrics(&self) -> std::collections::HashMap<String, f64> {
        self.inner
            .metrics
            .iter()
            .map(|(k, v)| (k.as_str().to_string(), *v))
            .collect()
    }

    #[pyo3(text_signature = "($self)")]
    fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        to_dict_via_serde(py, &self.inner)
    }

    fn __repr__(&self) -> String {
        format!(
            "TrancheValuation(tranche_id='{}', pv={}, clean_price={:.4}, wal={:.2})",
            self.inner.tranche_id, self.inner.pv, self.inner.clean_price, self.inner.wal,
        )
    }
}

// ============================================================================
// PyPaymentRecord
// ============================================================================

/// Record of an individual waterfall payment.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "PaymentRecord",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyPaymentRecord {
    pub(crate) inner: RustPaymentRecord,
}

#[pymethods]
impl PyPaymentRecord {
    #[getter]
    fn tier_id(&self) -> &str {
        &self.inner.tier_id
    }

    #[getter]
    fn recipient_id(&self) -> &str {
        &self.inner.recipient_id
    }

    #[getter]
    fn priority(&self) -> usize {
        self.inner.priority
    }

    #[getter]
    fn recipient(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        to_dict_via_serde(py, &self.inner.recipient)
    }

    #[getter]
    fn recipient_type(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        to_dict_via_serde(py, &self.inner.recipient)
    }

    #[getter]
    fn requested_amount(&self) -> PyMoney {
        PyMoney::new(self.inner.requested_amount)
    }

    #[getter]
    fn paid_amount(&self) -> PyMoney {
        PyMoney::new(self.inner.paid_amount)
    }

    #[getter]
    fn shortfall(&self) -> PyMoney {
        PyMoney::new(self.inner.shortfall)
    }

    #[getter]
    fn diverted(&self) -> bool {
        self.inner.diverted
    }

    #[pyo3(text_signature = "($self)")]
    fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        to_dict_via_serde(py, &self.inner)
    }

    fn __repr__(&self) -> String {
        format!(
            "PaymentRecord(tier='{}', recipient='{}', paid={}, shortfall={})",
            self.inner.tier_id,
            self.inner.recipient_id,
            self.inner.paid_amount,
            self.inner.shortfall,
        )
    }
}

// ============================================================================
// PyWaterfallDistribution
// ============================================================================

/// Result of a single-period waterfall distribution.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "WaterfallDistribution",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyWaterfallDistribution {
    pub(crate) inner: RustWaterfallDistribution,
}

#[pymethods]
impl PyWaterfallDistribution {
    #[getter]
    fn payment_date(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.payment_date)
    }

    #[getter]
    fn total_available(&self) -> PyMoney {
        PyMoney::new(self.inner.total_available)
    }

    #[getter]
    fn diverted_cash(&self) -> PyMoney {
        PyMoney::new(self.inner.diverted_cash)
    }

    #[getter]
    fn remaining_cash(&self) -> PyMoney {
        PyMoney::new(self.inner.remaining_cash)
    }

    #[getter]
    fn had_diversions(&self) -> bool {
        self.inner.had_diversions
    }

    #[getter]
    fn diversion_reason(&self) -> Option<&str> {
        self.inner.diversion_reason.as_deref()
    }

    #[getter]
    fn tier_allocations(&self) -> Vec<(String, PyMoney)> {
        self.inner
            .tier_allocations
            .iter()
            .map(|(id, money)| (id.clone(), PyMoney::new(*money)))
            .collect()
    }

    #[getter]
    fn payment_records(&self) -> Vec<PyPaymentRecord> {
        self.inner
            .payment_records
            .iter()
            .map(|r| PyPaymentRecord { inner: r.clone() })
            .collect()
    }

    #[getter]
    fn coverage_tests(&self) -> Vec<(String, f64, bool)> {
        self.inner.coverage_tests.clone()
    }

    #[getter]
    fn distributions(&self) -> HashMap<String, PyMoney> {
        self.inner
            .distributions
            .iter()
            .map(|(key, money)| (format!("{:?}", key), PyMoney::new(*money)))
            .collect()
    }

    #[pyo3(text_signature = "($self)")]
    fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        to_dict_via_serde(py, &self.inner)
    }

    fn __repr__(&self) -> String {
        format!(
            "WaterfallDistribution(date={}, available={}, remaining={}, diversions={})",
            self.inner.payment_date,
            self.inner.total_available,
            self.inner.remaining_cash,
            self.inner.had_diversions,
        )
    }
}

// ============================================================================
// PyTestResult
// ============================================================================

/// Result of a coverage test calculation (OC/IC).
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "TestResult",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyTestResult {
    pub(crate) inner: RustTestResult,
}

#[pymethods]
impl PyTestResult {
    #[getter]
    fn test_id(&self) -> &str {
        &self.inner.test_id
    }

    #[getter]
    fn current_ratio(&self) -> f64 {
        self.inner.current_ratio
    }

    #[getter]
    fn is_passing(&self) -> bool {
        self.inner.is_passing
    }

    #[getter]
    fn cure_amount(&self) -> Option<PyMoney> {
        self.inner.cure_amount.map(PyMoney::new)
    }

    #[pyo3(text_signature = "($self)")]
    fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        to_dict_via_serde(py, &self.inner)
    }

    fn __repr__(&self) -> String {
        format!(
            "TestResult(test_id='{}', ratio={:.4}, passing={})",
            self.inner.test_id, self.inner.current_ratio, self.inner.is_passing,
        )
    }
}

// ============================================================================
// REGISTRATION
// ============================================================================

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyTrancheCashflows>()?;
    module.add_class::<PyTrancheValuation>()?;
    module.add_class::<PyPaymentRecord>()?;
    module.add_class::<PyWaterfallDistribution>()?;
    module.add_class::<PyTestResult>()?;

    Ok(vec![
        "TrancheCashflows",
        "TrancheValuation",
        "PaymentRecord",
        "WaterfallDistribution",
        "TestResult",
    ])
}
