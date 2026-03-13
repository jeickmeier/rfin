use crate::core::currency::PyCurrency;
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::core::market_data::PyMarketContext;
use crate::core::money::{extract_money, PyMoney};
use crate::errors::core_to_py;
use crate::valuations::common::PyInstrumentType;
use finstack_core::currency::Currency;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::fx::fx_forward::FxForward;
use finstack_valuations::instruments::Attributes;
use finstack_valuations::prelude::Instrument;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::{Bound, Py, PyRefMut};
use std::fmt;
use std::sync::Arc;

/// FX forward (outright forward) instrument.
///
/// Represents a commitment to exchange one currency for another at a specified
/// future date at a predetermined rate. The position is long base currency
/// (foreign) and short quote currency (domestic).
///
/// Pricing
/// -------
///
/// Forward value is calculated using covered interest rate parity:
///     F_market = S * DF_foreign(T) / DF_domestic(T)
///     PV = notional * (F_market - F_contract) * DF_domestic(T)
///
/// Examples
/// --------
/// Create a 6-month EUR/USD forward::
///
///     from finstack import Money, Date
///     from finstack.valuations.instruments import FxForward
///
///     fwd = (
///         FxForward.builder("EURUSD-FWD-6M")
///         .base_currency("EUR")
///         .quote_currency("USD")
///         .maturity(Date(2025, 6, 15))
///         .notional(Money.from_code(1_000_000, "EUR"))
///         .domestic_discount_curve("USD-OIS")
///         .foreign_discount_curve("EUR-OIS")
///         .contract_rate(1.12)
///         .build()
///     )
///
/// See Also
/// --------
/// Ndf : Non-deliverable forward
/// FxSwap : FX swap
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "FxForward",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyFxForward {
    pub(crate) inner: Arc<FxForward>,
}

impl PyFxForward {
    pub(crate) fn new(inner: FxForward) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

#[pyclass(
    module = "finstack.valuations.instruments",
    name = "FxForwardBuilder",
    unsendable
)]
pub struct PyFxForwardBuilder {
    instrument_id: InstrumentId,
    base_currency: Option<Currency>,
    quote_currency: Option<Currency>,
    maturity: Option<time::Date>,
    notional: Option<Money>,
    contract_rate: Option<f64>,
    domestic_discount_curve_id: Option<CurveId>,
    foreign_discount_curve_id: Option<CurveId>,
    spot_rate_override: Option<f64>,
    base_calendar_id: Option<String>,
    quote_calendar_id: Option<String>,
}

impl PyFxForwardBuilder {
    fn new_with_id(id: InstrumentId) -> Self {
        Self {
            instrument_id: id,
            base_currency: None,
            quote_currency: None,
            maturity: None,
            notional: None,
            contract_rate: None,
            domestic_discount_curve_id: None,
            foreign_discount_curve_id: None,
            spot_rate_override: None,
            base_calendar_id: None,
            quote_calendar_id: None,
        }
    }

    fn validate_and_build(&self) -> PyResult<FxForward> {
        let base_currency = self
            .base_currency
            .ok_or_else(|| PyValueError::new_err("base_currency is required"))?;

        let quote_currency = self
            .quote_currency
            .ok_or_else(|| PyValueError::new_err("quote_currency is required"))?;

        if base_currency == quote_currency {
            return Err(PyValueError::new_err(
                "base_currency must differ from quote_currency",
            ));
        }

        let maturity = self
            .maturity
            .ok_or_else(|| PyValueError::new_err("maturity is required"))?;

        let notional = self
            .notional
            .ok_or_else(|| PyValueError::new_err("notional is required"))?;

        if notional.currency() != base_currency {
            return Err(PyValueError::new_err(format!(
                "notional currency ({}) must match base_currency ({})",
                notional.currency(),
                base_currency
            )));
        }

        let domestic_discount_curve_id = self
            .domestic_discount_curve_id
            .clone()
            .ok_or_else(|| PyValueError::new_err("domestic_discount_curve is required"))?;

        let foreign_discount_curve_id = self
            .foreign_discount_curve_id
            .clone()
            .ok_or_else(|| PyValueError::new_err("foreign_discount_curve is required"))?;

        if let Some(rate) = self.contract_rate {
            if rate <= 0.0 {
                return Err(PyValueError::new_err("contract_rate must be positive"));
            }
        }

        FxForward::builder()
            .id(self.instrument_id.clone())
            .base_currency(base_currency)
            .quote_currency(quote_currency)
            .maturity(maturity)
            .notional(notional)
            .domestic_discount_curve_id(domestic_discount_curve_id)
            .foreign_discount_curve_id(foreign_discount_curve_id)
            .contract_rate_opt(self.contract_rate)
            .spot_rate_override_opt(self.spot_rate_override)
            .base_calendar_id_opt(self.base_calendar_id.clone())
            .quote_calendar_id_opt(self.quote_calendar_id.clone())
            .attributes(Attributes::new())
            .build()
            .map_err(core_to_py)
    }
}

