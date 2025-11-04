//! Python bindings for P&L attribution.

use crate::core::utils::py_to_date;
use finstack_valuations::attribution::{AttributionFactor, AttributionMethod, PnlAttribution};
use pyo3::prelude::*;

/// Python wrapper for AttributionMethod.
#[pyclass(name = "AttributionMethod")]
#[derive(Clone)]
pub struct PyAttributionMethod {
    pub(crate) inner: AttributionMethod,
}

#[pymethods]
impl PyAttributionMethod {
    #[staticmethod]
    fn parallel() -> Self {
        Self {
            inner: AttributionMethod::Parallel,
        }
    }

    #[staticmethod]
    fn waterfall(factors: Vec<String>) -> PyResult<Self> {
        let factors: Vec<AttributionFactor> = factors
            .into_iter()
            .map(|s| match s.to_lowercase().as_str() {
                "carry" => Ok(AttributionFactor::Carry),
                "rates_curves" => Ok(AttributionFactor::RatesCurves),
                "credit_curves" => Ok(AttributionFactor::CreditCurves),
                "inflation_curves" => Ok(AttributionFactor::InflationCurves),
                "correlations" => Ok(AttributionFactor::Correlations),
                "fx" => Ok(AttributionFactor::Fx),
                "volatility" => Ok(AttributionFactor::Volatility),
                "model_parameters" => Ok(AttributionFactor::ModelParameters),
                "market_scalars" => Ok(AttributionFactor::MarketScalars),
                _ => Err(pyo3::exceptions::PyValueError::new_err(
                    format!("Unknown attribution factor: {}", s)
                )),
            })
            .collect::<PyResult<Vec<_>>>()?;

        Ok(Self {
            inner: AttributionMethod::Waterfall(factors),
        })
    }

    #[staticmethod]
    fn metrics_based() -> Self {
        Self {
            inner: AttributionMethod::MetricsBased,
        }
    }

    fn __repr__(&self) -> String {
        format!("{}", self.inner)
    }
}

/// Python wrapper for PnlAttribution.
#[pyclass(name = "PnlAttribution")]
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

    fn to_csv(&self) -> String {
        self.inner.to_csv()
    }

    fn explain(&self) -> String {
        self.inner.explain()
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

/// Python function to perform P&L attribution.
#[pyfunction]
#[pyo3(signature = (_instrument, _market_t0, _market_t1, as_of_t0, as_of_t1, _method=None))]
pub fn attribute_pnl(
    _instrument: Bound<'_, PyAny>,
    _market_t0: &crate::core::market_data::PyMarketContext,
    _market_t1: &crate::core::market_data::PyMarketContext,
    as_of_t0: Bound<'_, PyAny>,
    as_of_t1: Bound<'_, PyAny>,
    _method: Option<&PyAttributionMethod>,
) -> PyResult<PyPnlAttribution> {
    // Validate dates can be parsed
    let _date_t0 = py_to_date(&as_of_t0)?;
    let _date_t1 = py_to_date(&as_of_t1)?;
    
    // Extract instrument (this is a simplified approach, real implementation would
    // need to handle all instrument types)
    // For now, we'll return an error asking for proper instrument types
    Err(pyo3::exceptions::PyNotImplementedError::new_err(
        "P&L attribution requires instrument-specific bindings. \
         Please ensure all instrument types have proper Python bindings.",
    ))
}

/// Register attribution bindings with Python module.
pub fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<PyAttributionMethod>()?;
    module.add_class::<PyPnlAttribution>()?;
    module.add_function(wrap_pyfunction!(attribute_pnl, module)?)?;
    Ok(())
}

