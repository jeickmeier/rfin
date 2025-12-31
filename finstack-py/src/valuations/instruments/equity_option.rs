use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::core::money::{extract_money, PyMoney};
use crate::valuations::common::PyInstrumentType;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::equity::equity_option::EquityOption;
use finstack_valuations::instruments::{ExerciseStyle, OptionType};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::Bound;
use std::fmt;

/// Equity option priced via Black–Scholes style models.
///
/// Examples:
///     >>> option = EquityOption.european_call(
///     ...     "opt_aapl_jan",
///     ...     "AAPL",
///     ...     180.0,
///     ...     date(2024, 1, 19),
///     ...     Money("USD", 100)
///     ... )
///     >>> option.option_type
///     'call'
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "EquityOption",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyEquityOption {
    pub(crate) inner: EquityOption,
}

impl PyEquityOption {
    pub(crate) fn new(inner: EquityOption) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyEquityOption {
    #[classmethod]
    #[pyo3(
        text_signature = "(cls, instrument_id, ticker, strike, expiry, notional, contract_size=1.0)"
    )]
    /// Create a European call option with standard market conventions.
    ///
    /// Args:
    ///     instrument_id: Instrument identifier or string-like object.
    ///     ticker: Equity ticker symbol for the underlying asset.
    ///     strike: Strike price expressed in quote currency units.
    ///     expiry: Option expiry date.
    ///     notional: Contract notional as :class:`finstack.core.money.Money`.
    ///     contract_size: Optional contract size multiplier.
    ///
    /// Returns:
    ///     EquityOption: Configured call option instrument.
    ///
    /// Raises:
    ///     ValueError: If identifiers or dates cannot be parsed.
    fn european_call(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        ticker: &str,
        strike: f64,
        expiry: Bound<'_, PyAny>,
        notional: Bound<'_, PyAny>,
        contract_size: Option<f64>,
    ) -> PyResult<Self> {
        use crate::errors::PyContext;
        let id = InstrumentId::new(instrument_id.extract::<&str>().context("instrument_id")?);
        let expiry_date = py_to_date(&expiry).context("expiry")?;
        let notional_money = extract_money(&notional).context("notional")?;
        let contract = contract_size.unwrap_or(1.0);
        Ok(Self::new(
            EquityOption::european_call(
                id.into_string(),
                ticker,
                strike,
                expiry_date,
                notional_money,
                contract,
            )
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?,
        ))
    }

    #[classmethod]
    #[pyo3(
        text_signature = "(cls, instrument_id, ticker, strike, expiry, notional, contract_size=1.0)"
    )]
    /// Create a European put option with standard market conventions.
    ///
    /// Args:
    ///     instrument_id: Instrument identifier or string-like object.
    ///     ticker: Equity ticker symbol for the underlying asset.
    ///     strike: Strike price expressed in quote currency units.
    ///     expiry: Option expiry date.
    ///     notional: Contract notional as :class:`finstack.core.money.Money`.
    ///     contract_size: Optional contract size multiplier.
    ///
    /// Returns:
    ///     EquityOption: Configured put option instrument.
    ///
    /// Raises:
    ///     ValueError: If identifiers or dates cannot be parsed.
    fn european_put(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        ticker: &str,
        strike: f64,
        expiry: Bound<'_, PyAny>,
        notional: Bound<'_, PyAny>,
        contract_size: Option<f64>,
    ) -> PyResult<Self> {
        use crate::errors::PyContext;
        let id = InstrumentId::new(instrument_id.extract::<&str>().context("instrument_id")?);
        let expiry_date = py_to_date(&expiry).context("expiry")?;
        let notional_money = extract_money(&notional).context("notional")?;
        let contract = contract_size.unwrap_or(1.0);
        Ok(Self::new(
            EquityOption::european_put(
                id.into_string(),
                ticker,
                strike,
                expiry_date,
                notional_money,
                contract,
            )
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?,
        ))
    }

    #[classmethod]
    #[pyo3(
        text_signature = "(cls, instrument_id, ticker, strike, expiry, notional, discount_curve, spot_id, vol_surface, /, *, div_yield_id=None, contract_size=1.0)",
        signature = (
            instrument_id,
            ticker,
            strike,
            expiry,
            notional,
            discount_curve,
            spot_id,
            vol_surface,
            *,
            div_yield_id=None,
            contract_size=None
        )
    )]
    #[allow(clippy::too_many_arguments)]
    /// Create an equity option with explicit discount curve, spot id, vol surface and optional dividend yield.
    fn builder(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        ticker: &str,
        strike: f64,
        expiry: Bound<'_, PyAny>,
        notional: Bound<'_, PyAny>,
        discount_curve: Bound<'_, PyAny>,
        spot_id: &str,
        vol_surface: Bound<'_, PyAny>,
        div_yield_id: Option<&str>,
        contract_size: Option<f64>,
    ) -> PyResult<Self> {
        use crate::errors::PyContext;
        use finstack_valuations::instruments::EquityUnderlyingParams;
        use finstack_valuations::instruments::equity::equity_option::EquityOptionParams;

        let id = InstrumentId::new(instrument_id.extract::<&str>().context("instrument_id")?);
        let expiry_date = py_to_date(&expiry).context("expiry")?;
        let notional_money = extract_money(&notional).context("notional")?;
        let discount_curve_id =
            CurveId::new(discount_curve.extract::<&str>().context("discount_curve")?);
        let vol_surface_id = vol_surface.extract::<&str>().context("vol_surface")?;

        let mut underlying =
            EquityUnderlyingParams::new(ticker, spot_id, notional_money.currency());
        if let Some(div) = div_yield_id {
            underlying = underlying.with_dividend_yield(div);
        }
        if let Some(cs) = contract_size {
            underlying = underlying.with_contract_size(cs);
        }

        let strike_money = finstack_core::money::Money::new(strike, notional_money.currency());
        let cs = contract_size.unwrap_or(1.0);
        let params = EquityOptionParams::european_call(strike_money, expiry_date, cs);
        let option = finstack_valuations::instruments::equity::equity_option::EquityOption::new(
            id.into_string(),
            &params,
            &underlying,
            discount_curve_id,
            vol_surface_id.into(),
        );
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

    /// Contract size (units per contract).
    ///
    /// Returns:
    ///     float: Number of underlying units per option contract.
    #[getter]
    fn contract_size(&self) -> f64 {
        self.inner.contract_size
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

    /// Exercise style label.
    ///
    /// Returns:
    ///     str: Exercise style such as ``"european"``.
    #[getter]
    fn exercise_style(&self) -> &'static str {
        match self.inner.exercise_style {
            ExerciseStyle::European => "european",
            ExerciseStyle::American => "american",
            ExerciseStyle::Bermudan => "bermudan",
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

    /// Discount curve identifier.
    ///
    /// Returns:
    ///     str: Discount curve identifier.
    #[getter]
    fn discount_curve(&self) -> String {
        self.inner.discount_curve_id.as_str().to_string()
    }

    /// Volatility surface identifier.
    ///
    /// Returns:
    ///     str: Volatility surface identifier used for pricing.
    #[getter]
    fn vol_surface(&self) -> String {
        self.inner.vol_surface_id.as_str().to_string()
    }

    /// Instrument type enum (``InstrumentType.EQUITY_OPTION``).
    ///
    /// Returns:
    ///     InstrumentType: Enumeration value identifying the instrument family.
    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::EquityOption)
    }

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!(
            "EquityOption(id='{}', ticker='{}', type='{}')",
            self.inner.id,
            self.inner.underlying_ticker,
            self.option_type()
        ))
    }
}

impl fmt::Display for PyEquityOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "EquityOption({}, ticker={}, type={})",
            self.inner.id,
            self.inner.underlying_ticker,
            self.option_type()
        )
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyEquityOption>()?;
    Ok(vec!["EquityOption"])
}
