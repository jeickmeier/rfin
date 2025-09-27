use crate::currency::extract_currency;
use crate::error::unknown_rounding_mode;
use finstack_core::config::{FinstackConfig, RoundingMode};
use pyo3::basic::CompareOp;
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyAnyMethods, PyList, PyModule, PyModuleMethods, PyType};
use pyo3::{Bound, IntoPyObjectExt};
use std::fmt;

/// Finstack numeric configuration (rounding and currency scales).
#[pyclass(name = "FinstackConfig", module = "finstack.config")]
#[derive(Clone)]
pub struct PyFinstackConfig {
    pub(crate) inner: FinstackConfig,
}

impl PyFinstackConfig {
    pub(crate) fn new(inner: FinstackConfig) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyFinstackConfig {
    #[new]
    #[pyo3(text_signature = "()")]
    /// Create a configuration with the default rounding rules.
    fn ctor() -> Self {
        Self::new(FinstackConfig::default())
    }

    /// Clone this configuration.
    #[pyo3(text_signature = "(self)")]
    fn copy(&self) -> Self {
        Self::new(self.inner.clone())
    }

    /// Active rounding mode.
    #[getter]
    /// Active rounding mode for display/output formatting.
    fn rounding_mode(&self) -> PyRoundingMode {
        PyRoundingMode::new(self.inner.rounding.mode)
    }

    /// Update the rounding mode.
    #[pyo3(text_signature = "(self, mode)")]
    fn set_rounding_mode(&mut self, mode: Bound<'_, PyAny>) -> PyResult<()> {
        self.inner.rounding.mode = extract_rounding_mode(&mode)?;
        Ok(())
    }

    /// Get the ingest decimal scale for a currency.
    #[pyo3(text_signature = "(self, currency)")]
    /// Decimal places accepted when ingesting source data for `currency`.
    fn ingest_scale(&self, currency: Bound<'_, PyAny>) -> PyResult<u32> {
        let ccy = extract_currency(currency.as_ref())?;
        Ok(self.inner.ingest_scale(ccy))
    }

    /// Override the ingest decimal scale for a currency.
    #[pyo3(text_signature = "(self, currency, decimals)")]
    fn set_ingest_scale(&mut self, currency: Bound<'_, PyAny>, decimals: u32) -> PyResult<()> {
        let ccy = extract_currency(currency.as_ref())?;
        self.inner
            .rounding
            .ingest_scale
            .overrides
            .insert(ccy, decimals);
        Ok(())
    }

    /// Get the output decimal scale for a currency.
    #[pyo3(text_signature = "(self, currency)")]
    /// Decimal places used when exporting values for `currency`.
    fn output_scale(&self, currency: Bound<'_, PyAny>) -> PyResult<u32> {
        let ccy = extract_currency(currency.as_ref())?;
        Ok(self.inner.output_scale(ccy))
    }

    /// Override the output decimal scale for a currency.
    #[pyo3(text_signature = "(self, currency, decimals)")]
    fn set_output_scale(&mut self, currency: Bound<'_, PyAny>, decimals: u32) -> PyResult<()> {
        let ccy = extract_currency(currency.as_ref())?;
        self.inner
            .rounding
            .output_scale
            .overrides
            .insert(ccy, decimals);
        Ok(())
    }
}

/// Rounding mode enum exposed to Python.
#[pyclass(name = "RoundingMode", module = "finstack.config", frozen)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PyRoundingMode {
    pub(crate) inner: RoundingMode,
}

impl PyRoundingMode {
    pub(crate) const fn new(inner: RoundingMode) -> Self {
        Self { inner }
    }

    fn label(&self) -> &'static str {
        match self.inner {
            RoundingMode::Bankers => "bankers",
            RoundingMode::AwayFromZero => "away_from_zero",
            RoundingMode::TowardZero => "toward_zero",
            RoundingMode::Floor => "floor",
            RoundingMode::Ceil => "ceil",
            _ => "unknown",
        }
    }
}

