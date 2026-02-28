use crate::core::common::labels::normalize_label;
use finstack_core::math::stats::{
    correlation as core_correlation, covariance as core_covariance,
    log_returns as core_log_returns, mean as core_mean, mean_var as core_mean_var,
    realized_variance as core_realized_variance,
    realized_variance_ohlc as core_realized_variance_ohlc, variance as core_variance,
    RealizedVarMethod,
};
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule, PyType};
use pyo3::Bound;

#[pyclass(
    name = "RealizedVarMethod",
    module = "finstack.core.math.stats",
    frozen,
    from_py_object
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Methods for calculating realized variance from price or OHLC series.
pub struct PyRealizedVarMethod {
    pub(crate) inner: RealizedVarMethod,
}

impl PyRealizedVarMethod {
    pub(crate) const fn new(inner: RealizedVarMethod) -> Self {
        Self { inner }
    }

    fn label(&self) -> &'static str {
        match self.inner {
            RealizedVarMethod::CloseToClose => "close_to_close",
            RealizedVarMethod::Parkinson => "parkinson",
            RealizedVarMethod::GarmanKlass => "garman_klass",
            RealizedVarMethod::RogersSatchell => "rogers_satchell",
            RealizedVarMethod::YangZhang => "yang_zhang",
        }
    }
}

fn parse_realized_var_method_name(name: &str) -> PyResult<RealizedVarMethod> {
    match normalize_label(name).as_str() {
        "close_to_close" | "close" => Ok(RealizedVarMethod::CloseToClose),
        "parkinson" => Ok(RealizedVarMethod::Parkinson),
        "garman_klass" | "garmanklass" => Ok(RealizedVarMethod::GarmanKlass),
        "rogers_satchell" => Ok(RealizedVarMethod::RogersSatchell),
        "yang_zhang" => Ok(RealizedVarMethod::YangZhang),
        other => Err(PyValueError::new_err(format!(
            "Unknown realized variance method: {other}",
        ))),
    }
}

fn parse_realized_var_method(method: Option<Bound<'_, PyAny>>) -> PyResult<RealizedVarMethod> {
    if let Some(obj) = method {
        if let Ok(wrapper) = obj.extract::<PyRef<PyRealizedVarMethod>>() {
            return Ok(wrapper.inner);
        }
        if let Ok(name) = obj.extract::<&str>() {
            return parse_realized_var_method_name(name);
        }
        return Err(PyTypeError::new_err(
            "Expected RealizedVarMethod or string label",
        ));
    }
    Ok(RealizedVarMethod::CloseToClose)
}

#[pymethods]
impl PyRealizedVarMethod {
    #[classattr]
    const CLOSE_TO_CLOSE: Self = Self {
        inner: RealizedVarMethod::CloseToClose,
    };

    #[classattr]
    const PARKINSON: Self = Self {
        inner: RealizedVarMethod::Parkinson,
    };

    #[classattr]
    const GARMAN_KLASS: Self = Self {
        inner: RealizedVarMethod::GarmanKlass,
    };

    #[classattr]
    const ROGERS_SATCHELL: Self = Self {
        inner: RealizedVarMethod::RogersSatchell,
    };

    #[classattr]
    const YANG_ZHANG: Self = Self {
        inner: RealizedVarMethod::YangZhang,
    };

    #[classmethod]
    #[pyo3(text_signature = "(cls, name)")]
    /// Parse a realized variance method from a snake-/kebab-case label.
    fn from_name(_cls: &Bound<'_, PyType>, name: &str) -> PyResult<Self> {
        Ok(Self::new(parse_realized_var_method_name(name)?))
    }

    #[getter]
    /// Snake-case label for this method.
    fn name(&self) -> &'static str {
        self.label()
    }

    fn __repr__(&self) -> String {
        format!("RealizedVarMethod('{}')", self.label())
    }

    fn __str__(&self) -> &'static str {
        self.label()
    }

    fn __hash__(&self) -> isize {
        self.inner as isize
    }
}

#[pyfunction(name = "mean")]
#[pyo3(text_signature = "(data)")]
pub fn mean_py(data: Vec<f64>) -> PyResult<f64> {
    if data.is_empty() {
        return Err(PyValueError::new_err("Data must not be empty"));
    }
    Ok(core_mean(&data))
}

#[pyfunction(name = "variance")]
#[pyo3(text_signature = "(data)")]
pub fn variance_py(data: Vec<f64>) -> PyResult<f64> {
    if data.len() < 2 {
        return Err(PyValueError::new_err("Data must have at least 2 elements"));
    }
    Ok(core_variance(&data))
}

