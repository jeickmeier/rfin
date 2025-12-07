//! Money bindings: currency-tagged amounts with safe arithmetic.
//!
//! This module exposes the Rust `Money` type to Python with currency safety:
//! arithmetic requires matching currencies and raises `ValueError` otherwise.
//! Formatting respects ISO minor units by default and can be customized via
//! `FinstackConfig` ingest/output scales. Conversions to/from tuples are
//! supported for ergonomics and interoperability with Python code.
use crate::core::config::PyFinstackConfig;
use crate::core::currency::{extract_currency, PyCurrency};
use crate::core::dates::utils::py_to_date;
use crate::core::market_data::fx::{parse_policy, PyFxMatrix};
use crate::errors::{core_to_py, PyContext};
use finstack_core::money::Money;
use pyo3::basic::CompareOp;
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyAnyMethods, PyList, PyModule, PyModuleMethods, PyType};
use pyo3::{Bound, IntoPyObjectExt};
use std::fmt;

/// Represent a currency-tagged monetary amount with safe arithmetic semantics.
///
/// Parameters
/// ----------
/// amount : float
///     Scalar value expressed in minor units defined by ``currency``.
/// currency : str or Currency
///     ISO code or :class:`Currency` instance describing the legal tender.
///
/// Returns
/// -------
/// Money
///     Money wrapper supporting arithmetic, formatting, and tuple conversions.
#[pyclass(name = "Money", module = "finstack.core.money")]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PyMoney {
    pub(crate) inner: Money,
}

