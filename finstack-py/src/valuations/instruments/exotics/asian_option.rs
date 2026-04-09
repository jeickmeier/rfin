use crate::core::dates::daycount::PyDayCount;
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::core::money::{extract_money, PyMoney};
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::exotics::asian_option::{AsianOption, AveragingMethod};
use finstack_valuations::instruments::OptionType;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyList, PyModule, PyTuple, PyType};
use pyo3::Bound;
use std::str::FromStr;
use std::sync::Arc;

/// Averaging method for Asian options.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "AveragingMethod",
    frozen,
    from_py_object
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
        AveragingMethod::from_str(name)
            .map(Self::new)
            .map_err(|e| PyValueError::new_err(e.to_string()))
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
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyAsianOption {
    pub(crate) inner: Arc<AsianOption>,
}

impl PyAsianOption {
    pub(crate) fn new(inner: AsianOption) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

#[pymethods]
impl PyAsianOption {
    #[classmethod]
    #[pyo3(
        signature = (instrument_id, ticker, strike, expiry, fixing_dates, notional, discount_curve, spot_id, vol_surface, *, averaging_method=None, option_type=None, div_yield_id=None, past_fixings=None),
        text_signature = "(cls, instrument_id, ticker, strike, expiry, fixing_dates, notional, discount_curve, spot_id, vol_surface, *, averaging_method='arithmetic', option_type='call', div_yield_id=None, past_fixings=None)"
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
    ///     div_yield_id: Optional dividend yield identifier.
    ///     past_fixings: Optional list of ``(date, float)`` tuples for seasoned options.
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
        div_yield_id: Option<&str>,
        past_fixings: Option<Bound<'_, PyList>>,
    ) -> PyResult<Self> {
        use crate::errors::PyContext;
        use finstack_core::dates::DayCount;

        let id = InstrumentId::new(instrument_id.extract::<&str>().context("instrument_id")?);
        let expiry_date = py_to_date(&expiry).context("expiry")?;
        let notional_money = extract_money(&notional).context("notional")?;
        let discount_curve_id =
            CurveId::new(discount_curve.extract::<&str>().context("discount_curve")?);
        let vol_surface_id = CurveId::new(vol_surface.extract::<&str>().context("vol_surface")?);

        let mut fixing_dates_vec = Vec::new();
        for item in fixing_dates.iter() {
            fixing_dates_vec.push(py_to_date(&item).context("fixing_dates")?);
        }

        let avg_method = match averaging_method {
            Some(m) => {
                AveragingMethod::from_str(m).map_err(|e| PyValueError::new_err(e.to_string()))?
            }
            None => AveragingMethod::Arithmetic,
        };

        let opt_type = match option_type {
            Some(t) => OptionType::from_str(t).map_err(|e| PyValueError::new_err(e.to_string()))?,
            None => OptionType::Call,
        };

        let mut past_fixings_vec = Vec::new();
        if let Some(pf) = past_fixings {
            for item in pf.iter() {
                let tuple = item.cast::<PyTuple>().map_err(|_| {
                    PyValueError::new_err("past_fixings must be a list of (date, float) tuples")
                })?;
                if tuple.len() != 2 {
                    return Err(PyValueError::new_err(
                        "Each past_fixings entry must be a (date, float) tuple",
                    ));
                }
                let date = py_to_date(&tuple.get_item(0)?).context("past_fixings date")?;
                let price = tuple
                    .get_item(1)?
                    .extract::<f64>()
                    .map_err(|_| PyValueError::new_err("past_fixings price must be a float"))?;
                past_fixings_vec.push((date, price));
            }
        }

        let mut builder = AsianOption::builder();
        builder = builder.id(id);
        builder = builder.underlying_ticker(ticker.to_string());
        builder = builder.strike(strike);
        builder = builder.option_type(opt_type);
        builder = builder.averaging_method(avg_method);
        builder = builder.expiry(expiry_date);
        builder = builder.fixing_dates(fixing_dates_vec);
        builder = builder.notional(notional_money);
        builder = builder.day_count(DayCount::Act365F);
        builder = builder
            .pricing_overrides(finstack_valuations::instruments::PricingOverrides::default());
        builder = builder.discount_curve_id(discount_curve_id);
        builder = builder.spot_id(spot_id.to_string().into());
        builder = builder.vol_surface_id(vol_surface_id);
        if let Some(div) = div_yield_id {
            builder = builder.div_yield_id(div.into());
        }
        if !past_fixings_vec.is_empty() {
            builder = builder.past_fixings(past_fixings_vec);
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

    /// Strike price as scalar.
    ///
    /// Returns:
    ///     float: Strike price in underlying price units.
    #[getter]
    fn strike(&self) -> f64 {
        self.inner.strike
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
    fn expiry(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.expiry)
    }

    /// List of fixing dates for averaging.
    ///
    /// Returns:
    ///     list: List of :class:`datetime.date` objects.
    #[getter]
    fn fixing_dates(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let dates = PyList::empty(py);
        for d in &self.inner.fixing_dates {
            dates.append(date_to_py(py, *d)?)?;
        }
        Ok(dates.into())
    }

    /// Past fixings for seasoned options.
    ///
    /// Returns:
    ///     list: List of ``(datetime.date, float)`` tuples, or empty list.
    #[getter]
    fn past_fixings(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let result = PyList::empty(py);
        for (date, price) in &self.inner.past_fixings {
            let py_date = date_to_py(py, *date)?;
            let tuple = PyTuple::new(py, &[py_date, price.into_pyobject(py)?.into_any().unbind()])?;
            result.append(tuple)?;
        }
        Ok(result.into())
    }

    /// Notional amount.
    ///
    /// Returns:
    ///     Money: Notional wrapped as :class:`finstack.core.money.Money`.
    #[getter]
    fn notional(&self) -> PyMoney {
        PyMoney::new(self.inner.notional)
    }

    /// Day count convention.
    ///
    /// Returns:
    ///     DayCount: Day count convention used for time fraction calculations.
    #[getter]
    fn day_count(&self) -> PyDayCount {
        PyDayCount::new(self.inner.day_count)
    }

    /// Discount curve identifier.
    ///
    /// Returns:
    ///     str: Discount curve identifier.
    #[getter]
    fn discount_curve(&self) -> String {
        self.inner.discount_curve_id.as_str().to_string()
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
        self.inner.vol_surface_id.as_str().to_string()
    }

    /// Dividend yield identifier (if any).
    ///
    /// Returns:
    ///     str | None: Dividend yield identifier or None.
    #[getter]
    fn div_yield_id(&self) -> Option<&str> {
        self.inner.div_yield_id.as_deref()
    }

    fn __repr__(&self) -> String {
        format!(
            "AsianOption(id='{}', ticker='{}', strike={}, expiry={}, averaging_method='{}')",
            self.inner.id.as_str(),
            self.inner.underlying_ticker,
            self.inner.strike,
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
