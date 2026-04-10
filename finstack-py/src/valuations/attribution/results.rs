use super::types::{
    PyAttributionMeta, PyCarryDetail, PyCorrelationsAttribution, PyCreditCurvesAttribution,
    PyCrossFactorDetail, PyFxAttribution, PyInflationCurvesAttribution, PyModelParamsAttribution,
    PyRatesCurvesAttribution, PyScalarsAttribution, PyVolAttribution,
};
use crate::errors::core_to_py;
use finstack_core::HashMap;
use finstack_valuations::attribution::PnlAttribution;
use pyo3::prelude::*;

/// Python wrapper for PnlAttribution.
#[pyclass(name = "PnlAttribution", from_py_object)]
#[derive(Clone)]
pub struct PyPnlAttribution {
    pub(crate) inner: PnlAttribution,
}

#[pymethods]
impl PyPnlAttribution {
    #[getter]
    fn total_pnl(&self) -> crate::core::money::PyMoney {
        crate::core::money::PyMoney {
            inner: self.inner.total_pnl,
        }
    }

    #[getter]
    fn carry(&self) -> crate::core::money::PyMoney {
        crate::core::money::PyMoney {
            inner: self.inner.carry,
        }
    }

    #[getter]
    fn rates_curves_pnl(&self) -> crate::core::money::PyMoney {
        crate::core::money::PyMoney {
            inner: self.inner.rates_curves_pnl,
        }
    }

    #[getter]
    fn credit_curves_pnl(&self) -> crate::core::money::PyMoney {
        crate::core::money::PyMoney {
            inner: self.inner.credit_curves_pnl,
        }
    }

    #[getter]
    fn inflation_curves_pnl(&self) -> crate::core::money::PyMoney {
        crate::core::money::PyMoney {
            inner: self.inner.inflation_curves_pnl,
        }
    }

    #[getter]
    fn correlations_pnl(&self) -> crate::core::money::PyMoney {
        crate::core::money::PyMoney {
            inner: self.inner.correlations_pnl,
        }
    }

    #[getter]
    fn fx_pnl(&self) -> crate::core::money::PyMoney {
        crate::core::money::PyMoney {
            inner: self.inner.fx_pnl,
        }
    }

    #[getter]
    fn vol_pnl(&self) -> crate::core::money::PyMoney {
        crate::core::money::PyMoney {
            inner: self.inner.vol_pnl,
        }
    }

    #[getter]
    fn model_params_pnl(&self) -> crate::core::money::PyMoney {
        crate::core::money::PyMoney {
            inner: self.inner.model_params_pnl,
        }
    }

    #[getter]
    fn market_scalars_pnl(&self) -> crate::core::money::PyMoney {
        crate::core::money::PyMoney {
            inner: self.inner.market_scalars_pnl,
        }
    }

    #[getter]
    fn residual(&self) -> crate::core::money::PyMoney {
        crate::core::money::PyMoney {
            inner: self.inner.residual,
        }
    }

