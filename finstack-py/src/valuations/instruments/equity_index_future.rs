use crate::core::common::labels::normalize_label;
use crate::core::currency::PyCurrency;
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::valuations::common::PyInstrumentType;
use finstack_core::currency::Currency;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::equity::equity_index_future::{
    EquityFutureSpecs, EquityIndexFuture,
};
use finstack_valuations::instruments::rates::ir_future::Position;
use finstack_valuations::instruments::Attributes;
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::{Bound, Py, PyRef, PyRefMut};
use std::fmt;
use std::sync::Arc;

/// Position side (Long or Short) for futures contracts.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "FuturePosition",
    frozen
)]
#[derive(Clone, Copy, Debug)]
pub struct PyPosition {
    pub(crate) inner: Position,
}

impl PyPosition {
    const fn new(inner: Position) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyPosition {
    #[classattr]
    const LONG: Self = Self::new(Position::Long);
    #[classattr]
    const SHORT: Self = Self::new(Position::Short);

    fn __repr__(&self) -> &'static str {
        match self.inner {
            Position::Long => "FuturePosition.LONG",
            Position::Short => "FuturePosition.SHORT",
            _ => unreachable!("unknown Position variant"),
        }
    }

    fn __str__(&self) -> &'static str {
        match self.inner {
            Position::Long => "long",
            Position::Short => "short",
            _ => unreachable!("unknown Position variant"),
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct PositionArg(PyPosition);

impl<'py> FromPyObject<'py> for PositionArg {
    fn extract_bound(obj: &Bound<'py, PyAny>) -> PyResult<Self> {
        if let Ok(existing) = obj.extract::<PyRef<'py, PyPosition>>() {
            return Ok(PositionArg(*existing));
        }

        if let Ok(label) = obj.extract::<&str>() {
            let normalized = normalize_label(label);
            let position = match normalized.as_str() {
                "long" | "buy" => Position::Long,
                "short" | "sell" => Position::Short,
                other => {
                    return Err(PyValueError::new_err(format!(
                        "Unknown position: {other}. Expected 'long' or 'short'"
                    )))
                }
            };
            return Ok(PositionArg(PyPosition::new(position)));
        }

        Err(PyTypeError::new_err(
            "Expected FuturePosition or string identifier",
        ))
    }
}

/// Equity index future contract specifications.
///
/// Contains exchange-specific contract parameters such as multiplier,
/// tick size, and settlement method.
///
/// Examples
/// --------
/// Create E-mini S&P 500 specifications::
///
///     specs = EquityIndexFuture.sp500_emini_specs()
///     print(specs.multiplier)  # 50.0
///     print(specs.tick_size)   # 0.25
///
/// Create custom specifications::
///
///     custom = EquityFutureSpecs(
///         multiplier=10.0,
///         tick_size=0.5,
///         tick_value=5.0,
///         settlement_method="Cash settled"
///     )
#[pyclass(module = "finstack.valuations.instruments", name = "EquityFutureSpecs")]
#[derive(Clone, Debug)]
pub struct PyEquityFutureSpecs {
    pub(crate) inner: EquityFutureSpecs,
}

#[pymethods]
impl PyEquityFutureSpecs {
    /// Create custom equity future specifications.
    ///
    /// Parameters
    /// ----------
    /// multiplier : float
    ///     Contract multiplier (currency per index point)
    /// tick_size : float
    ///     Minimum price increment in index points
    /// tick_value : float
    ///     Value of one tick in currency units
    /// settlement_method : str
    ///     Settlement method description
    #[new]
    #[pyo3(signature = (multiplier, tick_size, tick_value, settlement_method))]
    fn new_py(multiplier: f64, tick_size: f64, tick_value: f64, settlement_method: String) -> Self {
        Self {
            inner: EquityFutureSpecs {
                multiplier,
                tick_size,
                tick_value,
                settlement_method,
            },
        }
    }