#[pymethods]
impl PyRoundingMode {
    #[classattr]
    const BANKERS: Self = Self {
        inner: RoundingMode::Bankers,
    };
    #[classattr]
    const AWAY_FROM_ZERO: Self = Self {
        inner: RoundingMode::AwayFromZero,
    };
    #[classattr]
    const TOWARD_ZERO: Self = Self {
        inner: RoundingMode::TowardZero,
    };
    #[classattr]
    const FLOOR: Self = Self {
        inner: RoundingMode::Floor,
    };
    #[classattr]
    const CEIL: Self = Self {
        inner: RoundingMode::Ceil,
    };

    #[classmethod]
    #[pyo3(text_signature = "(cls, name)")]
    fn from_name(_cls: &Bound<'_, PyType>, name: &str) -> PyResult<Self> {
        parse_rounding_mode(name)
    }

    /// Snake-case name of the rounding mode.
    #[getter]
    fn name(&self) -> &'static str {
        self.label()
    }

    fn __repr__(&self) -> String {
        format!("RoundingMode('{}')", self.label())
    }

    fn __str__(&self) -> &'static str {
        self.label()
    }

    fn __hash__(&self) -> isize {
        self.inner as isize
    }

    fn __richcmp__(
        &self,
        other: Bound<'_, PyAny>,
        op: CompareOp,
        py: Python<'_>,
    ) -> PyResult<PyObject> {
        let rhs = match extract_rounding_mode(&other) {
            Ok(value) => Some(value),
            Err(_) => None,
        };

        let result = match op {
            CompareOp::Eq => rhs.map(|v| v == self.inner).unwrap_or(false),
            CompareOp::Ne => rhs.map(|v| v != self.inner).unwrap_or(true),
            _ => return Err(PyValueError::new_err("Unsupported comparison")),
        };

        let py_bool = result.into_bound_py_any(py)?;
        Ok(py_bool.into())
    }
}

impl fmt::Display for PyRoundingMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}

pub(crate) fn register<'py>(py: Python<'py>, parent: &Bound<'py, PyModule>) -> PyResult<()> {
    let module = PyModule::new(py, "config")?;
    module.setattr(
        "__doc__",
        "Configuration helpers for finstack rounding rules and currency scales.",
    )?;
    module.add_class::<PyFinstackConfig>()?;
    module.add_class::<PyRoundingMode>()?;
    let all = PyList::new(py, ["FinstackConfig", "RoundingMode"])?;
    module.setattr("__all__", all)?;
    parent.add_submodule(&module)?;
    Ok(())
}

pub(crate) fn extract_rounding_mode(value: &Bound<'_, PyAny>) -> PyResult<RoundingMode> {
    if let Ok(mode) = value.extract::<PyRef<PyRoundingMode>>() {
        return Ok(mode.inner);
    }

    if let Ok(name) = value.extract::<&str>() {
        return parse_rounding_mode(name).map(|wrapper| wrapper.inner);
    }

    Err(PyTypeError::new_err(
        "Expected RoundingMode or string identifier",
    ))
}

fn parse_rounding_mode(name: &str) -> PyResult<PyRoundingMode> {
    match name.to_ascii_lowercase().as_str() {
        "bankers" | "banker's" | "banker" => Ok(PyRoundingMode::new(RoundingMode::Bankers)),
        "away_from_zero" | "away-from-zero" | "awayfromzero" => {
            Ok(PyRoundingMode::new(RoundingMode::AwayFromZero))
        }
        "toward_zero" | "towards_zero" | "towardzero" => {
            Ok(PyRoundingMode::new(RoundingMode::TowardZero))
        }
        "floor" => Ok(PyRoundingMode::new(RoundingMode::Floor)),
        "ceil" | "ceiling" => Ok(PyRoundingMode::new(RoundingMode::Ceil)),
        other => Err(unknown_rounding_mode(other)),
    }
}
