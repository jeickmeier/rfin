use crate::core::money::PyMoney;
use crate::core::utils::{date_to_py, py_to_date};
use crate::valuations::common::{extract_curve_id, extract_instrument_id};
use finstack_valuations::instruments::quanto_option::QuantoOption;
use finstack_valuations::instruments::OptionType;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::Bound;

/// Quanto option instrument.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "QuantoOption",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyQuantoOption {
    pub(crate) inner: QuantoOption,
}

impl PyQuantoOption {
    pub(crate) fn new(inner: QuantoOption) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyQuantoOption {
    #[classmethod]
    #[pyo3(
        text_signature = "(cls, instrument_id, ticker, equity_strike, option_type, expiry, notional, domestic_currency, foreign_currency, correlation, discount_curve, spot_id, vol_surface, *, dividend_yield_id=None, fx_rate_id=None, fx_vol_id=None)"
    )]
    #[allow(clippy::too_many_arguments)]
    /// Create a quanto option.
    ///
    /// Args:
    ///     instrument_id: Instrument identifier.
    ///     ticker: Equity ticker symbol.
    ///     equity_strike: Strike price in foreign currency.
    ///     option_type: Option type (``"call"`` or ``"put"``).
    ///     expiry: Option expiry date.
    ///     notional: Contract notional amount.
    ///     domestic_currency: Currency for settlement.
    ///     foreign_currency: Currency of the underlying.
    ///     correlation: Correlation between equity and FX.
    ///     discount_curve: Discount curve identifier.
    ///     spot_id: Spot price identifier.
    ///     vol_surface: Volatility surface identifier.
    ///     dividend_yield_id: Optional dividend yield identifier.
    ///     fx_rate_id: Optional FX rate identifier.
    ///     fx_vol_id: Optional FX volatility surface identifier.
    ///
    /// Returns:
    ///     QuantoOption: Configured quanto option instrument.
    fn builder(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        ticker: &str,
        equity_strike: f64,
        option_type: &str,
        expiry: Bound<'_, PyAny>,
        notional: f64,
        domestic_currency: Bound<'_, PyAny>,
        foreign_currency: Bound<'_, PyAny>,
        correlation: f64,
        discount_curve: Bound<'_, PyAny>,
        spot_id: &str,
        vol_surface: Bound<'_, PyAny>,
        dividend_yield_id: Option<&str>,
        fx_rate_id: Option<&str>,
        fx_vol_id: Option<Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        use crate::core::common::args::CurrencyArg;
        use crate::core::common::labels::normalize_label;
        use finstack_core::dates::DayCount;

        let id = extract_instrument_id(&instrument_id)?;
        let expiry_date = py_to_date(&expiry)?;
        let disc_id = extract_curve_id(&discount_curve)?;
        let vol_id = extract_curve_id(&vol_surface)?;
        let CurrencyArg(dom_currency) = domestic_currency.extract()?;
        let CurrencyArg(for_currency) = foreign_currency.extract()?;

        let opt_type = match normalize_label(option_type).as_str() {
            "call" => OptionType::Call,
            "put" => OptionType::Put,
            other => {
                return Err(PyValueError::new_err(format!(
                    "Unknown option type: {other}"
                )))
            }
        };

        let equity_strike_money = finstack_core::money::Money::new(equity_strike, for_currency);

        let fx_vol_curve_id = fx_vol_id
            .map(|v| extract_curve_id(&v).ok())
            .flatten();

        let mut builder = QuantoOption::builder();
        builder = builder.id(id);
        builder = builder.underlying_ticker(ticker.to_string());
        builder = builder.equity_strike(equity_strike_money);
        builder = builder.option_type(opt_type);
        builder = builder.expiry(expiry_date);
        builder = builder.notional(notional);
        builder = builder.domestic_currency(dom_currency);
        builder = builder.foreign_currency(for_currency);
        builder = builder.correlation(correlation);
        builder = builder.day_count(DayCount::Act365F);
        builder = builder.disc_id(disc_id);
        builder = builder.spot_id(spot_id.to_string());
        builder = builder.vol_id(vol_id);
        if let Some(div) = dividend_yield_id {
            builder = builder.div_yield_id(div.to_string());
        }
        if let Some(fx_rate) = fx_rate_id {
            builder = builder.fx_rate_id(fx_rate.to_string());
        }
        if let Some(fx_vol) = fx_vol_curve_id {
            builder = builder.fx_vol_id(fx_vol);
        }
        let option = builder.build().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to build QuantoOption: {e}"))
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

    /// Equity strike as money.
    #[getter]
    fn equity_strike(&self) -> PyMoney {
        PyMoney::new(self.inner.equity_strike)
    }

    /// Option type label.
    #[getter]
    fn option_type(&self) -> &'static str {
        match self.inner.option_type {
            OptionType::Call => "call",
            OptionType::Put => "put",
        }
    }

    /// Expiry date.
    #[getter]
    fn expiry(&self, py: Python<'_>) -> PyResult<PyObject> {
        date_to_py(py, self.inner.expiry)
    }

    /// Notional amount.
    #[getter]
    fn notional(&self) -> f64 {
        self.inner.notional
    }

    /// Correlation between equity and FX.
    #[getter]
    fn correlation(&self) -> f64 {
        self.inner.correlation
    }

    fn __repr__(&self) -> String {
        format!(
            "QuantoOption(id='{}', ticker='{}', correlation={})",
            self.inner.id.as_str(),
            self.inner.underlying_ticker,
            self.inner.correlation
        )
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    parent.add_class::<PyQuantoOption>()?;
    Ok(vec!["QuantoOption"])
}

