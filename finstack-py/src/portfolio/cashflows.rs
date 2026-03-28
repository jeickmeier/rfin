//! Python bindings for portfolio cashflow aggregation.

use crate::core::currency::extract_currency;
use crate::core::dates::periods::PyPeriod;
use crate::core::dates::utils::date_to_py;
use crate::core::market_data::context::PyMarketContext;
use crate::core::money::PyMoney;
use crate::portfolio::error::portfolio_to_py;
use crate::portfolio::positions::extract_portfolio;
use finstack_portfolio::cashflows::{
    aggregate_cashflows, cashflows_to_base_by_period, collapse_cashflows_to_base_by_date,
    CashflowWarning, PortfolioCashflowBuckets, PortfolioCashflows,
};
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict, PyList, PyModule, PyTuple};
use pyo3::Bound;

/// Aggregated portfolio cashflows by date and currency.
#[pyclass(
    module = "finstack.portfolio",
    name = "PortfolioCashflows",
    from_py_object
)]
#[derive(Clone)]
pub struct PyPortfolioCashflows {
    pub(crate) inner: PortfolioCashflows,
}

impl PyPortfolioCashflows {
    pub(crate) fn new(inner: PortfolioCashflows) -> Self {
        Self { inner }
    }
}

/// Warning emitted when a position's contractual cashflows could not be built.
#[pyclass(
    module = "finstack.portfolio",
    name = "CashflowWarning",
    from_py_object
)]
#[derive(Clone)]
pub struct PyCashflowWarning {
    pub(crate) inner: CashflowWarning,
}

impl PyCashflowWarning {
    pub(crate) fn new(inner: CashflowWarning) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyCashflowWarning {
    #[getter]
    fn position_id(&self) -> &str {
        self.inner.position_id.as_str()
    }

    #[getter]
    fn instrument_id(&self) -> &str {
        &self.inner.instrument_id
    }

    #[getter]
    fn instrument_type(&self) -> &str {
        &self.inner.instrument_type
    }

    #[getter]
    fn message(&self) -> &str {
        &self.inner.message
    }

    fn __repr__(&self) -> String {
        format!(
            "CashflowWarning(position_id='{}', instrument_id='{}', instrument_type='{}')",
            self.inner.position_id.as_str(),
            self.inner.instrument_id,
            self.inner.instrument_type
        )
    }
}

#[pymethods]
impl PyPortfolioCashflows {
    #[getter]
    fn by_date(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let outer = PyDict::new(py);
        for (date, per_ccy) in &self.inner.by_date {
            let ccy_map = PyDict::new(py);
            for (ccy, money) in per_ccy {
                ccy_map.set_item(ccy.to_string(), PyMoney::new(*money))?;
            }
            outer.set_item(date_to_py(py, *date)?, ccy_map)?;
        }
        Ok(outer.into())
    }

    #[getter]
    fn by_position(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let dict = PyDict::new(py);
        for (pos_id, flows) in &self.inner.by_position {
            let py_flows: Vec<Py<PyAny>> = flows
                .iter()
                .map(|(d, m)| {
                    let date_obj = date_to_py(py, *d)?;
                    let money_obj = Py::new(py, PyMoney::new(*m))?;
                    Ok(PyTuple::new(py, [date_obj, money_obj.into()])?.into())
                })
                .collect::<PyResult<Vec<Py<PyAny>>>>()?;
            dict.set_item(pos_id.as_str(), PyList::new(py, py_flows)?)?;
        }
        Ok(dict.into())
    }

    #[getter]
    fn warnings(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let warnings: Vec<Py<PyCashflowWarning>> = self
            .inner
            .warnings
            .iter()
            .cloned()
            .map(|warning| Py::new(py, PyCashflowWarning::new(warning)))
            .collect::<PyResult<Vec<_>>>()?;
        Ok(PyList::new(py, warnings)?.into())
    }

    fn __repr__(&self) -> String {
        format!(
            "PortfolioCashflows(dates={}, positions={}, warnings={})",
            self.inner.by_date.len(),
            self.inner.by_position.len(),
            self.inner.warnings.len()
        )
    }
}

/// Portfolio cashflows bucketed by reporting period.
#[pyclass(
    module = "finstack.portfolio",
    name = "PortfolioCashflowBuckets",
    from_py_object
)]
#[derive(Clone)]
pub struct PyPortfolioCashflowBuckets {
    pub(crate) inner: PortfolioCashflowBuckets,
}