#[pyfunction(name = "covariance")]
#[pyo3(text_signature = "(x, y)")]
pub fn covariance_py(x: Vec<f64>, y: Vec<f64>) -> PyResult<f64> {
    if x.len() != y.len() {
        return Err(PyValueError::new_err("Input arrays must have same length"));
    }
    if x.len() < 2 {
        return Err(PyValueError::new_err("Data must have at least 2 elements"));
    }
    Ok(core_covariance(&x, &y))
}

#[pyfunction(name = "correlation")]
#[pyo3(text_signature = "(x, y)")]
pub fn correlation_py(x: Vec<f64>, y: Vec<f64>) -> PyResult<f64> {
    if x.len() != y.len() {
        return Err(PyValueError::new_err("Input arrays must have same length"));
    }
    if x.len() < 2 {
        return Err(PyValueError::new_err("Data must have at least 2 elements"));
    }
    Ok(core_correlation(&x, &y))
}

#[pyfunction(name = "mean_var")]
#[pyo3(text_signature = "(data)")]
/// Return the (mean, variance) pair for a data series.
pub fn mean_var_py(data: Vec<f64>) -> PyResult<(f64, f64)> {
    if data.is_empty() {
        return Err(PyValueError::new_err("Data must not be empty"));
    }
    Ok(core_mean_var(&data))
}

#[pyfunction(name = "log_returns")]
#[pyo3(text_signature = "(prices)")]
/// Compute log returns from a price series.
///
/// Parameters
/// ----------
/// prices : list[float]
///     Price series ordered in time.
///
/// Returns
/// -------
/// list[float]
///     Log returns ``ln(P_t / P_{t-1})``. Empty list if fewer than 2 prices.
pub fn log_returns_py(prices: Vec<f64>) -> PyResult<Vec<f64>> {
    Ok(core_log_returns(&prices))
}

#[pyfunction(
    name = "realized_variance",
    text_signature = "(prices, method='close_to_close', annualization_factor=252.0)"
)]
/// Calculate realized variance from a close price series.
///
/// Parameters
/// ----------
/// prices : list[float]
///     Close prices ordered in time.
/// method : RealizedVarMethod or str, optional
///     Estimation method (default ``'close_to_close'``).
/// annualization_factor : float, optional
///     Scaling factor for annualization (default ``252.0``).
pub fn realized_variance_py(
    prices: Vec<f64>,
    method: Option<Bound<'_, PyAny>>,
    annualization_factor: Option<f64>,
) -> PyResult<f64> {
    let m = parse_realized_var_method(method)?;
    let af = annualization_factor.unwrap_or(252.0);
    Ok(core_realized_variance(&prices, m, af))
}

#[pyfunction(
    name = "realized_variance_ohlc",
    text_signature = "(open, high, low, close, method='yang_zhang', annualization_factor=252.0)"
)]
/// Calculate realized variance from OHLC data using advanced estimators.
///
/// Parameters
/// ----------
/// open, high, low, close : list[float]
///     OHLC price series of equal length.
/// method : RealizedVarMethod or str, optional
///     Estimation method (default ``'yang_zhang'``).
/// annualization_factor : float, optional
///     Scaling factor for annualization (default ``252.0``).
pub fn realized_variance_ohlc_py(
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    method: Option<Bound<'_, PyAny>>,
    annualization_factor: Option<f64>,
) -> PyResult<f64> {
    let n = open.len();
    if high.len() != n || low.len() != n || close.len() != n {
        return Err(PyValueError::new_err(
            "open, high, low, and close must have the same length",
        ));
    }
    let m = parse_realized_var_method(method)?;
    let af = annualization_factor.unwrap_or(252.0);
    Ok(core_realized_variance_ohlc(
        &open, &high, &low, &close, m, af,
    ))
}

pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "stats")?;
    module.setattr(
        "__doc__",
        "Statistical helpers (means, variances, covariances, realized variance).",
    )?;
    module.add_class::<PyRealizedVarMethod>()?;
    module.add_function(wrap_pyfunction!(mean_py, &module)?)?;
    module.add_function(wrap_pyfunction!(variance_py, &module)?)?;
    module.add_function(wrap_pyfunction!(covariance_py, &module)?)?;
    module.add_function(wrap_pyfunction!(correlation_py, &module)?)?;
    module.add_function(wrap_pyfunction!(mean_var_py, &module)?)?;
    module.add_function(wrap_pyfunction!(log_returns_py, &module)?)?;
    module.add_function(wrap_pyfunction!(realized_variance_py, &module)?)?;
    module.add_function(wrap_pyfunction!(realized_variance_ohlc_py, &module)?)?;

    let exports = [
        "RealizedVarMethod",
        "mean",
        "variance",
        "covariance",
        "correlation",
        "mean_var",
        "log_returns",
        "realized_variance",
        "realized_variance_ohlc",
    ];
    module.setattr("__all__", PyList::new(py, exports)?)?;
    parent.add_submodule(&module)?;
    Ok(exports.to_vec())
}
