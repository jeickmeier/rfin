use super::context::PyMarketContext;
use crate::core::currency::PyCurrency;
use crate::core::dates::utils::py_to_date;
use crate::errors::core_to_py;
use finstack_core::market_data::diff::{
    measure_bucketed_discount_shift, measure_correlation_shift, measure_discount_curve_shift,
    measure_fx_shift, measure_hazard_curve_shift, measure_inflation_curve_shift,
    measure_scalar_shift, measure_vol_surface_shift, TenorSamplingMethod, ATM_MONEYNESS,
    DEFAULT_VOL_EXPIRY, STANDARD_TENORS,
};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyList, PyModule, PyType};
use pyo3::Bound;

#[pyclass(
    module = "finstack.core.market_data.diff",
    name = "TenorSamplingMethod",
    frozen
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyTenorSamplingMethod {
    pub(crate) inner: TenorSamplingMethod,
}

#[pymethods]
impl PyTenorSamplingMethod {
    #[classattr]
    const STANDARD: Self = Self {
        inner: TenorSamplingMethod::Standard,
    };

    #[classattr]
    const DYNAMIC: Self = Self {
        inner: TenorSamplingMethod::Dynamic,
    };

    #[staticmethod]
    #[pyo3(text_signature = "(tenors)")]
    fn custom(tenors: Vec<f64>) -> Self {
        Self {
            inner: TenorSamplingMethod::Custom(tenors),
        }
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls)")]
    fn default(_cls: &Bound<'_, PyType>) -> Self {
        Self {
            inner: TenorSamplingMethod::default(),
        }
    }

    fn __repr__(&self) -> String {
        match &self.inner {
            TenorSamplingMethod::Standard => "TenorSamplingMethod.STANDARD".to_string(),
            TenorSamplingMethod::Dynamic => "TenorSamplingMethod.DYNAMIC".to_string(),
            TenorSamplingMethod::Custom(tenors) => {
                format!("TenorSamplingMethod.Custom({tenors:?})")
            }
        }
    }
}

#[pyfunction(name = "standard_tenors")]
fn standard_tenors_py() -> Vec<f64> {
    STANDARD_TENORS.to_vec()
}

#[pyfunction(name = "measure_discount_curve_shift")]
fn measure_discount_curve_shift_py(
    curve_id: &str,
    market_t0: &PyMarketContext,
    market_t1: &PyMarketContext,
    method: Option<PyRef<'_, PyTenorSamplingMethod>>,
) -> PyResult<f64> {
    let sampling = method.map_or(TenorSamplingMethod::Standard, |m| m.inner.clone());
    measure_discount_curve_shift(curve_id, &market_t0.inner, &market_t1.inner, sampling)
        .map_err(core_to_py)
}

#[pyfunction(name = "measure_bucketed_discount_shift")]
fn measure_bucketed_discount_shift_py(
    curve_id: &str,
    market_t0: &PyMarketContext,
    market_t1: &PyMarketContext,
    tenors: Vec<f64>,
) -> PyResult<Vec<(f64, f64)>> {
    measure_bucketed_discount_shift(curve_id, &market_t0.inner, &market_t1.inner, &tenors)
        .map_err(core_to_py)
}

#[pyfunction(name = "measure_hazard_curve_shift")]
fn measure_hazard_curve_shift_py(
    curve_id: &str,
    market_t0: &PyMarketContext,
    market_t1: &PyMarketContext,
    method: Option<PyRef<'_, PyTenorSamplingMethod>>,
) -> PyResult<f64> {
    let sampling = method.map_or(TenorSamplingMethod::Standard, |m| m.inner.clone());
    measure_hazard_curve_shift(curve_id, &market_t0.inner, &market_t1.inner, sampling)
        .map_err(core_to_py)
}

#[pyfunction(name = "measure_inflation_curve_shift")]
fn measure_inflation_curve_shift_py(
    curve_id: &str,
    market_t0: &PyMarketContext,
    market_t1: &PyMarketContext,
) -> PyResult<f64> {
    measure_inflation_curve_shift(curve_id, &market_t0.inner, &market_t1.inner).map_err(core_to_py)
}