impl PyPortfolioCashflowBuckets {
    pub(crate) fn new(inner: PortfolioCashflowBuckets) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyPortfolioCashflowBuckets {
    #[getter]
    fn by_period(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let dict = PyDict::new(py);
        for (period_id, money) in &self.inner.by_period {
            dict.set_item(period_id.to_string(), PyMoney::new(*money))?;
        }
        Ok(dict.into())
    }

    fn __repr__(&self) -> String {
        format!(
            "PortfolioCashflowBuckets(periods={})",
            self.inner.by_period.len()
        )
    }
}

fn extract_cashflows(value: &Bound<'_, PyAny>) -> PyResult<PortfolioCashflows> {
    if let Ok(py_cf) = value.extract::<PyRef<PyPortfolioCashflows>>() {
        Ok(py_cf.inner.clone())
    } else {
        Err(PyTypeError::new_err("Expected PortfolioCashflows"))
    }
}

/// Aggregate portfolio cashflows by date and currency.
#[pyfunction]
#[pyo3(signature = (portfolio, market_context))]
fn py_aggregate_cashflows(
    portfolio: &Bound<'_, PyAny>,
    market_context: &Bound<'_, PyAny>,
) -> PyResult<PyPortfolioCashflows> {
    let portfolio_inner = extract_portfolio(portfolio)?;
    let market = market_context.extract::<PyRef<PyMarketContext>>()?;
    let ladder = aggregate_cashflows(&portfolio_inner, &market.inner).map_err(portfolio_to_py)?;
    Ok(PyPortfolioCashflows::new(ladder))
}

/// Collapse a multi-currency cashflow ladder into base currency by date.
#[pyfunction]
#[pyo3(signature = (ladder, market_context, base_ccy))]
fn py_collapse_cashflows_to_base_by_date(
    ladder: &Bound<'_, PyAny>,
    market_context: &Bound<'_, PyAny>,
    base_ccy: &Bound<'_, PyAny>,
    py: Python<'_>,
) -> PyResult<Py<PyAny>> {
    let ladder_inner = extract_cashflows(ladder)?;
    let market = market_context.extract::<PyRef<PyMarketContext>>()?;
    let currency = extract_currency(base_ccy)?;

    let collapsed = collapse_cashflows_to_base_by_date(&ladder_inner, &market.inner, currency)
        .map_err(portfolio_to_py)?;

    let dict = PyDict::new(py);
    for (date, money) in collapsed {
        dict.set_item(date_to_py(py, date)?, PyMoney::new(money))?;
    }

    Ok(dict.into())
}

/// Bucket base-currency cashflows by reporting period.
#[pyfunction]
#[pyo3(signature = (ladder, market_context, base_ccy, periods))]
fn py_cashflows_to_base_by_period(
    ladder: &Bound<'_, PyAny>,
    market_context: &Bound<'_, PyAny>,
    base_ccy: &Bound<'_, PyAny>,
    periods: Vec<Py<PyPeriod>>,
) -> PyResult<PyPortfolioCashflowBuckets> {
    let ladder_inner = extract_cashflows(ladder)?;
    let market = market_context.extract::<PyRef<PyMarketContext>>()?;
    let currency = extract_currency(base_ccy)?;

    let mut rust_periods = Vec::with_capacity(periods.len());
    Python::attach(|py| {
        for period in periods {
            rust_periods.push(period.extract::<PyRef<PyPeriod>>(py)?.inner.clone());
        }
        Ok::<(), PyErr>(())
    })?;

    let buckets =
        cashflows_to_base_by_period(&ladder_inner, &market.inner, currency, &rust_periods)
            .map_err(portfolio_to_py)?;

    Ok(PyPortfolioCashflowBuckets::new(buckets))
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<String>> {
    parent.add_class::<PyCashflowWarning>()?;
    parent.add_class::<PyPortfolioCashflows>()?;
    parent.add_class::<PyPortfolioCashflowBuckets>()?;

    let agg = wrap_pyfunction!(py_aggregate_cashflows, parent)?;
    parent.add("aggregate_cashflows", agg)?;

    let collapse = wrap_pyfunction!(py_collapse_cashflows_to_base_by_date, parent)?;
    parent.add("collapse_cashflows_to_base_by_date", collapse)?;

    let bucket = wrap_pyfunction!(py_cashflows_to_base_by_period, parent)?;
    parent.add("cashflows_to_base_by_period", bucket)?;

    Ok(vec![
        "CashflowWarning".to_string(),
        "PortfolioCashflows".to_string(),
        "PortfolioCashflowBuckets".to_string(),
        "aggregate_cashflows".to_string(),
        "collapse_cashflows_to_base_by_date".to_string(),
        "cashflows_to_base_by_period".to_string(),
    ])
}
