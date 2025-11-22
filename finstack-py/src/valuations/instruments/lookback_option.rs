use crate::core::money::{extract_money, PyMoney};
use crate::core::utils::{date_to_py, py_to_date};
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::lookback_option::{LookbackOption, LookbackType};
use finstack_valuations::instruments::OptionType;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::Bound;

/// Lookback option type.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "LookbackType",
    frozen
)]
#[derive(Clone, Copy, Debug)]
pub struct PyLookbackType {
    pub(crate) inner: LookbackType,
}

impl PyLookbackType {
    pub(crate) const fn new(inner: LookbackType) -> Self {
        Self { inner }
    }

    fn label(&self) -> &'static str {
        match self.inner {
            LookbackType::FixedStrike => "fixed_strike",
            LookbackType::FloatingStrike => "floating_strike",
        }
    }
}

#[pymethods]
impl PyLookbackType {
    #[classattr]
    const FIXED_STRIKE: Self = Self::new(LookbackType::FixedStrike);
    #[classattr]
    const FLOATING_STRIKE: Self = Self::new(LookbackType::FloatingStrike);

    #[classmethod]
    #[pyo3(text_signature = "(cls, name)")]
    /// Parse a lookback type from a string label.
    fn from_name(_cls: &Bound<'_, PyType>, name: &str) -> PyResult<Self> {
        use crate::core::common::labels::normalize_label;
        match normalize_label(name).as_str() {
            "fixed_strike" | "fixedstrike" => Ok(Self::new(LookbackType::FixedStrike)),
            "floating_strike" | "floatingstrike" => Ok(Self::new(LookbackType::FloatingStrike)),
            other => Err(PyValueError::new_err(format!(
                "Unknown lookback type: {other}"
            ))),
        }
    }

    #[getter]
    fn name(&self) -> &'static str {
        self.label()
    }

    fn __repr__(&self) -> String {
        format!("LookbackType('{}')", self.label())
    }

    fn __str__(&self) -> &'static str {
        self.label()
    }
}

/// Lookback option instrument.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "LookbackOption",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyLookbackOption {
    pub(crate) inner: LookbackOption,
}

