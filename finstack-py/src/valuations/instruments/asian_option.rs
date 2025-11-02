use crate::core::money::{extract_money, PyMoney};
use crate::core::utils::{date_to_py, py_to_date};
use crate::valuations::common::{extract_curve_id, extract_instrument_id};
use finstack_valuations::instruments::asian_option::{AsianOption, AveragingMethod};
use finstack_valuations::instruments::OptionType;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyList, PyModule, PyType};
use pyo3::Bound;

/// Averaging method for Asian options.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "AveragingMethod",
    frozen
)]
#[derive(Clone, Copy, Debug)]
pub struct PyAveragingMethod {
    pub(crate) inner: AveragingMethod,
}

impl PyAveragingMethod {
    pub(crate) const fn new(inner: AveragingMethod) -> Self {
        Self { inner }
    }

    fn label(&self) -> &'static str {
        match self.inner {
            AveragingMethod::Arithmetic => "arithmetic",
            AveragingMethod::Geometric => "geometric",
        }
    }
}

#[pymethods]
impl PyAveragingMethod {
    #[classattr]
    const ARITHMETIC: Self = Self::new(AveragingMethod::Arithmetic);
    #[classattr]
    const GEOMETRIC: Self = Self::new(AveragingMethod::Geometric);

    #[classmethod]
    #[pyo3(text_signature = "(cls, name)")]
    /// Parse an averaging method from a string label.
    ///
    /// Args:
    ///     name: One of ``"arithmetic"`` or ``"geometric"``.
    ///
    /// Returns:
    ///     AveragingMethod: Enum value corresponding to ``name``.
    fn from_name(_cls: &Bound<'_, PyType>, name: &str) -> PyResult<Self> {
        match name.to_lowercase().as_str() {
            "arithmetic" => Ok(Self::new(AveragingMethod::Arithmetic)),
            "geometric" => Ok(Self::new(AveragingMethod::Geometric)),
            other => Err(PyValueError::new_err(format!(
                "Unknown averaging method: {other}"
            ))),
        }
    }

    #[getter]
    /// Snake-case label for this averaging method.
    fn name(&self) -> &'static str {
        self.label()
    }

    fn __repr__(&self) -> String {
        format!("AveragingMethod('{}')", self.label())
    }

    fn __str__(&self) -> &'static str {
        self.label()
    }
}

/// Asian option instrument with arithmetic or geometric averaging.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "AsianOption",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyAsianOption {
    pub(crate) inner: AsianOption,
}