    /// Contract multiplier (currency per index point).
    #[getter]
    fn multiplier(&self) -> f64 {
        self.inner.multiplier
    }

    /// Tick size in index points.
    #[getter]
    fn tick_size(&self) -> f64 {
        self.inner.tick_size
    }

    /// Tick value in currency units.
    #[getter]
    fn tick_value(&self) -> f64 {
        self.inner.tick_value
    }

    /// Settlement method description.
    #[getter]
    fn settlement_method(&self) -> String {
        self.inner.settlement_method.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "EquityFutureSpecs(multiplier={}, tick_size={}, tick_value={})",
            self.inner.multiplier, self.inner.tick_size, self.inner.tick_value
        )
    }
}

/// Equity index future contract.
///
/// Represents a futures contract on an equity index such as S&P 500, Nasdaq-100,
/// Euro Stoxx 50, DAX, FTSE 100, or Nikkei 225.
///
/// The contract supports two pricing modes:
///
/// 1. **Mark-to-Market** (when quoted_price is provided):
///    NPV = (quoted_price - entry_price) × multiplier × quantity × position_sign
///
/// 2. **Fair Value** (cost-of-carry model):
///    F = S₀ × exp((r - q) × T)
///    NPV = (F - entry_price) × multiplier × quantity × position_sign
///
/// where:
/// - S₀ = Current spot index level
/// - r = Risk-free rate (from discount curve)
/// - q = Continuous dividend yield
/// - T = Time to expiry in years
///
/// Examples
/// --------
/// Create an E-mini S&P 500 future::
///
///     from finstack import Date
///     from finstack.valuations.instruments import EquityIndexFuture
///
///     future = (
///         EquityIndexFuture.builder("ES-2025M03")
///         .index_ticker("SPX")
///         .currency("USD")
///         .quantity(10.0)
///         .expiry_date(Date(2025, 3, 21))
///         .last_trading_date(Date(2025, 3, 20))
///         .entry_price(4500.0)
///         .quoted_price(4550.0)
///         .position("long")
///         .contract_specs(EquityIndexFuture.sp500_emini_specs())
///         .discount_curve("USD-OIS")
///         .index_price_id("SPX-SPOT")
///         .build()
///     )
///
/// See Also
/// --------
/// InterestRateFuture : Interest rate futures
/// BondFuture : Bond futures
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "EquityIndexFuture",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyEquityIndexFuture {
    pub(crate) inner: Arc<EquityIndexFuture>,
}

impl PyEquityIndexFuture {
    pub(crate) fn new(inner: EquityIndexFuture) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

#[pyclass(
    module = "finstack.valuations.instruments",
    name = "EquityIndexFutureBuilder",
    unsendable
)]
pub struct PyEquityIndexFutureBuilder {
    instrument_id: InstrumentId,
    index_ticker: Option<String>,
    currency: Option<Currency>,
    quantity: Option<f64>,
    expiry_date: Option<time::Date>,
    last_trading_date: Option<time::Date>,
    entry_price: Option<f64>,
    quoted_price: Option<f64>,
    position: Position,
    contract_specs: Option<EquityFutureSpecs>,
    discount_curve_id: Option<CurveId>,
    index_price_id: Option<String>,
    dividend_yield_id: Option<String>,
}

impl PyEquityIndexFutureBuilder {
    fn new_with_id(id: InstrumentId) -> Self {
        Self {
            instrument_id: id,
            index_ticker: None,
            currency: None,
            quantity: None,
            expiry_date: None,
            last_trading_date: None,
            entry_price: None,
            quoted_price: None,
            position: Position::Long,
            contract_specs: None,
            discount_curve_id: None,
            index_price_id: None,
            dividend_yield_id: None,
        }
    }

