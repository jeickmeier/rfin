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
use crate::errors::{unknown_currency, unknown_rounding_mode, PyContext};
use finstack_core::config::{
    results_meta, rounding_context_from, CurrencyScalePolicy, FinstackConfig, NumericMode,
    ResultsMeta, RoundingContext, RoundingMode, RoundingPolicy, ZeroKind, NUMERIC_MODE,
};
use pyo3::basic::CompareOp;
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
// use pyo3::FromPyObject; // only needed at call sites using .extract()
use pyo3::types::{PyAnyMethods, PyList, PyModule, PyModuleMethods, PyType};
use pyo3::Bound;
use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

#[pyclass(name = "CurrencyScalePolicy", module = "finstack.core.config", frozen)]
#[derive(Clone, Debug)]
pub struct PyCurrencyScalePolicy {
    pub(crate) inner: CurrencyScalePolicy,
}

impl PyCurrencyScalePolicy {
    pub(crate) fn new(inner: CurrencyScalePolicy) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyCurrencyScalePolicy {
    #[new]
    #[pyo3(signature = (overrides=None))]
    #[pyo3(text_signature = "(overrides=None)")]
    fn ctor(overrides: Option<HashMap<String, u32>>) -> PyResult<Self> {
        let mut policy = CurrencyScalePolicy::default();
        if let Some(map) = overrides {
            for (code, scale) in map {
                let ccy = finstack_core::currency::Currency::from_str(&code)
                    .map_err(|_| unknown_currency(&code))?;
                policy.overrides.insert(ccy, scale);
            }
        }
        Ok(Self::new(policy))
    }

    #[getter]
    fn overrides(&self) -> HashMap<String, u32> {
        self.inner
            .overrides
            .iter()
            .map(|(ccy, scale)| (ccy.to_string(), *scale))
            .collect()
    }
}

fn extract_scale_policy(value: &Bound<'_, PyAny>) -> PyResult<CurrencyScalePolicy> {
    if let Ok(policy) = value.extract::<PyRef<PyCurrencyScalePolicy>>() {
        return Ok(policy.inner.clone());
    }
    if let Ok(map) = value.extract::<HashMap<String, u32>>() {
        let mut policy = CurrencyScalePolicy::default();
        for (code, scale) in map {
            let ccy = finstack_core::currency::Currency::from_str(&code)
                .map_err(|_| unknown_currency(&code))?;
            policy.overrides.insert(ccy, scale);
        }
        return Ok(policy);
    }
    Err(PyTypeError::new_err(
        "Expected CurrencyScalePolicy or mapping[str, int]",
    ))
}

#[pyclass(name = "RoundingPolicy", module = "finstack.core.config", frozen)]
#[derive(Clone, Debug)]
pub struct PyRoundingPolicy {
    pub(crate) inner: RoundingPolicy,
}

impl PyRoundingPolicy {
    pub(crate) fn new(inner: RoundingPolicy) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyRoundingPolicy {
    #[new]
    #[pyo3(signature = (*, mode=None, ingest_scale=None, output_scale=None))]
    #[pyo3(text_signature = "(*, mode=None, ingest_scale=None, output_scale=None)")]
    fn ctor(
        mode: Option<Bound<'_, PyAny>>,
        ingest_scale: Option<Bound<'_, PyAny>>,
        output_scale: Option<Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        let mut policy = RoundingPolicy::default();
        if let Some(mode_any) = mode {
            let RoundingModeArg(m) = mode_any.extract().context("mode")?;
            policy.mode = m;
        }
        if let Some(val) = ingest_scale {
            policy.ingest_scale = extract_scale_policy(&val).context("ingest_scale")?;
        }
        if let Some(val) = output_scale {
            policy.output_scale = extract_scale_policy(&val).context("output_scale")?;
        }
        Ok(Self::new(policy))
    }

    #[getter]
    fn mode(&self) -> PyRoundingMode {
        PyRoundingMode::new(self.inner.mode)
    }

    #[getter]
    fn ingest_scale(&self) -> PyCurrencyScalePolicy {
        PyCurrencyScalePolicy::new(self.inner.ingest_scale.clone())
    }

    #[getter]
    fn output_scale(&self) -> PyCurrencyScalePolicy {
        PyCurrencyScalePolicy::new(self.inner.output_scale.clone())
    }
}

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
        let RoundingModeArg(value) = mode.extract().context("mode")?;
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
        let CurrencyArg(ccy) = currency.extract().context("currency")?;
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
        let CurrencyArg(ccy) = currency.extract().context("currency")?;
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
        let CurrencyArg(ccy) = currency.extract().context("currency")?;
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
        let CurrencyArg(ccy) = currency.extract().context("currency")?;
        self.inner
            .rounding
            .output_scale
            .overrides
            .insert(ccy, decimals);
        Ok(())
    }