impl PyMoney {
    pub(crate) fn new(inner: Money) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyMoney {
    #[new]
    #[pyo3(text_signature = "(amount, currency)")]
    /// Create a money amount with the provided ISO currency.
    ///
    /// Parameters
    /// ----------
    /// amount : float
    ///     Numeric value expressed in the currency's minor units.
    /// currency : Currency or str
    ///     ISO code or :class:`Currency` instance.
    ///
    /// Returns
    /// -------
    /// Money
    ///     Money instance representing ``amount`` in ``currency``.
    ///
    /// Examples
    /// --------
    /// >>> Money(125.5, "USD")
    fn ctor(amount: f64, currency: Bound<'_, PyAny>) -> PyResult<Self> {
        let ccy = extract_currency(&currency).context("currency")?;
        Ok(Self::new(Money::new(amount, ccy)))
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, amount, currency, config)")]
    /// Construct a money value using a configuration for ingest rounding.
    ///
    /// Parameters
    /// ----------
    /// amount : float
    ///     Raw monetary value.
    /// currency : Currency or str
    ///     Currency identifier.
    /// config : FinstackConfig
    ///     Configuration controlling ingest rounding/scale.
    ///
    /// Returns
    /// -------
    /// Money
    ///     Money instance respecting custom ingest rules.
    ///
    /// Examples
    /// --------
    /// >>> cfg = FinstackConfig()
    /// >>> cfg.set_ingest_scale("JPY", 4)
    /// >>> Money.from_config(123.4567, "JPY", cfg)
    fn from_config(
        _cls: &Bound<'_, PyType>,
        amount: f64,
        currency: Bound<'_, PyAny>,
        config: &PyFinstackConfig,
    ) -> PyResult<Self> {
        let ccy = extract_currency(&currency).context("currency")?;
        Ok(Self::new(Money::new_with_config(
            amount,
            ccy,
            &config.inner,
        )))
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, currency)")]
    /// Zero amount in the requested currency.
    ///
    /// Parameters
    /// ----------
    /// currency : Currency or str
    ///     Currency identifier.
    ///
    /// Returns
    /// -------
    /// Money
    ///     Money instance with amount ``0`` in ``currency``.
    fn zero(_cls: &Bound<'_, PyType>, currency: Bound<'_, PyAny>) -> PyResult<Self> {
        let ccy = extract_currency(&currency).context("currency")?;
        Ok(Self::new(Money::new(0.0, ccy)))
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, value)")]
    /// Construct from an ``(amount, currency)`` tuple or another :class:`Money` instance.
    ///
    /// Parameters
    /// ----------
    /// value : tuple[float, Currency] or Money
    ///     Source value to coerce into :class:`Money`.
    ///
    /// Returns
    /// -------
    /// Money
    ///     Money instance matching the input.
    fn from_tuple(_cls: &Bound<'_, PyType>, value: Bound<'_, PyAny>) -> PyResult<Self> {
        let inner = extract_money(&value).context("value")?;
        Ok(Self::new(inner))
    }

    #[getter]
    /// Amount as floating-point value.
    ///
    /// Returns
    /// -------
    /// float
    ///     Monetary amount in native units.
    fn amount(&self) -> f64 {
        self.inner.amount()
    }

    #[getter]
    /// Currency of this amount.
    ///
    /// Returns
    /// -------
    /// Currency
    ///     Currency associated with the amount.
    fn currency(&self) -> PyCurrency {
        PyCurrency::new(self.inner.currency())
    }

    #[pyo3(text_signature = "(self)")]
    /// Return ``(amount, currency)`` tuple.
    ///
    /// Returns
    /// -------
    /// tuple[float, Currency]
    ///     Tuple containing the numeric amount and currency object.
    fn to_tuple(&self) -> PyResult<(f64, PyCurrency)> {
        Ok((self.inner.amount(), PyCurrency::new(self.inner.currency())))
    }

    #[pyo3(text_signature = "(self)")]
    /// Format using ISO minor units (e.g., ``"USD 10.00"``).
    ///
    /// Returns
    /// -------
    /// str
    ///     Formatted string with ISO code prefix.
    fn format(&self) -> String {
        format!("{}", self.inner)
    }

    #[pyo3(text_signature = "(self, config)")]
    /// Format using a configuration (custom rounding/scales).
    ///
    /// Parameters
    /// ----------
    /// config : FinstackConfig
    ///     Configuration detailing rounding and output scales.
    ///
    /// Returns
    /// -------
    /// str
    ///     Formatted money string respecting ``config`` overrides.
    fn format_with_config(&self, config: &PyFinstackConfig) -> String {
        self.inner.format_with_config(&config.inner)
    }

    #[pyo3(text_signature = "(self, decimals, show_currency=True)")]
    /// Format with explicit decimal precision and optional currency display.
    ///
    /// Parameters
    /// ----------
    /// decimals : int
    ///     Number of decimal places to render.
    /// show_currency : bool, optional
    ///     Include the currency code when ``True`` (default).
    fn format_custom(&self, decimals: usize, show_currency: bool) -> String {
        self.inner.format(decimals, show_currency)
    }

    #[pyo3(text_signature = "(self, decimals)")]
    /// Format with thousands separators and explicit decimal places.
    ///
    /// Parameters
    /// ----------
    /// decimals : int
    ///     Number of decimal places to render.
    fn format_with_separators(&self, decimals: usize) -> String {
        self.inner.format_with_separators(decimals)
    }

    #[pyo3(
        signature = (to_currency, on, fx_matrix, policy=None),
        text_signature = "(self, to_currency, on, fx_matrix, policy=None)"
    )]
    /// Convert this amount into another currency using the provided FX matrix.
    ///
    /// Parameters
    /// ----------
    /// to_currency : Currency or str
    ///     Target currency.
    /// on : datetime.date
    ///     Valuation date for the conversion.
    /// fx_matrix : FxMatrix
    ///     FX source used for rate discovery.
    /// policy : FxConversionPolicy or str, optional
    ///     Conversion policy hint (defaults to cashflow date).
    fn convert(
        &self,
        py: Python<'_>,
        to_currency: Bound<'_, PyAny>,
        on: Bound<'_, PyAny>,
        fx_matrix: &PyFxMatrix,
        policy: Option<Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        let target = extract_currency(&to_currency).context("to_currency")?;
        let date = py_to_date(&on).context("on")?;
        let fx_policy = parse_policy(py, policy)?;
        // Manual conversion since Arc<dyn FxProvider> can't be passed to convert directly
        if self.inner.currency() == target {
            return Ok(self.clone());
        }
        let provider = fx_matrix.inner.provider();
        let rate = provider
            .rate(self.inner.currency(), target, date, fx_policy)
            .map_err(core_to_py)?;
        if !rate.is_finite() {
            return Err(core_to_py(finstack_core::error::InputError::Invalid.into()));
        }
        let converted =
            finstack_core::money::Money::new((self.inner.amount() * rate).into(), target);
        Ok(Self::new(converted))
    }

    #[pyo3(text_signature = "(self, other)")]
    /// Checked addition.
    ///
    /// Parameters
    /// ----------
    /// other : Money or tuple
    ///     Right-hand operand to add.
    ///
    /// Returns
    /// -------
    /// Money
    ///     Sum of both amounts.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If currencies differ.
    fn checked_add(&self, other: Bound<'_, PyAny>) -> PyResult<Self> {
        let rhs = extract_money(&other).context("other")?;
        (self.inner + rhs).map(Self::new).map_err(core_to_py)
    }

    #[pyo3(text_signature = "(self, other)")]
    /// Checked subtraction.
    ///
    /// Parameters
    /// ----------
    /// other : Money or tuple
    ///     Right-hand operand to subtract.
    ///
    /// Returns
    /// -------
    /// Money
    ///     Difference ``self - other``.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If currencies differ.
    fn checked_sub(&self, other: Bound<'_, PyAny>) -> PyResult<Self> {
        let rhs = extract_money(&other).context("other")?;
        (self.inner - rhs).map(Self::new).map_err(core_to_py)
    }

    fn __repr__(&self) -> String {
        format!(
            "Money(amount={amount:.6}, currency='{code}')",
            amount = self.inner.amount(),
            code = self.inner.currency()
        )
    }

    fn __str__(&self) -> String {
        self.format()
    }

    fn __hash__(&self) -> isize {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.inner.currency().hash(&mut hasher);
        let bits = self.inner.amount().to_bits();
        bits.hash(&mut hasher);
        (hasher.finish() & isize::MAX as u64) as isize
    }

    fn __richcmp__(
        &self,
        other: Bound<'_, PyAny>,
        op: CompareOp,
        py: Python<'_>,
    ) -> PyResult<Py<PyAny>> {
        let rhs = match extract_money(&other) {
            Ok(value) => Some(value),
            Err(_) => None,
        };

        let result = match op {
            CompareOp::Eq => rhs
                .map(|v| v.amount() == self.inner.amount() && v.currency() == self.inner.currency())
                .unwrap_or(false),
            CompareOp::Ne => rhs
                .map(|v| v.amount() != self.inner.amount() || v.currency() != self.inner.currency())
                .unwrap_or(true),
            _ => return Err(PyValueError::new_err("Unsupported comparison")),
        };
        let py_bool = result.into_bound_py_any(py)?;
        Ok(py_bool.into())
    }

    fn __add__(&self, other: Bound<'_, PyAny>) -> PyResult<Self> {
        let rhs = extract_money(&other).context("other")?;
        (self.inner + rhs).map(Self::new).map_err(core_to_py)
    }

    /// Reflected addition: ``other + self``.
    fn __radd__(&self, other: Bound<'_, PyAny>) -> PyResult<Self> {
        self.__add__(other)
    }

    /// Subtract another money amount, enforcing matching currencies.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If currencies differ.
    fn __sub__(&self, other: Bound<'_, PyAny>) -> PyResult<Self> {
        let rhs = extract_money(&other).context("other")?;
        (self.inner - rhs).map(Self::new).map_err(core_to_py)
    }

    /// Reflected subtraction: ``other - self``.
    fn __rsub__(&self, other: Bound<'_, PyAny>) -> PyResult<Self> {
        let rhs = extract_money(&other).context("other")?;
        (rhs - self.inner).map(Self::new).map_err(core_to_py)
    }

    /// Scale this amount by a floating-point factor.
    fn __mul__(&self, factor: f64) -> Self {
        Self::new(self.inner * factor)
    }

    /// Support scalar * money multiplication.
    fn __rmul__(&self, factor: f64) -> Self {
        Self::new(self.inner * factor)
    }

    /// Divide by a scalar, raising on zero divisors.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If ``divisor`` is zero.
    fn __truediv__(&self, divisor: f64) -> PyResult<Self> {
        if divisor == 0.0 {
            return Err(PyValueError::new_err("Cannot divide by zero"));
        }
        Ok(Self::new(self.inner / divisor))
    }

    /// Prevent scalar / money operations which have no meaning.
    ///
    /// Raises
    /// ------
    /// TypeError
    ///     Always raised; scalar divided by money is undefined.
    fn __rtruediv__(&self, value: f64) -> PyResult<()> {
        let _ = value;
        Err(PyTypeError::new_err(
            "Division of scalar by Money is undefined",
        ))
    }

    /// Deconstruct into ``(amount, currency)`` tuple (used by pickle).
    fn __getnewargs__(&self) -> PyResult<(f64, PyCurrency)> {
        Ok((self.inner.amount(), PyCurrency::new(self.inner.currency())))
    }
}

impl fmt::Display for PyMoney {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.inner)
    }
}

pub(crate) fn register<'py>(py: Python<'py>, parent: &Bound<'py, PyModule>) -> PyResult<()> {
    let module = PyModule::new(py, "money")?;
    module.setattr(
        "__doc__",
        "Currency-tagged money amounts with safe arithmetic and configurable formatting.",
    )?;
    module.add_class::<PyMoney>()?;
    let all = PyList::new(py, ["Money"])?;
    module.setattr("__all__", all)?;
    parent.add_submodule(&module)?;
    Ok(())
}

pub(crate) fn extract_money(value: &Bound<'_, PyAny>) -> PyResult<Money> {
    if let Ok(mny) = value.extract::<PyRef<PyMoney>>() {
        return Ok(mny.inner);
    }

    if let Ok(tuple) = value.downcast::<pyo3::types::PyTuple>() {
        if tuple.len() == 2 {
            let amount = tuple.get_item(0)?.extract::<f64>()?;
            let currency_obj = tuple.get_item(1)?;
            let currency = extract_currency(&currency_obj)?;
            return Ok(Money::new(amount, currency));
        }
    }

    Err(PyTypeError::new_err(
        "Expected Money instance or (amount, currency) tuple",
    ))
}
