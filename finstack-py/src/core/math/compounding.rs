//! Python bindings for compounding conventions.

use crate::errors::map_error;
use finstack_core::math::Compounding;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyModule;
use pyo3::Bound;
use std::num::NonZeroU32;

/// Compounding convention for interest rates.
#[pyclass(
    name = "Compounding",
    module = "finstack.core.math",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyCompounding {
    /// Wrapped Rust [`Compounding`] value.
    pub(crate) inner: Compounding,
}

impl PyCompounding {
    pub(crate) fn from_inner(inner: Compounding) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyCompounding {
    /// Create a Compounding convention.
    ///
    /// Args:
    ///     kind: One of "continuous", "annual", "simple", or "periodic"
    ///     periods_per_year: Required when kind is "periodic"
    #[new]
    #[pyo3(signature = (kind, periods_per_year=None))]
    fn new(kind: &str, periods_per_year: Option<u32>) -> PyResult<Self> {
        let inner = match kind.to_lowercase().as_str() {
            "continuous" => Compounding::Continuous,
            "annual" => Compounding::Annual,
            "simple" => Compounding::Simple,
            "semi_annual" | "semiannual" | "semi-annual" => Compounding::SEMI_ANNUAL,
            "quarterly" => Compounding::QUARTERLY,
            "monthly" => Compounding::MONTHLY,
            "periodic" => {
                let n = periods_per_year.ok_or_else(|| {
                    PyValueError::new_err("periods_per_year required for periodic compounding")
                })?;
                let nz = NonZeroU32::new(n)
                    .ok_or_else(|| PyValueError::new_err("periods_per_year must be > 0"))?;
                Compounding::Periodic(nz)
            }
            other => {
                return Err(PyValueError::new_err(format!(
                    "Unknown compounding kind: '{}'. Expected one of: continuous, annual, \
                     simple, semi_annual, quarterly, monthly, periodic",
                    other
                )));
            }
        };
        Ok(Self { inner })
    }

    /// Continuous compounding convention.
    #[staticmethod]
    fn continuous() -> Self {
        Self::from_inner(Compounding::Continuous)
    }

    /// Annual compounding convention.
    #[staticmethod]
    fn annual() -> Self {
        Self::from_inner(Compounding::Annual)
    }

    /// Simple interest (no compounding).
    #[staticmethod]
    fn simple() -> Self {
        Self::from_inner(Compounding::Simple)
    }

    /// Semi-annual compounding (n=2).
    #[staticmethod]
    fn semi_annual() -> Self {
        Self::from_inner(Compounding::SEMI_ANNUAL)
    }

    /// Quarterly compounding (n=4).
    #[staticmethod]
    fn quarterly() -> Self {
        Self::from_inner(Compounding::QUARTERLY)
    }

    /// Monthly compounding (n=12).
    #[staticmethod]
    fn monthly() -> Self {
        Self::from_inner(Compounding::MONTHLY)
    }

    /// Periodic compounding with custom frequency.
    #[staticmethod]
    fn periodic(n: u32) -> PyResult<Self> {
        let nz = NonZeroU32::new(n)
            .ok_or_else(|| PyValueError::new_err("periods_per_year must be > 0"))?;
        Ok(Self::from_inner(Compounding::Periodic(nz)))
    }

    /// Number of compounding periods per year, if applicable.
    #[getter]
    fn get_periods_per_year(&self) -> Option<u32> {
        self.inner.periods_per_year()
    }

    /// Whether this is a periodic compounding convention (including annual).
    #[getter]
    fn get_is_periodic(&self) -> bool {
        self.inner.is_periodic()
    }

    /// Convert an interest rate to a discount factor for time t (in years).
    fn df_from_rate(&self, rate: f64, t: f64) -> f64 {
        self.inner.df_from_rate(rate, t)
    }

    /// Convert a discount factor to an interest rate for time t (in years).
    fn rate_from_df(&self, df: f64, t: f64) -> f64 {
        self.inner.rate_from_df(df, t)
    }

    /// Fallible version of rate_from_df. Raises on degenerate inputs.
    fn try_rate_from_df(&self, df: f64, t: f64) -> PyResult<f64> {
        self.inner.try_rate_from_df(df, t).map_err(map_error)
    }

    /// Convert a rate from this convention to another.
    fn convert_rate(&self, rate: f64, t: f64, to: &PyCompounding) -> f64 {
        self.inner.convert_rate(rate, t, &to.inner)
    }

    fn __repr__(&self) -> String {
        format!("Compounding({})", self.inner)
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }

    fn __hash__(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.inner.hash(&mut hasher);
        hasher.finish()
    }
}

pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    parent.add_class::<PyCompounding>()?;
    let _ = py;
    Ok(vec!["Compounding"])
}
