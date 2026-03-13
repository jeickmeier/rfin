//! Forecast method bindings.

use crate::statements::utils::{json_to_py, py_to_json};
use finstack_statements::types::{ForecastMethod, ForecastSpec, SeasonalMode};
use indexmap::IndexMap;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyModule, PyType};
use pyo3::Bound;

/// Forecast method enumeration.
///
/// Defines how to forecast future values for a node.
#[pyclass(
    module = "finstack.statements.types",
    name = "ForecastMethod",
    frozen,
    from_py_object
)]
#[derive(Clone, Copy, Debug)]
pub struct PyForecastMethod {
    pub(crate) inner: ForecastMethod,
}

impl PyForecastMethod {
    pub(crate) fn new(inner: ForecastMethod) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyForecastMethod {
    #[classattr]
    const FORWARD_FILL: Self = Self {
        inner: ForecastMethod::ForwardFill,
    };
    #[classattr]
    const GROWTH_PCT: Self = Self {
        inner: ForecastMethod::GrowthPct,
    };
    #[classattr]
    const CURVE_PCT: Self = Self {
        inner: ForecastMethod::CurvePct,
    };
    #[classattr]
    const OVERRIDE: Self = Self {
        inner: ForecastMethod::Override,
    };
    #[classattr]
    const NORMAL: Self = Self {
        inner: ForecastMethod::Normal,
    };
    #[classattr]
    const LOG_NORMAL: Self = Self {
        inner: ForecastMethod::LogNormal,
    };
    #[classattr]
    const TIME_SERIES: Self = Self {
        inner: ForecastMethod::TimeSeries,
    };
    #[classattr]
    const SEASONAL: Self = Self {
        inner: ForecastMethod::Seasonal,
    };

    fn __repr__(&self) -> String {
        format!("ForecastMethod.{:?}", self.inner)
    }

    fn __str__(&self) -> String {
        format!("{:?}", self.inner)
    }
}

/// Seasonal forecast mode.
///
/// Determines how seasonal patterns are applied.
#[pyclass(
    module = "finstack.statements.types",
    name = "SeasonalMode",
    frozen,
    from_py_object
)]
#[derive(Clone, Copy, Debug)]
pub struct PySeasonalMode {
    pub(crate) inner: SeasonalMode,
}

#[pymethods]
impl PySeasonalMode {
    #[classattr]
    const ADDITIVE: Self = Self {
        inner: SeasonalMode::Additive,
    };
    #[classattr]
    const MULTIPLICATIVE: Self = Self {
        inner: SeasonalMode::Multiplicative,
    };

    fn __repr__(&self) -> String {
        format!("SeasonalMode.{:?}", self.inner)
    }
}

/// Forecast specification.
///
/// Defines how to forecast future values for a node using a specific method
/// and parameters.
#[pyclass(
    module = "finstack.statements.types",
    name = "ForecastSpec",
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyForecastSpec {
    pub(crate) inner: ForecastSpec,
}