    /// Inspect the full rounding policy (mode plus ingest/output overrides).
    #[getter]
    fn rounding_policy(&self) -> PyRoundingPolicy {
        PyRoundingPolicy::new(self.inner.rounding.clone())
    }

    /// Build an immutable rounding context snapshot from this configuration.
    #[pyo3(text_signature = "(self)")]
    fn rounding_context(&self) -> PyRoundingContext {
        PyRoundingContext::new(rounding_context_from(&self.inner))
    }

    /// Build a results metadata snapshot from this configuration.
    #[pyo3(text_signature = "(self)")]
    fn results_meta(&self) -> PyResultsMeta {
        PyResultsMeta::new(results_meta(&self.inner))
    }

    /// Numeric mode compiled into the core crate.
    #[getter]
    fn numeric_mode(&self) -> PyNumericMode {
        PyNumericMode::new(NUMERIC_MODE)
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

#[pyclass(name = "ZeroKind", module = "finstack.core.config", frozen)]
#[derive(Clone, Debug)]
pub struct PyZeroKind {
    pub(crate) inner: ZeroKind,
}

impl PyZeroKind {
    pub(crate) const fn new(inner: ZeroKind) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyZeroKind {
    #[classattr]
    const GENERIC: Self = Self {
        inner: ZeroKind::Generic,
    };

    #[classattr]
    const RATE: Self = Self {
        inner: ZeroKind::Rate,
    };

    #[classmethod]
    #[pyo3(text_signature = "(cls, currency)")]
    /// Construct a money zero-kind for the specified currency.
    fn money(_cls: &Bound<'_, PyType>, currency: Bound<'_, PyAny>) -> PyResult<Self> {
        let CurrencyArg(ccy) = currency.extract().context("currency")?;
        Ok(Self::new(ZeroKind::Money(ccy)))
    }

    #[getter]
    fn kind(&self) -> &'static str {
        match self.inner {
            ZeroKind::Money(_) => "money",
            ZeroKind::Rate => "rate",
            ZeroKind::Generic => "generic",
            _ => "generic",
        }
    }

    fn __repr__(&self) -> String {
        match self.inner {
            ZeroKind::Money(ccy) => format!("ZeroKind.money('{ccy}')"),
            ZeroKind::Rate => "ZeroKind.RATE".to_string(),
            ZeroKind::Generic => "ZeroKind.GENERIC".to_string(),
            _ => "ZeroKind.GENERIC".to_string(),
        }
    }
}

fn extract_zero_kind(value: &Bound<'_, PyAny>) -> PyResult<ZeroKind> {
    if let Ok(kind) = value.extract::<PyRef<PyZeroKind>>() {
        return Ok(kind.inner);
    }
    Err(PyTypeError::new_err(
        "Expected ZeroKind (use ZeroKind.money/ZeroKind.RATE/ZeroKind.GENERIC)",
    ))
}

#[pyclass(name = "RoundingContext", module = "finstack.core.config", frozen)]
#[derive(Clone, Debug)]
pub struct PyRoundingContext {
    pub(crate) inner: RoundingContext,
}

impl PyRoundingContext {
    pub(crate) fn new(inner: RoundingContext) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyRoundingContext {
    #[getter]
    fn mode(&self) -> PyRoundingMode {
        PyRoundingMode::new(self.inner.mode)
    }

    #[getter]
    fn version(&self) -> u32 {
        self.inner.version
    }

    #[getter]
    fn ingest_scale_by_currency(&self) -> HashMap<String, u32> {
        self.inner
            .ingest_scale_by_ccy
            .iter()
            .map(|(ccy, scale)| (ccy.to_string(), *scale))
            .collect()
    }

    #[getter]
    fn output_scale_by_currency(&self) -> HashMap<String, u32> {
        self.inner
            .output_scale_by_ccy
            .iter()
            .map(|(ccy, scale)| (ccy.to_string(), *scale))
            .collect()
    }

    /// Effective output scale for a currency.
    #[pyo3(text_signature = "(self, currency)")]
    fn output_scale(&self, currency: Bound<'_, PyAny>) -> PyResult<u32> {
        let CurrencyArg(ccy) = currency.extract().context("currency")?;
        Ok(self.inner.output_scale(ccy))
    }

    /// Money epsilon derived from the currency output scale.
    #[pyo3(text_signature = "(self, currency)")]
    fn money_epsilon(&self, currency: Bound<'_, PyAny>) -> PyResult<f64> {
        let CurrencyArg(ccy) = currency.extract().context("currency")?;
        Ok(self.inner.money_epsilon(ccy))
    }

    /// True if a money amount is effectively zero under this context.
    #[pyo3(text_signature = "(self, amount, currency)")]
    fn is_effectively_zero_money(&self, amount: f64, currency: Bound<'_, PyAny>) -> PyResult<bool> {
        let CurrencyArg(ccy) = currency.extract().context("currency")?;
        Ok(self.inner.is_effectively_zero_money(amount, ccy))
    }

    /// True if a floating value is effectively zero for the specified kind.
    #[pyo3(text_signature = "(self, value, kind)")]
    fn is_effectively_zero(&self, value: f64, kind: Bound<'_, PyAny>) -> PyResult<bool> {
        let zk = extract_zero_kind(&kind)?;
        Ok(self.inner.is_effectively_zero(value, zk))
    }
}

#[pyclass(name = "NumericMode", module = "finstack.core.config", frozen)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PyNumericMode {
    pub(crate) inner: NumericMode,
}

impl PyNumericMode {
    pub(crate) const fn new(inner: NumericMode) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyNumericMode {
    #[classattr]
    const F64: Self = Self {
        inner: NumericMode::F64,
    };

    fn __repr__(&self) -> &'static str {
        "NumericMode.F64"
    }
}

#[pyclass(name = "ResultsMeta", module = "finstack.core.config", frozen)]
#[derive(Clone, Debug)]
pub struct PyResultsMeta {
    pub(crate) inner: ResultsMeta,
}

impl PyResultsMeta {
    pub(crate) fn new(inner: ResultsMeta) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyResultsMeta {
    #[getter]
    fn numeric_mode(&self) -> PyNumericMode {
        PyNumericMode::new(self.inner.numeric_mode)
    }

    #[getter]
    fn rounding(&self) -> PyRoundingContext {
        PyRoundingContext::new(self.inner.rounding.clone())
    }

    #[getter]
    fn fx_policy_applied(&self) -> Option<String> {
        self.inner.fx_policy_applied.clone()
    }

    #[getter]
    fn timestamp(&self) -> Option<String> {
        self.inner.timestamp.map(|t| t.to_string())
    }

    #[getter]
    fn version(&self) -> Option<String> {
        self.inner.version.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "ResultsMeta(numeric_mode={:?}, version={:?})",
            self.inner.numeric_mode, self.inner.version
        )
    }
}

pub(crate) fn register<'py>(py: Python<'py>, parent: &Bound<'py, PyModule>) -> PyResult<()> {
    let module = PyModule::new(py, "config")?;
    module.setattr(
        "__doc__",
        "Configuration helpers for finstack rounding rules and currency scales.",
    )?;
    module.add_class::<PyCurrencyScalePolicy>()?;
    module.add_class::<PyRoundingPolicy>()?;
    module.add_class::<PyFinstackConfig>()?;
    module.add_class::<PyRoundingMode>()?;
    module.add_class::<PyZeroKind>()?;
    module.add_class::<PyRoundingContext>()?;
    module.add_class::<PyNumericMode>()?;
    module.add_class::<PyResultsMeta>()?;
    module.add_function(wrap_pyfunction!(py_rounding_context_from, &module)?)?;
    module.add_function(wrap_pyfunction!(py_results_meta, &module)?)?;
    module.setattr("NUMERIC_MODE", PyNumericMode::new(NUMERIC_MODE))?;
    let all = PyList::new(
        py,
        [
            "CurrencyScalePolicy",
            "RoundingPolicy",
            "FinstackConfig",
            "RoundingMode",
            "ZeroKind",
            "RoundingContext",
            "NumericMode",
            "NUMERIC_MODE",
            "ResultsMeta",
            "rounding_context_from",
            "results_meta",
        ],
    )?;
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

/// Build a rounding context snapshot from a configuration.
#[pyfunction(name = "rounding_context_from")]
#[pyo3(text_signature = "(config)")]
fn py_rounding_context_from(cfg: PyRef<PyFinstackConfig>) -> PyRoundingContext {
    PyRoundingContext::new(rounding_context_from(&cfg.inner))
}

/// Build a results metadata snapshot from a configuration.
#[pyfunction(name = "results_meta")]
#[pyo3(text_signature = "(config)")]
fn py_results_meta(cfg: PyRef<PyFinstackConfig>) -> PyResultsMeta {
    PyResultsMeta::new(results_meta(&cfg.inner))
}