impl PyAsianOption {
    pub(crate) fn new(inner: AsianOption) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyAsianOption {
    #[classmethod]
    #[pyo3(
        text_signature = "(cls, instrument_id, ticker, strike, expiry, fixing_dates, notional, discount_curve, spot_id, vol_surface, *, averaging_method='arithmetic', option_type='call', dividend_yield_id=None)"
    )]
    #[allow(clippy::too_many_arguments)]
    /// Create an Asian option with explicit parameters.
    ///
    /// Args:
    ///     instrument_id: Instrument identifier or string-like object.
    ///     ticker: Equity ticker symbol for the underlying asset.
    ///     strike: Strike price expressed in quote currency units.
    ///     expiry: Option expiry date.
    ///     fixing_dates: List of fixing dates for averaging.
    ///     notional: Contract notional as :class:`finstack.core.money.Money`.
    ///     discount_curve: Discount curve identifier.
    ///     spot_id: Spot price identifier.
    ///     vol_surface: Volatility surface identifier.
    ///     averaging_method: Averaging method (``"arithmetic"`` or ``"geometric"``).
    ///     option_type: Option type (``"call"`` or ``"put"``).
    ///     dividend_yield_id: Optional dividend yield identifier.
    ///
    /// Returns:
    ///     AsianOption: Configured Asian option instrument.
    ///
    /// Raises:
    ///     ValueError: If identifiers or dates cannot be parsed.
    fn builder(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        ticker: &str,
        strike: f64,
        expiry: Bound<'_, PyAny>,
        fixing_dates: Bound<'_, PyList>,
        notional: Bound<'_, PyAny>,
        discount_curve: Bound<'_, PyAny>,
        spot_id: &str,
        vol_surface: Bound<'_, PyAny>,
        averaging_method: Option<&str>,
        option_type: Option<&str>,
        dividend_yield_id: Option<&str>,
    ) -> PyResult<Self> {
        use crate::core::common::labels::normalize_label;
        use finstack_core::dates::DayCount;

        let id = extract_instrument_id(&instrument_id)?;
        let expiry_date = py_to_date(&expiry)?;
        let notional_money = extract_money(&notional)?;
        let disc_id = extract_curve_id(&discount_curve)?;
        let vol_id = extract_curve_id(&vol_surface)?;

        // Parse fixing dates
        let mut fixing_dates_vec = Vec::new();
        for item in fixing_dates.iter() {
            fixing_dates_vec.push(py_to_date(&item)?);
        }

        // Parse averaging method
        let avg_method = match averaging_method.map(normalize_label).as_deref() {
            None | Some("arithmetic") => AveragingMethod::Arithmetic,
            Some("geometric") => AveragingMethod::Geometric,
            Some(other) => {
                return Err(PyValueError::new_err(format!(
                    "Unknown averaging method: {other}"
                )))
            }
        };

        // Parse option type
        let opt_type = match option_type.map(normalize_label).as_deref() {
            None | Some("call") => OptionType::Call,
            Some("put") => OptionType::Put,
            Some(other) => {
                return Err(PyValueError::new_err(format!(
                    "Unknown option type: {other}"
                )))
            }
        };

        let strike_money = finstack_core::money::Money::new(strike, notional_money.currency());

        let mut builder = AsianOption::builder();
        builder = builder.id(id);
        builder = builder.underlying_ticker(ticker.to_string());
        builder = builder.strike(strike_money);
        builder = builder.option_type(opt_type);
        builder = builder.averaging_method(avg_method);
        builder = builder.expiry(expiry_date);
        builder = builder.fixing_dates(fixing_dates_vec);
        builder = builder.notional(notional_money);
        builder = builder.day_count(DayCount::Act365F);
        builder = builder
            .pricing_overrides(finstack_valuations::instruments::PricingOverrides::default());
        builder = builder.disc_id(disc_id);
        builder = builder.spot_id(spot_id.to_string());
        builder = builder.vol_id(vol_id);
        if let Some(div) = dividend_yield_id {
            builder = builder.div_yield_id(div.to_string());
        }
        let option = builder.build().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to build AsianOption: {e}"))
        })?;
        Ok(Self::new(option))
    }

    /// Instrument identifier.
    ///
    /// Returns:
    ///     str: Unique identifier assigned to the instrument.
    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    /// Underlying ticker symbol.
    ///
    /// Returns:
    ///     str: Ticker for the underlying equity.
    #[getter]
    fn ticker(&self) -> &str {
        &self.inner.underlying_ticker
    }

    /// Strike price as money.
    ///
    /// Returns:
    ///     Money: Strike price wrapped as :class:`finstack.core.money.Money`.
    #[getter]
    fn strike(&self) -> PyMoney {
        PyMoney::new(self.inner.strike)
    }

    /// Option type label (``"call"``/``"put"``).
    ///
    /// Returns:
    ///     str: ``"call"`` or ``"put"`` depending on option direction.
    #[getter]
    fn option_type(&self) -> &'static str {
        match self.inner.option_type {
            OptionType::Call => "call",
            OptionType::Put => "put",
        }
    }

    /// Averaging method label.
    ///
    /// Returns:
    ///     str: ``"arithmetic"`` or ``"geometric"``.
    #[getter]
    fn averaging_method(&self) -> &'static str {
        match self.inner.averaging_method {
            AveragingMethod::Arithmetic => "arithmetic",
            AveragingMethod::Geometric => "geometric",
        }
    }

    /// Expiry date of the option.
    ///
    /// Returns:
    ///     datetime.date: Expiry date in calendar form.
    #[getter]
    fn expiry(&self, py: Python<'_>) -> PyResult<PyObject> {
        date_to_py(py, self.inner.expiry)
    }

    /// List of fixing dates for averaging.
    ///
    /// Returns:
    ///     list: List of :class:`datetime.date` objects.
    #[getter]
    fn fixing_dates(&self, py: Python<'_>) -> PyResult<PyObject> {
        let dates = PyList::empty(py);
        for d in &self.inner.fixing_dates {
            dates.append(date_to_py(py, *d)?)?;
        }
        Ok(dates.into())
    }

    /// Notional amount.
    ///
    /// Returns:
    ///     Money: Notional wrapped as :class:`finstack.core.money.Money`.
    #[getter]
    fn notional(&self) -> PyMoney {
        PyMoney::new(self.inner.notional)
    }

    /// Discount curve identifier.
    ///
    /// Returns:
    ///     str: Discount curve identifier.
    #[getter]
    fn discount_curve(&self) -> String {
        self.inner.disc_id.as_str().to_string()
    }

    /// Spot price identifier.
    ///
    /// Returns:
    ///     str: Spot price identifier.
    #[getter]
    fn spot_id(&self) -> &str {
        &self.inner.spot_id
    }

    /// Volatility surface identifier.
    ///
    /// Returns:
    ///     str: Volatility surface identifier used for pricing.
    #[getter]
    fn vol_surface(&self) -> String {
        self.inner.vol_id.as_str().to_string()
    }

    /// Dividend yield identifier (if any).
    ///
    /// Returns:
    ///     str | None: Dividend yield identifier or None.
    #[getter]
    fn dividend_yield_id(&self) -> Option<&str> {
        self.inner.div_yield_id.as_deref()
    }

    fn __repr__(&self) -> String {
        format!(
            "AsianOption(id='{}', ticker='{}', strike={}, expiry={}, averaging_method='{}')",
            self.inner.id.as_str(),
            self.inner.underlying_ticker,
            self.inner.strike.amount(),
            self.inner.expiry,
            match self.inner.averaging_method {
                AveragingMethod::Arithmetic => "arithmetic",
                AveragingMethod::Geometric => "geometric",
            }
        )
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    parent.add_class::<PyAveragingMethod>()?;
    parent.add_class::<PyAsianOption>()?;
    Ok(vec!["AveragingMethod", "AsianOption"])
}
