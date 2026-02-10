//! Python bindings for portfolio-level P&L attribution.

use crate::core::config::PyFinstackConfig;
use crate::core::dates::utils::py_to_date;
use crate::core::market_data::context::PyMarketContext;
use crate::core::money::PyMoney;
use crate::portfolio::error::portfolio_to_py;
use crate::portfolio::positions::extract_portfolio;
use crate::valuations::attribution::{
    PyAttributionMethod, PyCreditCurvesAttribution, PyPnlAttribution, PyRatesCurvesAttribution,
};
use finstack_portfolio::attribution::{attribute_portfolio_pnl, PortfolioAttribution};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict, PyModule};
use pyo3::Bound;

/// Portfolio-level P&L attribution result.
#[pyclass(module = "finstack.portfolio", name = "PortfolioAttribution")]
#[derive(Clone)]
pub struct PyPortfolioAttribution {
    pub(crate) inner: PortfolioAttribution,
}

impl PyPortfolioAttribution {
    pub(crate) fn new(inner: PortfolioAttribution) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyPortfolioAttribution {
    #[getter]
    fn total_pnl(&self) -> PyMoney {
        PyMoney::new(self.inner.total_pnl)
    }

    #[getter]
    fn carry(&self) -> PyMoney {
        PyMoney::new(self.inner.carry)
    }

    #[getter]
    fn rates_curves_pnl(&self) -> PyMoney {
        PyMoney::new(self.inner.rates_curves_pnl)
    }

    #[getter]
    fn credit_curves_pnl(&self) -> PyMoney {
        PyMoney::new(self.inner.credit_curves_pnl)
    }

    #[getter]
    fn inflation_curves_pnl(&self) -> PyMoney {
        PyMoney::new(self.inner.inflation_curves_pnl)
    }

    #[getter]
    fn correlations_pnl(&self) -> PyMoney {
        PyMoney::new(self.inner.correlations_pnl)
    }

    #[getter]
    fn fx_pnl(&self) -> PyMoney {
        PyMoney::new(self.inner.fx_pnl)
    }

    #[getter]
    fn fx_translation_pnl(&self) -> PyMoney {
        PyMoney::new(self.inner.fx_translation_pnl)
    }

    #[getter]
    fn vol_pnl(&self) -> PyMoney {
        PyMoney::new(self.inner.vol_pnl)
    }

    #[getter]
    fn model_params_pnl(&self) -> PyMoney {
        PyMoney::new(self.inner.model_params_pnl)
    }

    #[getter]
    fn market_scalars_pnl(&self) -> PyMoney {
        PyMoney::new(self.inner.market_scalars_pnl)
    }

    #[getter]
    fn residual(&self) -> PyMoney {
        PyMoney::new(self.inner.residual)
    }

    #[getter]
    fn by_position(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let dict = PyDict::new(py);
        for (pos_id, attr) in &self.inner.by_position {
            dict.set_item(
                pos_id.as_str(),
                PyPnlAttribution {
                    inner: attr.clone(),
                },
            )?;
        }
        Ok(dict.into())
    }

    #[getter]
    fn rates_detail(&self) -> Option<PyRatesCurvesAttribution> {
        self.inner
            .rates_detail
            .as_ref()
            .map(|d| PyRatesCurvesAttribution { inner: d.clone() })
    }

    #[getter]
    fn credit_detail(&self) -> Option<PyCreditCurvesAttribution> {
        self.inner
            .credit_detail
            .as_ref()
            .map(|d| PyCreditCurvesAttribution { inner: d.clone() })
    }

    #[getter]
    fn inflation_detail(&self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        if let Some(detail) = &self.inner.inflation_detail {
            let obj = pythonize::pythonize(py, detail)
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
            Ok(Some(obj.unbind()))
        } else {
            Ok(None)
        }
    }

    #[getter]
    fn correlations_detail(&self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        if let Some(detail) = &self.inner.correlations_detail {
            let obj = pythonize::pythonize(py, detail)
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
            Ok(Some(obj.unbind()))
        } else {
            Ok(None)
        }
    }

    #[getter]
    fn fx_detail(&self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        if let Some(detail) = &self.inner.fx_detail {
            let obj = pythonize::pythonize(py, detail)
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
            Ok(Some(obj.unbind()))
        } else {
            Ok(None)
        }
    }

    #[getter]
    fn vol_detail(&self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        if let Some(detail) = &self.inner.vol_detail {
            let obj = pythonize::pythonize(py, detail)
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
            Ok(Some(obj.unbind()))
        } else {
            Ok(None)
        }
    }

    #[getter]
    fn scalars_detail(&self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        if let Some(detail) = &self.inner.scalars_detail {
            let obj = pythonize::pythonize(py, detail)
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
            Ok(Some(obj.unbind()))
        } else {
            Ok(None)
        }
    }

    fn to_csv(&self) -> String {
        self.inner.to_csv()
    }

    fn position_detail_to_csv(&self) -> String {
        self.inner.position_detail_to_csv()
    }

    fn explain(&self) -> String {
        self.inner.explain()
    }

    fn __repr__(&self) -> String {
        format!("PortfolioAttribution(total_pnl={})", self.inner.total_pnl)
    }
}

/// Perform portfolio-level P&L attribution.
#[pyfunction]
#[pyo3(
    signature = (portfolio, market_t0, market_t1, as_of_t0, as_of_t1, method, config=None)
)]
fn py_attribute_portfolio_pnl(
    portfolio: &Bound<'_, PyAny>,
    market_t0: &Bound<'_, PyAny>,
    market_t1: &Bound<'_, PyAny>,
    as_of_t0: &Bound<'_, PyAny>,
    as_of_t1: &Bound<'_, PyAny>,
    method: &Bound<'_, PyAny>,
    config: Option<&Bound<'_, PyAny>>,
) -> PyResult<PyPortfolioAttribution> {
    let portfolio_inner = extract_portfolio(portfolio)?;
    let market0 = market_t0.extract::<PyRef<PyMarketContext>>()?;
    let market1 = market_t1.extract::<PyRef<PyMarketContext>>()?;
    let t0 = py_to_date(as_of_t0)?;
    let t1 = py_to_date(as_of_t1)?;
    let method_inner = method
        .extract::<PyRef<PyAttributionMethod>>()?
        .inner
        .clone();

    let cfg = if let Some(cfg_obj) = config {
        cfg_obj.extract::<PyRef<PyFinstackConfig>>()?.inner.clone()
    } else {
        finstack_core::config::FinstackConfig::default()
    };

    let attribution = attribute_portfolio_pnl(
        &portfolio_inner,
        &market0.inner,
        &market1.inner,
        t0,
        t1,
        &cfg,
        method_inner,
    )
    .map_err(portfolio_to_py)?;

    Ok(PyPortfolioAttribution::new(attribution))
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<String>> {
    parent.add_class::<PyPortfolioAttribution>()?;
    let wrapped_fn = wrap_pyfunction!(py_attribute_portfolio_pnl, parent)?;
    parent.add("attribute_portfolio_pnl", wrapped_fn)?;

    Ok(vec![
        "PortfolioAttribution".to_string(),
        "attribute_portfolio_pnl".to_string(),
    ])
}