    fn validate_and_build(&self) -> PyResult<EquityIndexFuture> {
        use crate::errors::core_to_py;

        let index_ticker = self.index_ticker.clone().ok_or_else(|| {
            PyValueError::new_err("index_ticker is required (e.g., 'SPX', 'NDX')")
        })?;

        let currency = self
            .currency
            .ok_or_else(|| PyValueError::new_err("currency is required"))?;

        let quantity = self
            .quantity
            .ok_or_else(|| PyValueError::new_err("quantity is required"))?;

        let expiry_date = self
            .expiry_date
            .ok_or_else(|| PyValueError::new_err("expiry_date is required"))?;

        let last_trading_date = self
            .last_trading_date
            .ok_or_else(|| PyValueError::new_err("last_trading_date is required"))?;

        let discount_curve_id = self
            .discount_curve_id
            .clone()
            .ok_or_else(|| PyValueError::new_err("discount_curve_id is required"))?;

        let index_price_id = self
            .index_price_id
            .clone()
            .ok_or_else(|| PyValueError::new_err("index_price_id is required"))?;

        let contract_specs = self.contract_specs.clone().unwrap_or_default();

        EquityIndexFuture::builder()
            .id(self.instrument_id.clone())
            .index_ticker(index_ticker)
            .currency(currency)
            .quantity(quantity)
            .expiry_date(expiry_date)
            .last_trading_date(last_trading_date)
            .entry_price_opt(self.entry_price)
            .quoted_price_opt(self.quoted_price)
            .position(self.position)
            .contract_specs(contract_specs)
            .discount_curve_id(discount_curve_id)
            .index_price_id(index_price_id)
            .dividend_yield_id_opt(self.dividend_yield_id.clone())
            .attributes(Attributes::new())
            .build()
            .map_err(core_to_py)
    }
}

#[pymethods]
impl PyEquityIndexFutureBuilder {
    fn index_ticker<'py>(mut slf: PyRefMut<'py, Self>, ticker: &str) -> PyRefMut<'py, Self> {
        slf.index_ticker = Some(ticker.to_string());
        slf
    }

    fn currency<'py>(
        mut slf: PyRefMut<'py, Self>,
        ccy: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        use crate::core::currency::extract_currency;
        slf.currency = Some(extract_currency(&ccy)?);
        Ok(slf)
    }

    fn quantity<'py>(mut slf: PyRefMut<'py, Self>, qty: f64) -> PyRefMut<'py, Self> {
        slf.quantity = Some(qty);
        slf
    }

    fn expiry_date<'py>(
        mut slf: PyRefMut<'py, Self>,
        date: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.expiry_date = Some(py_to_date(&date)?);
        Ok(slf)
    }

    fn last_trading_date<'py>(
        mut slf: PyRefMut<'py, Self>,
        date: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.last_trading_date = Some(py_to_date(&date)?);
        Ok(slf)
    }

    fn entry_price<'py>(mut slf: PyRefMut<'py, Self>, price: f64) -> PyRefMut<'py, Self> {
        slf.entry_price = Some(price);
        slf
    }

    fn quoted_price<'py>(mut slf: PyRefMut<'py, Self>, price: f64) -> PyRefMut<'py, Self> {
        slf.quoted_price = Some(price);
        slf
    }

    fn position<'py>(mut slf: PyRefMut<'py, Self>, pos: PositionArg) -> PyRefMut<'py, Self> {
        slf.position = pos.0.inner;
        slf
    }

    fn contract_specs<'py>(
        mut slf: PyRefMut<'py, Self>,
        specs: &PyEquityFutureSpecs,
    ) -> PyRefMut<'py, Self> {
        slf.contract_specs = Some(specs.inner.clone());
        slf
    }

    fn discount_curve<'py>(mut slf: PyRefMut<'py, Self>, curve_id: &str) -> PyRefMut<'py, Self> {
        slf.discount_curve_id = Some(CurveId::new(curve_id));
        slf
    }

    fn index_price_id<'py>(mut slf: PyRefMut<'py, Self>, id: &str) -> PyRefMut<'py, Self> {
        slf.index_price_id = Some(id.to_string());
        slf
    }

    fn dividend_yield_id<'py>(mut slf: PyRefMut<'py, Self>, id: &str) -> PyRefMut<'py, Self> {
        slf.dividend_yield_id = Some(id.to_string());
        slf
    }

    fn build(slf: PyRef<'_, Self>) -> PyResult<PyEquityIndexFuture> {
        let inner = slf.validate_and_build()?;
        Ok(PyEquityIndexFuture::new(inner))
    }

    fn __repr__(&self) -> String {
        format!("EquityIndexFutureBuilder(id='{}')", self.instrument_id)
    }
}

