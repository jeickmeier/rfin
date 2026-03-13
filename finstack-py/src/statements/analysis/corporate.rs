//! Corporate DCF valuation bindings.

use crate::core::money::PyMoney;
use crate::statements::error::stmt_to_py;
use crate::statements::types::model::PyFinancialModelSpec;
use finstack_statements::analysis::corporate::{
    evaluate_dcf_with_market as rs_evaluate_dcf_with_market, CorporateValuationResult, DcfOptions,
};
use pyo3::prelude::*;
use pyo3::types::PyModule;
use pyo3::{wrap_pyfunction, Bound};

/// Optional configuration for DCF valuation.
#[pyclass(
    module = "finstack.statements.analysis",
    name = "DcfOptions",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyDcfOptions {
    pub(crate) inner: DcfOptions,
}

#[pymethods]
impl PyDcfOptions {
    #[new]
    #[pyo3(signature = (*, mid_year_convention=false, shares_outstanding=None))]
    fn new(mid_year_convention: bool, shares_outstanding: Option<f64>) -> Self {
        Self {
            inner: DcfOptions {
                mid_year_convention,
                equity_bridge: None,
                shares_outstanding,
                valuation_discounts: None,
            },
        }
    }

    #[getter]
    fn mid_year_convention(&self) -> bool {
        self.inner.mid_year_convention
    }

    #[getter]
    fn shares_outstanding(&self) -> Option<f64> {
        self.inner.shares_outstanding
    }

    fn __repr__(&self) -> String {
        format!(
            "DcfOptions(mid_year={}, shares={:?})",
            self.inner.mid_year_convention, self.inner.shares_outstanding
        )
    }
}

/// Corporate valuation result from DCF analysis.
#[pyclass(
    module = "finstack.statements.analysis",
    name = "CorporateValuationResult",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyCorporateValuationResult {
    pub(crate) inner: CorporateValuationResult,
}

impl PyCorporateValuationResult {
    pub(crate) fn new(inner: CorporateValuationResult) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyCorporateValuationResult {
    #[getter]
    fn equity_value(&self) -> PyMoney {
        PyMoney::new(self.inner.equity_value)
    }

    #[getter]
    fn enterprise_value(&self) -> PyMoney {
        PyMoney::new(self.inner.enterprise_value)
    }

    #[getter]
    fn net_debt(&self) -> PyMoney {
        PyMoney::new(self.inner.net_debt)
    }

    #[getter]
    fn terminal_value_pv(&self) -> PyMoney {
        PyMoney::new(self.inner.terminal_value_pv)
    }

    #[getter]
    fn equity_value_per_share(&self) -> Option<f64> {
        self.inner.equity_value_per_share
    }

    #[getter]
    fn diluted_shares(&self) -> Option<f64> {
        self.inner.diluted_shares
    }

    fn __repr__(&self) -> String {
        format!(
            "CorporateValuationResult(ev={}, equity={})",
            self.inner.enterprise_value, self.inner.equity_value
        )
    }
}

#[pyfunction]
#[pyo3(
    signature = (model, wacc, terminal_value, ufcf_node="ufcf", net_debt_override=None),
    name = "evaluate_dcf"
)]
/// Evaluate a financial model using DCF methodology.
///
/// Parameters
/// ----------
/// model : FinancialModelSpec
///     Financial model with forecast periods
/// wacc : float
///     Weighted average cost of capital (decimal, e.g., 0.10)
/// terminal_value : TerminalValueSpec
///     Terminal value specification
/// ufcf_node : str
///     Node ID containing UFCF values (default: "ufcf")
/// net_debt_override : float | None
///     Optional fixed net debt value
///
/// Returns
/// -------
/// CorporateValuationResult
///     DCF valuation results
fn py_evaluate_dcf(
    model: &PyFinancialModelSpec,
    wacc: f64,
    terminal_value: &crate::valuations::instruments::equity::dcf::PyTerminalValueSpec,
    ufcf_node: &str,
    net_debt_override: Option<f64>,
) -> PyResult<PyCorporateValuationResult> {
    let result = rs_evaluate_dcf_with_market(
        &model.inner,
        wacc,
        terminal_value.inner.clone(),
        ufcf_node,
        net_debt_override,
        &DcfOptions::default(),
        None,
    )
    .map_err(stmt_to_py)?;
    Ok(PyCorporateValuationResult::new(result))
}

#[pyfunction]
#[pyo3(
    signature = (model, wacc, terminal_value, ufcf_node="ufcf", net_debt_override=None, options=None),
    name = "evaluate_dcf_with_options"
)]
/// Evaluate DCF with additional options.
fn py_evaluate_dcf_with_options(
    model: &PyFinancialModelSpec,
    wacc: f64,
    terminal_value: &crate::valuations::instruments::equity::dcf::PyTerminalValueSpec,
    ufcf_node: &str,
    net_debt_override: Option<f64>,
    options: Option<&PyDcfOptions>,
) -> PyResult<PyCorporateValuationResult> {
    let opts = options.map(|o| o.inner.clone()).unwrap_or_default();
    let result = rs_evaluate_dcf_with_market(
        &model.inner,
        wacc,
        terminal_value.inner.clone(),
        ufcf_node,
        net_debt_override,
        &opts,
        None,
    )
    .map_err(stmt_to_py)?;
    Ok(PyCorporateValuationResult::new(result))
}

#[pyfunction]
#[pyo3(
    signature = (model, wacc, terminal_value, ufcf_node="ufcf", net_debt_override=None, options=None, market=None),
    name = "evaluate_dcf_with_market"
)]
/// Evaluate DCF with market context.
fn py_evaluate_dcf_with_market(
    model: &PyFinancialModelSpec,
    wacc: f64,
    terminal_value: &crate::valuations::instruments::equity::dcf::PyTerminalValueSpec,
    ufcf_node: &str,
    net_debt_override: Option<f64>,
    options: Option<&PyDcfOptions>,
    market: Option<&crate::core::market_data::context::PyMarketContext>,
) -> PyResult<PyCorporateValuationResult> {
    let opts = options.map(|o| o.inner.clone()).unwrap_or_default();
    let result = rs_evaluate_dcf_with_market(
        &model.inner,
        wacc,
        terminal_value.inner.clone(),
        ufcf_node,
        net_debt_override,
        &opts,
        market.map(|m| &m.inner),
    )
    .map_err(stmt_to_py)?;
    Ok(PyCorporateValuationResult::new(result))
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyDcfOptions>()?;
    module.add_class::<PyCorporateValuationResult>()?;
    module.add_function(wrap_pyfunction!(py_evaluate_dcf, module)?)?;
    module.add_function(wrap_pyfunction!(py_evaluate_dcf_with_options, module)?)?;
    module.add_function(wrap_pyfunction!(py_evaluate_dcf_with_market, module)?)?;
    Ok(vec![
        "DcfOptions",
        "CorporateValuationResult",
        "evaluate_dcf",
        "evaluate_dcf_with_options",
        "evaluate_dcf_with_market",
    ])
}