#[pyfunction(name = "measure_correlation_shift")]
fn measure_correlation_shift_py(
    curve_id: &str,
    market_t0: &PyMarketContext,
    market_t1: &PyMarketContext,
) -> PyResult<f64> {
    measure_correlation_shift(curve_id, &market_t0.inner, &market_t1.inner).map_err(core_to_py)
}

#[pyfunction(
    name = "measure_vol_surface_shift",
    signature = (surface_id, market_t0, market_t1, reference_expiry=None, reference_strike=None)
)]
fn measure_vol_surface_shift_py(
    surface_id: &str,
    market_t0: &PyMarketContext,
    market_t1: &PyMarketContext,
    reference_expiry: Option<f64>,
    reference_strike: Option<f64>,
) -> PyResult<f64> {
    measure_vol_surface_shift(
        surface_id,
        &market_t0.inner,
        &market_t1.inner,
        reference_expiry,
        reference_strike,
    )
    .map_err(core_to_py)
}

#[pyfunction(name = "measure_fx_shift")]
fn measure_fx_shift_py(
    base_currency: PyRef<'_, PyCurrency>,
    quote_currency: PyRef<'_, PyCurrency>,
    market_t0: &PyMarketContext,
    market_t1: &PyMarketContext,
    as_of_t0: &Bound<'_, PyAny>,
    as_of_t1: &Bound<'_, PyAny>,
) -> PyResult<f64> {
    let t0 = py_to_date(as_of_t0)?;
    let t1 = py_to_date(as_of_t1)?;

    measure_fx_shift(
        base_currency.inner,
        quote_currency.inner,
        &market_t0.inner,
        &market_t1.inner,
        t0,
        t1,
    )
    .map_err(core_to_py)
}

#[pyfunction(name = "measure_scalar_shift")]
fn measure_scalar_shift_py(
    scalar_id: &str,
    market_t0: &PyMarketContext,
    market_t1: &PyMarketContext,
) -> PyResult<f64> {
    measure_scalar_shift(scalar_id, &market_t0.inner, &market_t1.inner).map_err(core_to_py)
}

pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "diff")?;
    module.setattr(
        "__doc__",
        "Market data comparison helpers mirroring finstack-core market_data.diff.",
    )?;
    module.add_class::<PyTenorSamplingMethod>()?;
    module.add_function(wrap_pyfunction!(standard_tenors_py, &module)?)?;
    module.add_function(wrap_pyfunction!(measure_discount_curve_shift_py, &module)?)?;
    module.add_function(wrap_pyfunction!(
        measure_bucketed_discount_shift_py,
        &module
    )?)?;
    module.add_function(wrap_pyfunction!(measure_hazard_curve_shift_py, &module)?)?;
    module.add_function(wrap_pyfunction!(measure_inflation_curve_shift_py, &module)?)?;
    module.add_function(wrap_pyfunction!(measure_correlation_shift_py, &module)?)?;
    module.add_function(wrap_pyfunction!(measure_vol_surface_shift_py, &module)?)?;
    module.add_function(wrap_pyfunction!(measure_fx_shift_py, &module)?)?;
    module.add_function(wrap_pyfunction!(measure_scalar_shift_py, &module)?)?;

    module.setattr("ATM_MONEYNESS", ATM_MONEYNESS)?;
    module.setattr("DEFAULT_VOL_EXPIRY", DEFAULT_VOL_EXPIRY)?;
    module.setattr("STANDARD_TENORS", PyList::new(py, STANDARD_TENORS)?)?;

    let exports = [
        "TenorSamplingMethod",
        "standard_tenors",
        "measure_discount_curve_shift",
        "measure_bucketed_discount_shift",
        "measure_hazard_curve_shift",
        "measure_inflation_curve_shift",
        "measure_correlation_shift",
        "measure_vol_surface_shift",
        "measure_fx_shift",
        "measure_scalar_shift",
        "ATM_MONEYNESS",
        "DEFAULT_VOL_EXPIRY",
        "STANDARD_TENORS",
    ];
    module.setattr("__all__", PyList::new(py, exports)?)?;

    parent.add_submodule(&module)?;
    Ok(exports.to_vec())
}