#[pymethods]
impl PyEquityIndexFuture {
    /// Create a builder for an equity index future contract.
    ///
    /// Parameters
    /// ----------
    /// instrument_id : str
    ///     Unique instrument identifier (e.g., "ES-2025M03")
    ///
    /// Returns
    /// -------
    /// EquityIndexFutureBuilder
    ///     Builder instance for fluent configuration
    #[classmethod]
    fn builder(_cls: &Bound<'_, PyType>, instrument_id: &str) -> PyEquityIndexFutureBuilder {
        PyEquityIndexFutureBuilder::new_with_id(InstrumentId::new(instrument_id))
    }

    /// Create E-mini S&P 500 contract specifications.
    ///
    /// Returns
    /// -------
    /// EquityFutureSpecs
    ///     CME E-mini S&P 500 specifications (multiplier=50, tick_size=0.25)
    #[classmethod]
    fn sp500_emini_specs(_cls: &Bound<'_, PyType>) -> PyEquityFutureSpecs {
        PyEquityFutureSpecs {
            inner: EquityFutureSpecs::sp500_emini(),
        }
    }

    /// Create E-mini Nasdaq-100 contract specifications.
    ///
    /// Returns
    /// -------
    /// EquityFutureSpecs
    ///     CME E-mini Nasdaq-100 specifications (multiplier=20, tick_size=0.25)
    #[classmethod]
    fn nasdaq100_emini_specs(_cls: &Bound<'_, PyType>) -> PyEquityFutureSpecs {
        PyEquityFutureSpecs {
            inner: EquityFutureSpecs::nasdaq100_emini(),
        }
    }

    /// Create Micro E-mini S&P 500 contract specifications.
    ///
    /// Returns
    /// -------
    /// EquityFutureSpecs
    ///     CME Micro E-mini S&P 500 specifications (multiplier=5, tick_size=0.25)
    #[classmethod]
    fn sp500_micro_emini_specs(_cls: &Bound<'_, PyType>) -> PyEquityFutureSpecs {
        PyEquityFutureSpecs {
            inner: EquityFutureSpecs::sp500_micro_emini(),
        }
    }

    /// Create Euro Stoxx 50 future contract specifications.
    ///
    /// Returns
    /// -------
    /// EquityFutureSpecs
    ///     Eurex Euro Stoxx 50 specifications (multiplier=10, tick_size=1.0)
    #[classmethod]
    fn euro_stoxx_50_specs(_cls: &Bound<'_, PyType>) -> PyEquityFutureSpecs {
        PyEquityFutureSpecs {
            inner: EquityFutureSpecs::euro_stoxx_50(),
        }
    }

    /// Create DAX future contract specifications.
    ///
    /// Returns
    /// -------
    /// EquityFutureSpecs
    ///     Eurex DAX specifications (multiplier=25, tick_size=0.5)
    #[classmethod]
    fn dax_specs(_cls: &Bound<'_, PyType>) -> PyEquityFutureSpecs {
        PyEquityFutureSpecs {
            inner: EquityFutureSpecs::dax(),
        }
    }