impl PyForecastSpec {
    pub(crate) fn new(inner: ForecastSpec) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyForecastSpec {
    #[new]
    #[pyo3(text_signature = "(method, params=None)")]
    /// Create a forecast specification.
    ///
    /// Parameters
    /// ----------
    /// method : ForecastMethod
    ///     Forecast method to use
    /// params : dict, optional
    ///     Method-specific parameters
    ///
    /// Returns
    /// -------
    /// ForecastSpec
    ///     Forecast specification
    fn new_py(method: PyForecastMethod, params: Option<&Bound<'_, PyDict>>) -> PyResult<Self> {
        let params_map = if let Some(params_dict) = params {
            let mut map = IndexMap::new();
            for (key, value) in params_dict.iter() {
                let key_str: String = key.extract()?;
                let json_value = py_to_json(&value).map_err(|err| {
                    PyValueError::new_err(format!(
                        "Invalid params['{key_str}'] (must be JSON-serializable): {err}"
                    ))
                })?;
                map.insert(key_str, json_value);
            }
            map
        } else {
            IndexMap::new()
        };

        Ok(Self::new(ForecastSpec {
            method: method.inner,
            params: params_map,
        }))
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls)")]
    /// Create a forward-fill forecast (carry last value forward).
    ///
    /// Returns
    /// -------
    /// ForecastSpec
    ///     Forward-fill forecast spec
    fn forward_fill(_cls: &Bound<'_, PyType>) -> Self {
        Self::new(ForecastSpec::forward_fill())
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, rate)")]
    /// Create a growth percentage forecast.
    ///
    /// Parameters
    /// ----------
    /// rate : float
    ///     Growth rate (e.g., 0.05 for 5% growth)
    ///
    /// Returns
    /// -------
    /// ForecastSpec
    ///     Growth forecast spec
    fn growth(_cls: &Bound<'_, PyType>, rate: f64) -> Self {
        Self::new(ForecastSpec::growth(rate))
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, rates)")]
    /// Create a curve percentage forecast with period-specific rates.
    ///
    /// Parameters
    /// ----------
    /// rates : list[float]
    ///     Period-specific growth rates
    ///
    /// Returns
    /// -------
    /// ForecastSpec
    ///     Curve forecast spec
    fn curve(_cls: &Bound<'_, PyType>, rates: Vec<f64>) -> Self {
        Self::new(ForecastSpec::curve(rates))
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, mean, std, seed)")]
    /// Create a normal distribution forecast (deterministic with seed).
    ///
    /// Parameters
    /// ----------
    /// mean : float
    ///     Mean of the distribution
    /// std : float
    ///     Standard deviation
    /// seed : int
    ///     Random seed for determinism
    ///
    /// Returns
    /// -------
    /// ForecastSpec
    ///     Normal forecast spec
    fn normal(_cls: &Bound<'_, PyType>, mean: f64, std: f64, seed: u64) -> Self {
        Self::new(ForecastSpec::normal(mean, std, seed))
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, mean, std, seed)")]
    /// Create a log-normal distribution forecast (always positive).
    ///
    /// The generated values follow ``X = exp(mu + sigma * Z)`` where
    /// ``Z ~ N(0,1)``.  This means the **physical-space** expected value
    /// is ``E[X] = exp(mu + sigma^2 / 2)`` and the physical-space
    /// variance is ``(exp(sigma^2) - 1) * exp(2*mu + sigma^2)``.
    ///
    /// Parameters
    /// ----------
    /// mean : float
    ///     ``mu`` -- mean of the underlying **log-space** normal
    ///     distribution (not the expected value in physical space).
    /// std : float
    ///     ``sigma`` -- standard deviation in **log-space**.
    /// seed : int
    ///     Random seed for determinism
    ///
    /// Returns
    /// -------
    /// ForecastSpec
    ///     Log-normal forecast spec
    fn lognormal(_cls: &Bound<'_, PyType>, mean: f64, std: f64, seed: u64) -> Self {
        Self::new(ForecastSpec::lognormal(mean, std, seed))
    }

    #[getter]
    /// Get the forecast method.
    ///
    /// Returns
    /// -------
    /// ForecastMethod
    ///     Forecast method
    fn method(&self) -> PyForecastMethod {
        PyForecastMethod::new(self.inner.method)
    }

    #[getter]
    /// Get the forecast parameters.
    ///
    /// Returns
    /// -------
    /// dict
    ///     Parameters dictionary
    fn params(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let dict = PyDict::new(py);
        for (key, value) in &self.inner.params {
            let py_value = json_to_py(py, value)?;
            dict.set_item(key, py_value)?;
        }
        Ok(dict.into())
    }

    /// Convert to JSON string.
    ///
    /// Returns
    /// -------
    /// str
    ///     JSON representation
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner)
            .map_err(|e| PyValueError::new_err(format!("Serialization error: {}", e)))
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, json_str)")]
    /// Create from JSON string.
    ///
    /// Parameters
    /// ----------
    /// json_str : str
    ///     JSON string
    ///
    /// Returns
    /// -------
    /// ForecastSpec
    ///     Deserialized forecast spec
    fn from_json(_cls: &Bound<'_, PyType>, json_str: &str) -> PyResult<Self> {
        serde_json::from_str(json_str)
            .map(Self::new)
            .map_err(|e| PyValueError::new_err(format!("Deserialization error: {}", e)))
    }

    fn __repr__(&self) -> String {
        format!("ForecastSpec(method={:?})", self.inner.method)
    }
}

pub(crate) fn register<'py>(_py: Python<'py>, module: &Bound<'py, PyModule>) -> PyResult<()> {
    module.add_class::<PyForecastMethod>()?;
    module.add_class::<PySeasonalMode>()?;
    module.add_class::<PyForecastSpec>()?;
    Ok(())
}