#[pymethods]
impl PyFxForwardBuilder {
    fn base_currency<'py>(
        mut slf: PyRefMut<'py, Self>,
        ccy: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        use crate::core::currency::extract_currency;
        use crate::errors::PyContext;
        slf.base_currency = Some(extract_currency(&ccy).context("base_currency")?);
        Ok(slf)
    }

    fn quote_currency<'py>(
        mut slf: PyRefMut<'py, Self>,
        ccy: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        use crate::core::currency::extract_currency;
        use crate::errors::PyContext;
        slf.quote_currency = Some(extract_currency(&ccy).context("quote_currency")?);
        Ok(slf)
    }

    fn maturity<'py>(
        mut slf: PyRefMut<'py, Self>,
        date: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        use crate::errors::PyContext;
        slf.maturity = Some(py_to_date(&date).context("maturity")?);
        Ok(slf)
    }

    fn notional<'py>(
        mut slf: PyRefMut<'py, Self>,
        notional: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        use crate::errors::PyContext;
        slf.notional = Some(extract_money(&notional).context("notional")?);
        Ok(slf)
    }

    fn contract_rate<'py>(mut slf: PyRefMut<'py, Self>, rate: f64) -> PyRefMut<'py, Self> {
        slf.contract_rate = Some(rate);
        slf
    }

    fn domestic_discount_curve<'py>(
        mut slf: PyRefMut<'py, Self>,
        curve_id: &str,
    ) -> PyRefMut<'py, Self> {
        slf.domestic_discount_curve_id = Some(CurveId::new(curve_id));
        slf
    }

    fn foreign_discount_curve<'py>(
        mut slf: PyRefMut<'py, Self>,
        curve_id: &str,
    ) -> PyRefMut<'py, Self> {
        slf.foreign_discount_curve_id = Some(CurveId::new(curve_id));
        slf
    }

    fn spot_rate_override<'py>(mut slf: PyRefMut<'py, Self>, rate: f64) -> PyRefMut<'py, Self> {
        slf.spot_rate_override = Some(rate);
        slf
    }

    fn base_calendar<'py>(mut slf: PyRefMut<'py, Self>, calendar_id: &str) -> PyRefMut<'py, Self> {
        slf.base_calendar_id = Some(calendar_id.to_string());
        slf
    }

    fn quote_calendar<'py>(mut slf: PyRefMut<'py, Self>, calendar_id: &str) -> PyRefMut<'py, Self> {
        slf.quote_calendar_id = Some(calendar_id.to_string());
        slf
    }

    fn build(slf: PyRefMut<'_, Self>) -> PyResult<PyFxForward> {
        let inner = slf.validate_and_build()?;
        Ok(PyFxForward::new(inner))
    }

    fn __repr__(&self) -> String {
        format!("FxForwardBuilder(id='{}')", self.instrument_id)
    }
}