    /// Create FTSE 100 future contract specifications.
    ///
    /// Returns
    /// -------
    /// EquityFutureSpecs
    ///     ICE FTSE 100 specifications (multiplier=10, tick_size=0.5)
    #[classmethod]
    fn ftse_100_specs(_cls: &Bound<'_, PyType>) -> PyEquityFutureSpecs {
        PyEquityFutureSpecs {
            inner: EquityFutureSpecs::ftse_100(),
        }
    }

    /// Create Nikkei 225 future contract specifications.
    ///
    /// Returns
    /// -------
    /// EquityFutureSpecs
    ///     CME/OSE Nikkei 225 specifications (multiplier=500, tick_size=5.0)
    #[classmethod]
    fn nikkei_225_specs(_cls: &Bound<'_, PyType>) -> PyEquityFutureSpecs {
        PyEquityFutureSpecs {
            inner: EquityFutureSpecs::nikkei_225(),
        }
    }

    /// Instrument identifier.
    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    /// Instrument type.
    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::EquityIndexFuture)
    }

    /// Index ticker symbol (e.g., "SPX", "NDX").
    #[getter]
    fn index_ticker(&self) -> String {
        self.inner.index_ticker.clone()
    }

    /// Settlement currency.
    #[getter]
    fn currency(&self) -> PyCurrency {
        PyCurrency::new(self.inner.currency)
    }

    /// Number of contracts.
    #[getter]
    fn quantity(&self) -> f64 {
        self.inner.quantity
    }

    /// Expiry/settlement date.
    #[getter]
    fn expiry_date<'py>(&self, py: Python<'py>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.expiry_date)
    }

    /// Last trading date.
    #[getter]
    fn last_trading_date<'py>(&self, py: Python<'py>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.last_trading_date)
    }

    /// Entry price (if set).
    #[getter]
    fn entry_price(&self) -> Option<f64> {
        self.inner.entry_price
    }

    /// Quoted market price (if set).
    #[getter]
    fn quoted_price(&self) -> Option<f64> {
        self.inner.quoted_price
    }

    /// Position side (Long or Short).
    #[getter]
    fn position(&self) -> PyPosition {
        PyPosition::new(self.inner.position)
    }

    /// Contract specifications.
    #[getter]
    fn contract_specs(&self) -> PyEquityFutureSpecs {
        PyEquityFutureSpecs {
            inner: self.inner.contract_specs.clone(),
        }
    }

    /// Calculate the notional value of the position at a given price.
    ///
    /// Parameters
    /// ----------
    /// price : float
    ///     Index price
    ///
    /// Returns
    /// -------
    /// float
    ///     Notional value = price × multiplier × quantity
    fn notional_value(&self, price: f64) -> f64 {
        self.inner.notional_value(price)
    }

    /// Calculate delta exposure (index point sensitivity).
    ///
    /// Returns
    /// -------
    /// float
    ///     Delta = multiplier × quantity × position_sign
    ///
    /// This represents the currency P&L change for a 1-point move in the index.
    fn delta(&self) -> f64 {
        self.inner.delta()
    }

    fn __repr__(&self) -> String {
        format!(
            "EquityIndexFuture(id='{}', index='{}', quantity={}, position={}, expiry={})",
            self.inner.id,
            self.inner.index_ticker,
            self.inner.quantity,
            match self.inner.position {
                Position::Long => "long",
                Position::Short => "short",
                _ => unreachable!("unknown Position variant"),
            },
            self.inner.expiry_date
        )
    }
}

impl fmt::Display for PyEquityIndexFuture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "EquityIndexFuture({}, {}, qty={})",
            self.inner.id, self.inner.index_ticker, self.inner.quantity
        )
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyPosition>()?;
    module.add_class::<PyEquityFutureSpecs>()?;
    module.add_class::<PyEquityIndexFuture>()?;
    module.add_class::<PyEquityIndexFutureBuilder>()?;
    Ok(vec![
        "FuturePosition",
        "EquityFutureSpecs",
        "EquityIndexFuture",
        "EquityIndexFutureBuilder",
    ])
}