    #[getter]
    fn meta(&self) -> PyAttributionMeta {
        PyAttributionMeta {
            inner: self.inner.meta.clone(),
        }
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
    fn model_params_detail(&self) -> Option<PyModelParamsAttribution> {
        self.inner
            .model_params_detail
            .as_ref()
            .map(|d| PyModelParamsAttribution { inner: d.clone() })
    }

    #[getter]
    fn carry_detail(&self) -> Option<PyCarryDetail> {
        self.inner
            .carry_detail
            .as_ref()
            .map(|d| PyCarryDetail { inner: d.clone() })
    }

    #[getter]
    fn inflation_detail(&self) -> Option<PyInflationCurvesAttribution> {
        self.inner
            .inflation_detail
            .as_ref()
            .map(|d| PyInflationCurvesAttribution { inner: d.clone() })
    }

    #[getter]
    fn correlations_detail(&self) -> Option<PyCorrelationsAttribution> {
        self.inner
            .correlations_detail
            .as_ref()
            .map(|d| PyCorrelationsAttribution { inner: d.clone() })
    }

    #[getter]
    fn fx_detail(&self) -> Option<PyFxAttribution> {
        self.inner
            .fx_detail
            .as_ref()
            .map(|d| PyFxAttribution { inner: d.clone() })
    }

    #[getter]
    fn vol_detail(&self) -> Option<PyVolAttribution> {
        self.inner
            .vol_detail
            .as_ref()
            .map(|d| PyVolAttribution { inner: d.clone() })
    }

    #[getter]
    fn cross_factor_pnl(&self) -> crate::core::money::PyMoney {
        crate::core::money::PyMoney {
            inner: self.inner.cross_factor_pnl,
        }
    }

    #[getter]
    fn cross_factor_detail(&self) -> Option<PyCrossFactorDetail> {
        self.inner
            .cross_factor_detail
            .as_ref()
            .map(|d| PyCrossFactorDetail { inner: d.clone() })
    }

    #[getter]
    fn scalars_detail(&self) -> Option<PyScalarsAttribution> {
        self.inner
            .scalars_detail
            .as_ref()
            .map(|d| PyScalarsAttribution { inner: d.clone() })
    }

    fn to_csv(&self) -> String {
        self.inner.to_csv()
    }

    fn to_json(&self) -> PyResult<String> {
        self.inner.to_json().map_err(core_to_py)
    }

    fn rates_detail_to_csv(&self) -> Option<String> {
        self.inner.rates_detail_to_csv()
    }

    fn explain(&self) -> String {
        self.inner.explain()
    }

    fn residual_within_tolerance(&self, pct_tolerance: f64, abs_tolerance: f64) -> bool {
        self.inner
            .residual_within_tolerance(pct_tolerance, abs_tolerance)
    }

    fn __repr__(&self) -> String {
        format!(
            "PnlAttribution(total={}, carry={}, rates={}, residual={})",
            self.inner.total_pnl,
            self.inner.carry,
            self.inner.rates_curves_pnl,
            self.inner.residual
        )
    }
}

/// Python wrapper for PortfolioAttribution.
#[pyclass(name = "PortfolioAttribution", from_py_object)]
#[derive(Clone)]
pub struct PyPortfolioAttribution {
    pub(crate) inner: finstack_portfolio::PortfolioAttribution,
}

#[pymethods]
impl PyPortfolioAttribution {
    #[getter]
    fn total_pnl(&self) -> crate::core::money::PyMoney {
        crate::core::money::PyMoney {
            inner: self.inner.total_pnl,
        }
    }

    #[getter]
    fn carry(&self) -> crate::core::money::PyMoney {
        crate::core::money::PyMoney {
            inner: self.inner.carry,
        }
    }

    #[getter]
    fn rates_curves_pnl(&self) -> crate::core::money::PyMoney {
        crate::core::money::PyMoney {
            inner: self.inner.rates_curves_pnl,
        }
    }

    #[getter]
    fn credit_curves_pnl(&self) -> crate::core::money::PyMoney {
        crate::core::money::PyMoney {
            inner: self.inner.credit_curves_pnl,
        }
    }

    #[getter]
    fn inflation_curves_pnl(&self) -> crate::core::money::PyMoney {
        crate::core::money::PyMoney {
            inner: self.inner.inflation_curves_pnl,
        }
    }

    #[getter]
    fn correlations_pnl(&self) -> crate::core::money::PyMoney {
        crate::core::money::PyMoney {
            inner: self.inner.correlations_pnl,
        }
    }

    #[getter]
    fn fx_pnl(&self) -> crate::core::money::PyMoney {
        crate::core::money::PyMoney {
            inner: self.inner.fx_pnl,
        }
    }

    #[getter]
    fn vol_pnl(&self) -> crate::core::money::PyMoney {
        crate::core::money::PyMoney {
            inner: self.inner.vol_pnl,
        }
    }

    #[getter]
    fn model_params_pnl(&self) -> crate::core::money::PyMoney {
        crate::core::money::PyMoney {
            inner: self.inner.model_params_pnl,
        }
    }

    #[getter]
    fn market_scalars_pnl(&self) -> crate::core::money::PyMoney {
        crate::core::money::PyMoney {
            inner: self.inner.market_scalars_pnl,
        }
    }

    #[getter]
    fn residual(&self) -> crate::core::money::PyMoney {
        crate::core::money::PyMoney {
            inner: self.inner.residual,
        }
    }

    fn by_position_to_dict(&self) -> HashMap<String, PyPnlAttribution> {
        self.inner
            .by_position
            .iter()
            .map(|(k, v)| (k.to_string(), PyPnlAttribution { inner: v.clone() }))
            .collect()
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
        format!(
            "PortfolioAttribution(total={}, positions={})",
            self.inner.total_pnl,
            self.inner.by_position.len()
        )
    }
}