impl PyLookbackOption {
    pub(crate) fn new(inner: LookbackOption) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyLookbackOption {
    #[classmethod]
    #[pyo3(
        text_signature = "(cls, instrument_id, ticker, strike, option_type, lookback_type, expiry, notional, discount_curve, spot_id, vol_surface, *, div_yield_id=None)"
    )]
    #[allow(clippy::too_many_arguments)]
    /// Create a lookback option.
    ///
    /// Args:
    ///     instrument_id: Instrument identifier.
    ///     ticker: Equity ticker symbol.
    ///     strike: Strike price (None for floating strike).
    ///     option_type: Option type (``"call"`` or ``"put"``).
    ///     lookback_type: Lookback type (``"fixed_strike"`` or ``"floating_strike"``).
    ///     expiry: Option expiry date.
    ///     notional: Contract notional as :class:`finstack.core.money.Money`.
    ///     discount_curve: Discount curve identifier.
    ///     spot_id: Spot price identifier.
    ///     vol_surface: Volatility surface identifier.
    ///     div_yield_id: Optional dividend yield identifier.
    ///
    /// Returns:
    ///     LookbackOption: Configured lookback option instrument.
    fn builder(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        ticker: &str,
        strike: Option<f64>,
        option_type: &str,
        lookback_type: &str,
        expiry: Bound<'_, PyAny>,
        notional: Bound<'_, PyAny>,
        discount_curve: Bound<'_, PyAny>,
        spot_id: &str,
        vol_surface: Bound<'_, PyAny>,
        div_yield_id: Option<&str>,
    ) -> PyResult<Self> {
        use crate::core::common::labels::normalize_label;
        use finstack_core::dates::DayCount;
        use crate::errors::PyContext;

        let id = InstrumentId::new(instrument_id.extract::<&str>().context("instrument_id")?);
        let expiry_date = py_to_date(&expiry).context("expiry")?;
        let notional_money = extract_money(&notional).context("notional")?;
        let discount_curve_id = CurveId::new(discount_curve.extract::<&str>().context("discount_curve")?);
        let vol_surface_id = CurveId::new(vol_surface.extract::<&str>().context("vol_surface")?);

        let opt_type = match normalize_label(option_type).as_str() {
            "call" => OptionType::Call,
            "put" => OptionType::Put,
            other => {
                return Err(PyValueError::new_err(format!(
                    "Unknown option type: {other}"
                )))
            }
        };

        let lookback_type_enum = match normalize_label(lookback_type).as_str() {
            "fixed_strike" | "fixedstrike" => LookbackType::FixedStrike,
            "floating_strike" | "floatingstrike" => LookbackType::FloatingStrike,
            other => {
                return Err(PyValueError::new_err(format!(
                    "Unknown lookback type: {other}"
                )))
            }
        };

        let mut builder = LookbackOption::builder()
            .id(id)
            .underlying_ticker(ticker.to_string());

        if let Some(s) = strike {
            let strike_money = finstack_core::money::Money::new(s, notional_money.currency());
            builder = builder.strike(strike_money);
        }
        builder = builder.option_type(opt_type);
        builder = builder.lookback_type(lookback_type_enum);
        builder = builder.expiry(expiry_date);
        builder = builder.notional(notional_money);
        builder = builder.day_count(DayCount::Act365F);
        builder = builder
            .pricing_overrides(finstack_valuations::instruments::PricingOverrides::default());
        builder = builder.discount_curve_id(discount_curve_id);
        builder = builder.spot_id(spot_id.to_string());
        builder = builder.vol_surface_id(vol_surface_id.into());
        if let Some(div) = div_yield_id {
            builder = builder.div_yield_id(div.to_string());
        }
        let option = builder.build().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!(
                "Failed to build LookbackOption: {e}"
            ))
        })?;
        Ok(Self::new(option))
    }

    /// Instrument identifier.
    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    /// Underlying ticker symbol.
    #[getter]
    fn ticker(&self) -> &str {
        &self.inner.underlying_ticker
    }

    /// Strike price as money (None for floating strike).
    #[getter]
    fn strike(&self) -> Option<PyMoney> {
        self.inner.strike.map(PyMoney::new)
    }

    /// Option type label.
    #[getter]
    fn option_type(&self) -> &'static str {
        match self.inner.option_type {
            OptionType::Call => "call",
            OptionType::Put => "put",
        }
    }

    /// Lookback type label.
    #[getter]
    fn lookback_type(&self) -> &'static str {
        match self.inner.lookback_type {
            LookbackType::FixedStrike => "fixed_strike",
            LookbackType::FloatingStrike => "floating_strike",
        }
    }

    /// Expiry date.
    #[getter]
    fn expiry(&self, py: Python<'_>) -> PyResult<PyObject> {
        date_to_py(py, self.inner.expiry)
    }

    /// Notional amount.
    ///
    /// Returns:
    ///     Money: Notional wrapped as :class:`finstack.core.money.Money`.
    #[getter]
    fn notional(&self) -> PyMoney {
        PyMoney::new(self.inner.notional)
    }

    fn __repr__(&self) -> String {
        format!(
            "LookbackOption(id='{}', ticker='{}', lookback_type='{}')",
            self.inner.id.as_str(),
            self.inner.underlying_ticker,
            match self.inner.lookback_type {
                LookbackType::FixedStrike => "fixed_strike",
                LookbackType::FloatingStrike => "floating_strike",
            }
        )
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    parent.add_class::<PyLookbackType>()?;
    parent.add_class::<PyLookbackOption>()?;
    Ok(vec!["LookbackType", "LookbackOption"])
}
