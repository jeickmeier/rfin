//! Configuration bindings for rounding policies and currency scales.
//!
//! Provides a Python-facing `FinstackConfig` to manage global rounding behavior
//! and per-currency decimal scales for both ingestion and presentation. Use this
//! to control how `Money` values are parsed and formatted throughout analyses.
//! Also exposes a `RoundingMode` enum with common strategies (bankers, floor,
//! ceil, toward/away from zero).
// use crate::core::currency::extract_currency; // replaced by CurrencyArg
use crate::core::common::args::{CurrencyArg, RoundingModeArg};
use crate::core::common::{labels::normalize_label, pycmp::richcmp_eq_ne};
use crate::errors::unknown_rounding_mode;
use finstack_core::config::{FinstackConfig, RoundingMode};
use pyo3::basic::CompareOp;
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
// use pyo3::FromPyObject; // only needed at call sites using .extract()
use pyo3::types::{PyAnyMethods, PyList, PyModule, PyModuleMethods, PyType};
use pyo3::Bound;
use std::fmt;

/// Manage global rounding behaviour and currency decimal scales.
///
/// Parameters
/// ----------
/// None
///     Construct via :class:`FinstackConfig()` to use default rounding rules.
///
/// Returns
/// -------
/// FinstackConfig
///     Configuration handle that can be reused across money formatting operations.
#[pyclass(name = "FinstackConfig", module = "finstack.core.config")]
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
    ///
    /// Returns
    /// -------
    /// FinstackConfig
    ///     Configuration using finstack defaults (bankers rounding, ISO scales).
    fn ctor() -> Self {
        Self::new(FinstackConfig::default())
    }

    /// Clone this configuration.
    ///
    /// Returns
    /// -------
    /// FinstackConfig
    ///     A new configuration pointing to the same rounding overrides.
    #[pyo3(text_signature = "(self)")]
    fn copy(&self) -> Self {
        Self::new(self.inner.clone())
    }

    #[getter]
    /// Active rounding mode.
    ///
    /// Returns
    /// -------
    /// RoundingMode
    ///     Current rounding policy applied during formatting.
    fn rounding_mode(&self) -> PyRoundingMode {
        PyRoundingMode::new(self.inner.rounding.mode)
    }

    /// Update the rounding mode using a :class:`RoundingMode` or snake-case string.
    ///
    /// Parameters
    /// ----------
    /// mode : RoundingMode or str
    ///     Either an enum instance or a snake-case label such as ``"floor"``.
    ///
    /// Returns
    /// -------
    /// None
    ///
    /// Examples
    /// --------
    /// >>> cfg = FinstackConfig()
    /// >>> cfg.set_rounding_mode("away_from_zero")
    #[pyo3(text_signature = "(self, mode)")]
    fn set_rounding_mode(&mut self, mode: Bound<'_, PyAny>) -> PyResult<()> {
        let RoundingModeArg(value) = mode.extract()?;
        self.inner.rounding.mode = value;
        Ok(())
    }

    #[pyo3(text_signature = "(self, currency)")]
    /// Decimal places accepted when ingesting source data for ``currency``.
    ///
    /// Parameters
    /// ----------
    /// currency : Currency or str
    ///     Currency to inspect (ISO code or :class:`Currency`).
    ///
    /// Returns
    /// -------
    /// int
    ///     Number of decimal places allowed for ingestion.
    fn ingest_scale(&self, currency: Bound<'_, PyAny>) -> PyResult<u32> {
        let CurrencyArg(ccy) = currency.extract()?;
        Ok(self.inner.ingest_scale(ccy))
    }

    /// Override the ingest decimal scale for a currency.
    ///
    /// Parameters
    /// ----------
    /// currency : Currency or str
    ///     Currency whose ingest precision should change.
    /// decimals : int
    ///     Number of decimal places to accept (e.g. ``6`` for JPY pipettes).
    ///
    /// Returns
    /// -------
    /// None
    #[pyo3(text_signature = "(self, currency, decimals)")]
    fn set_ingest_scale(&mut self, currency: Bound<'_, PyAny>, decimals: u32) -> PyResult<()> {
        let CurrencyArg(ccy) = currency.extract()?;
        self.inner
            .rounding
            .ingest_scale
            .overrides
            .insert(ccy, decimals);
        Ok(())
    }

    #[pyo3(text_signature = "(self, currency)")]
    /// Decimal places used when exporting values for ``currency``.
    ///
    /// Parameters
    /// ----------
    /// currency : Currency or str
    ///     Currency to inspect.
    ///
    /// Returns
    /// -------
    /// int
    ///     Decimal places used during formatting.
    fn output_scale(&self, currency: Bound<'_, PyAny>) -> PyResult<u32> {
        let CurrencyArg(ccy) = currency.extract()?;
        Ok(self.inner.output_scale(ccy))
    }

    /// Override the output decimal scale for a currency.
    ///
    /// Parameters
    /// ----------
    /// currency : Currency or str
    ///     Currency whose presentation precision should change.
    /// decimals : int
    ///     Decimal places to use when formatting.
    ///
    /// Returns
    /// -------
    /// None
    ///
    /// Examples
    /// --------
    /// >>> cfg = FinstackConfig()
    /// >>> cfg.set_output_scale("JPY", 0)
    #[pyo3(text_signature = "(self, currency, decimals)")]
    fn set_output_scale(&mut self, currency: Bound<'_, PyAny>, decimals: u32) -> PyResult<()> {
        let CurrencyArg(ccy) = currency.extract()?;
        self.inner
            .rounding
            .output_scale
            .overrides
            .insert(ccy, decimals);
        Ok(())
    }
}

/// Enumerate supported rounding policies for monetary values.
///
/// Parameters
/// ----------
/// None
///     Access constants such as :attr:`RoundingMode.BANKERS` instead of instantiating directly.
///
/// Returns
/// -------
/// RoundingMode
///     Enum value describing the rounding strategy.
#[pyclass(name = "RoundingMode", module = "finstack.core.config", frozen)]
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
    /// Parse a rounding mode from a snake-case string.
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
        richcmp_eq_ne(py, &self.inner, rhs, op)
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

/// Parse a snake-case rounding mode label into a `PyRoundingMode`.
///
/// Parameters
/// ----------
/// name : &str
///     Case-insensitive snake-case identifier (e.g. "bankers", "floor").
///
/// Returns
/// -------
/// PyRoundingMode
///     Wrapper around the core rounding mode.
///
/// Raises
/// ------
/// ValueError
///     If the name is not a recognized rounding mode.
fn parse_rounding_mode(name: &str) -> PyResult<PyRoundingMode> {
    let n = normalize_label(name);
    match n.as_str() {
        "bankers" | "banker" => Ok(PyRoundingMode::new(RoundingMode::Bankers)),
        "away_from_zero" | "awayfromzero" => Ok(PyRoundingMode::new(RoundingMode::AwayFromZero)),
        "toward_zero" | "towards_zero" => Ok(PyRoundingMode::new(RoundingMode::TowardZero)),
        "floor" => Ok(PyRoundingMode::new(RoundingMode::Floor)),
        "ceil" | "ceiling" => Ok(PyRoundingMode::new(RoundingMode::Ceil)),
        other => Err(unknown_rounding_mode(other)),
    }
}