#[pymethods]
impl PyFxForward {
    /// Create a builder for an FX forward contract.
    ///
    /// Parameters
    /// ----------
    /// instrument_id : str
    ///     Unique instrument identifier (e.g., "EURUSD-FWD-6M")
    ///
    /// Returns
    /// -------
    /// FxForwardBuilder
    ///     Builder instance for fluent configuration
    #[classmethod]
    fn builder<'py>(
        cls: &Bound<'py, PyType>,
        instrument_id: &str,
    ) -> PyResult<Py<PyFxForwardBuilder>> {
        let py = cls.py();
        let builder = PyFxForwardBuilder::new_with_id(InstrumentId::new(instrument_id));
        Py::new(py, builder)
    }

    /// Instrument identifier.
    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    /// Instrument type.
    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::FxForward)
    }

    /// Base currency (foreign currency, numerator of the pair).
    #[getter]
    fn base_currency(&self) -> PyCurrency {
        PyCurrency::new(self.inner.base_currency)
    }

    /// Quote currency (domestic currency, denominator of the pair).
    #[getter]
    fn quote_currency(&self) -> PyCurrency {
        PyCurrency::new(self.inner.quote_currency)
    }

    /// Maturity/settlement date.
    #[getter]
    fn maturity<'py>(&self, py: Python<'py>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.maturity)
    }

    /// Notional amount in base currency.
    #[getter]
    fn notional(&self) -> PyMoney {
        PyMoney::new(self.inner.notional)
    }

    /// Contract forward rate (quote per base). None if at-market.
    #[getter]
    fn contract_rate(&self) -> Option<f64> {
        self.inner.contract_rate
    }

    /// Spot rate override (if set).
    #[getter]
    fn spot_rate_override(&self) -> Option<f64> {
        self.inner.spot_rate_override
    }

    /// Domestic (quote currency) discount curve identifier.
    #[getter]
    fn domestic_discount_curve(&self) -> String {
        self.inner.domestic_discount_curve_id.as_str().to_string()
    }

    /// Foreign (base currency) discount curve identifier.
    #[getter]
    fn foreign_discount_curve(&self) -> String {
        self.inner.foreign_discount_curve_id.as_str().to_string()
    }

    /// Base currency calendar identifier (if set).
    #[getter]
    fn base_calendar(&self) -> Option<String> {
        self.inner.base_calendar_id.clone()
    }

    /// Quote currency calendar identifier (if set).
    #[getter]
    fn quote_calendar(&self) -> Option<String> {
        self.inner.quote_calendar_id.clone()
    }

    /// Calculate present value of the FX forward.
    ///
    /// Parameters
    /// ----------
    /// market : MarketContext
    ///     Market data including discount curves and FX rates
    /// as_of : Date
    ///     Valuation date
    ///
    /// Returns
    /// -------
    /// Money
    ///     Present value in quote currency
    fn value(
        &self,
        py: Python<'_>,
        market: &PyMarketContext,
        as_of: Bound<'_, PyAny>,
    ) -> PyResult<PyMoney> {
        use crate::errors::PyContext;
        let date = py_to_date(&as_of).context("as_of")?;
        let value = py
            .detach(|| self.inner.value(&market.inner, date))
            .map_err(core_to_py)?;
        Ok(PyMoney::new(value))
    }

    /// Compute the market forward rate via covered interest rate parity.
    ///
    /// Parameters
    /// ----------
    /// market : MarketContext
    ///     Market data including discount curves and FX rates
    /// as_of : Date
    ///     Valuation date
    ///
    /// Returns
    /// -------
    /// float
    ///     Forward rate (quote per base)
    fn market_forward_rate(
        &self,
        py: Python<'_>,
        market: &PyMarketContext,
        as_of: Bound<'_, PyAny>,
    ) -> PyResult<f64> {
        use crate::errors::PyContext;
        let date = py_to_date(&as_of).context("as_of")?;
        py.detach(|| self.inner.market_forward_rate(&market.inner, date))
            .map_err(core_to_py)
    }

    fn __repr__(&self) -> String {
        let rate_str = match self.inner.contract_rate {
            Some(r) => format!("contract_rate={}", r),
            None => "at-market".to_string(),
        };
        format!(
            "FxForward(id='{}', {}/{}, {}, maturity={})",
            self.inner.id,
            self.inner.base_currency,
            self.inner.quote_currency,
            rate_str,
            self.inner.maturity
        )
    }
}

impl fmt::Display for PyFxForward {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "FxForward({}, {}/{})",
            self.inner.id, self.inner.base_currency, self.inner.quote_currency
        )
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyFxForward>()?;
    module.add_class::<PyFxForwardBuilder>()?;
    Ok(vec!["FxForward", "FxForwardBuilder"])
}
