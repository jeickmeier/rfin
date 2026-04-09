use crate::core::currency::PyCurrency;
use crate::core::dates::daycount::PyDayCount;
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::core::market_data::PyMarketContext;
use crate::core::money::{extract_money, PyMoney};
use crate::errors::core_to_py;
use crate::valuations::common::PyInstrumentType;
use finstack_core::currency::Currency;
use finstack_core::dates::{DayCount, Tenor};
use finstack_core::math::stats::RealizedVarMethod;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::fx::fx_variance_swap::{FxVarianceSwap, PayReceive};
use finstack_valuations::instruments::Attributes;
use finstack_valuations::prelude::Instrument;
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyList, PyModule, PyType};
use pyo3::{Bound, FromPyObject, Py, PyRef, PyRefMut};
use std::fmt;
use std::str::FromStr;
use std::sync::Arc;

/// Variance direction (pay or receive variance).
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "FxVarianceDirection",
    frozen,
    from_py_object
)]
#[derive(Clone, Copy, Debug)]
pub struct PyFxPayReceive {
    pub(crate) inner: PayReceive,
}

impl PyFxPayReceive {
    const fn new(inner: PayReceive) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyFxPayReceive {
    #[classattr]
    const PAY: Self = Self::new(PayReceive::Pay);
    #[classattr]
    const RECEIVE: Self = Self::new(PayReceive::Receive);

    fn __repr__(&self) -> &'static str {
        match self.inner {
            PayReceive::Pay => "FxVarianceDirection.PAY",
            PayReceive::Receive => "FxVarianceDirection.RECEIVE",
        }
    }

    fn __str__(&self) -> &'static str {
        match self.inner {
            PayReceive::Pay => "pay",
            PayReceive::Receive => "receive",
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct FxPayReceiveArg(PyFxPayReceive);

impl<'a, 'py> FromPyObject<'a, 'py> for FxPayReceiveArg {
    type Error = PyErr;
    fn extract(obj: pyo3::Borrowed<'a, 'py, PyAny>) -> PyResult<Self> {
        if let Ok(existing) = obj.extract::<PyRef<'py, PyFxPayReceive>>() {
            return Ok(FxPayReceiveArg(*existing));
        }

        if let Ok(label) = obj.extract::<&str>() {
            let direction = PayReceive::from_str(label)
                .map_err(|e| PyValueError::new_err(format!("Unknown variance direction: {e}")))?;
            return Ok(FxPayReceiveArg(PyFxPayReceive::new(direction)));
        }

        Err(PyTypeError::new_err(
            "Expected FxVarianceDirection or string identifier",
        ))
    }
}

/// Realized variance calculation method.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "FxRealizedVarianceMethod",
    frozen,
    from_py_object
)]
#[derive(Clone, Copy, Debug)]
pub struct PyFxRealizedVarMethod {
    pub(crate) inner: RealizedVarMethod,
}

impl PyFxRealizedVarMethod {
    const fn new(inner: RealizedVarMethod) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyFxRealizedVarMethod {
    #[classattr]
    const CLOSE_TO_CLOSE: Self = Self::new(RealizedVarMethod::CloseToClose);
    #[classattr]
    const PARKINSON: Self = Self::new(RealizedVarMethod::Parkinson);
    #[classattr]
    const GARMAN_KLASS: Self = Self::new(RealizedVarMethod::GarmanKlass);
    #[classattr]
    const ROGERS_SATCHELL: Self = Self::new(RealizedVarMethod::RogersSatchell);
    #[classattr]
    const YANG_ZHANG: Self = Self::new(RealizedVarMethod::YangZhang);

    fn __repr__(&self) -> &'static str {
        match self.inner {
            RealizedVarMethod::CloseToClose => "FxRealizedVarianceMethod.CLOSE_TO_CLOSE",
            RealizedVarMethod::Parkinson => "FxRealizedVarianceMethod.PARKINSON",
            RealizedVarMethod::GarmanKlass => "FxRealizedVarianceMethod.GARMAN_KLASS",
            RealizedVarMethod::RogersSatchell => "FxRealizedVarianceMethod.ROGERS_SATCHELL",
            RealizedVarMethod::YangZhang => "FxRealizedVarianceMethod.YANG_ZHANG",
        }
    }

    fn __str__(&self) -> &'static str {
        self.__repr__()
    }
}

#[derive(Clone, Copy, Debug)]
struct FxRealizedVarMethodArg(PyFxRealizedVarMethod);

impl<'a, 'py> FromPyObject<'a, 'py> for FxRealizedVarMethodArg {
    type Error = PyErr;
    fn extract(obj: pyo3::Borrowed<'a, 'py, PyAny>) -> PyResult<Self> {
        if let Ok(existing) = obj.extract::<PyRef<'py, PyFxRealizedVarMethod>>() {
            return Ok(FxRealizedVarMethodArg(*existing));
        }

        if let Ok(label) = obj.extract::<&str>() {
            let method = RealizedVarMethod::from_str(label).map_err(|e| {
                PyValueError::new_err(format!("Unknown realized variance method: {e}"))
            })?;
            return Ok(FxRealizedVarMethodArg(PyFxRealizedVarMethod::new(method)));
        }

        Err(PyTypeError::new_err(
            "Expected FxRealizedVarianceMethod or string identifier",
        ))
    }
}

/// FX variance swap instrument.
///
/// A variance swap on an FX rate pair. The payoff is based on the realized variance
/// of FX rate returns over the observation period.
///
/// Payoff Formula
/// --------------
/// Payoff = Notional × (Realized Variance - Strike Variance)
///
/// Pricing
/// -------
/// Before maturity, the contract is valued by combining:
/// - Partial realized variance from observed FX rates
/// - Implied forward variance from volatility surface
/// - Discounting to present value
///
/// Examples
/// --------
/// Create a 1-year EUR/USD variance swap::
///
///     from finstack import Money, Date
///     from finstack.valuations.instruments import FxVarianceSwap
///
///     var_swap = (
///         FxVarianceSwap.builder("FXVAR-EURUSD-1Y")
///         .base_currency("EUR")
///         .quote_currency("USD")
///         .notional(Money.from_code(1_000_000, "USD"))
///         .strike_variance(0.04)
///         .start_date(Date(2024, 1, 2))
///         .maturity(Date(2025, 1, 2))
///         .observation_freq("daily")
///         .realized_method("close_to_close")
///         .side("receive")
///         .domestic_discount_curve("USD-OIS")
///         .foreign_discount_curve("EUR-OIS")
///         .vol_surface("EURUSD-VOL")
///         .build()
///     )
///
/// See Also
/// --------
/// VarianceSwap : Equity variance swap
/// FxOption : Vanilla FX option
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "FxVarianceSwap",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyFxVarianceSwap {
    pub(crate) inner: Arc<FxVarianceSwap>,
}

impl PyFxVarianceSwap {
    pub(crate) fn new(inner: FxVarianceSwap) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

#[pyclass(
    module = "finstack.valuations.instruments",
    name = "FxVarianceSwapBuilder"
)]
pub struct PyFxVarianceSwapBuilder {
    instrument_id: InstrumentId,
    base_currency: Option<Currency>,
    quote_currency: Option<Currency>,
    spot_id: Option<String>,
    notional: Option<Money>,
    strike_variance: Option<f64>,
    start_date: Option<time::Date>,
    maturity: Option<time::Date>,
    observation_freq: Option<Tenor>,
    realized_var_method: RealizedVarMethod,
    open_series_id: Option<String>,
    high_series_id: Option<String>,
    low_series_id: Option<String>,
    close_series_id: Option<String>,
    side: PayReceive,
    domestic_discount_curve_id: Option<CurveId>,
    foreign_discount_curve_id: Option<CurveId>,
    vol_surface_id: Option<CurveId>,
    day_count: DayCount,
}

impl PyFxVarianceSwapBuilder {
    fn new_with_id(id: InstrumentId) -> Self {
        Self {
            instrument_id: id,
            base_currency: None,
            quote_currency: None,
            spot_id: None,
            notional: None,
            strike_variance: None,
            start_date: None,
            maturity: None,
            observation_freq: None,
            realized_var_method: RealizedVarMethod::CloseToClose,
            open_series_id: None,
            high_series_id: None,
            low_series_id: None,
            close_series_id: None,
            side: PayReceive::Receive,
            domestic_discount_curve_id: None,
            foreign_discount_curve_id: None,
            vol_surface_id: None,
            day_count: DayCount::Act365F,
        }
    }

    fn validate_and_build(&self) -> PyResult<FxVarianceSwap> {
        let base_currency = self
            .base_currency
            .ok_or_else(|| PyValueError::new_err("base_currency is required"))?;

        let quote_currency = self
            .quote_currency
            .ok_or_else(|| PyValueError::new_err("quote_currency is required"))?;

        let notional = self
            .notional
            .ok_or_else(|| PyValueError::new_err("notional is required"))?;

        let strike_variance = self
            .strike_variance
            .ok_or_else(|| PyValueError::new_err("strike_variance is required"))?;

        if strike_variance < 0.0 {
            return Err(PyValueError::new_err(
                "strike_variance must be non-negative",
            ));
        }

        let start_date = self
            .start_date
            .ok_or_else(|| PyValueError::new_err("start_date is required"))?;

        let maturity = self
            .maturity
            .ok_or_else(|| PyValueError::new_err("maturity is required"))?;

        if maturity <= start_date {
            return Err(PyValueError::new_err("maturity must be after start_date"));
        }

        let observation_freq = self
            .observation_freq
            .ok_or_else(|| PyValueError::new_err("observation_freq is required"))?;

        let domestic_discount_curve_id = self
            .domestic_discount_curve_id
            .clone()
            .ok_or_else(|| PyValueError::new_err("domestic_discount_curve_id is required"))?;

        let foreign_discount_curve_id = self
            .foreign_discount_curve_id
            .clone()
            .ok_or_else(|| PyValueError::new_err("foreign_discount_curve_id is required"))?;

        let vol_surface_id = self
            .vol_surface_id
            .clone()
            .ok_or_else(|| PyValueError::new_err("vol_surface_id is required"))?;

        Ok(FxVarianceSwap {
            id: self.instrument_id.clone(),
            base_currency,
            quote_currency,
            spot_id: self.spot_id.clone(),
            notional,
            strike_variance,
            start_date,
            maturity,
            observation_freq,
            realized_var_method: self.realized_var_method,
            open_series_id: self.open_series_id.clone(),
            high_series_id: self.high_series_id.clone(),
            low_series_id: self.low_series_id.clone(),
            close_series_id: self.close_series_id.clone(),
            side: self.side,
            domestic_discount_curve_id,
            foreign_discount_curve_id,
            vol_surface_id,
            day_count: self.day_count,
            pricing_overrides: finstack_valuations::instruments::PricingOverrides::default(),
            attributes: Attributes::new(),
        })
    }
}

#[pymethods]
impl PyFxVarianceSwapBuilder {
    fn base_currency<'py>(
        mut slf: PyRefMut<'py, Self>,
        ccy: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        use crate::core::currency::extract_currency;
        slf.base_currency = Some(extract_currency(&ccy)?);
        Ok(slf)
    }

    fn quote_currency<'py>(
        mut slf: PyRefMut<'py, Self>,
        ccy: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        use crate::core::currency::extract_currency;
        slf.quote_currency = Some(extract_currency(&ccy)?);
        Ok(slf)
    }

    fn spot_id<'py>(mut slf: PyRefMut<'py, Self>, id: &str) -> PyRefMut<'py, Self> {
        slf.spot_id = Some(id.to_string());
        slf
    }

    fn notional<'py>(
        mut slf: PyRefMut<'py, Self>,
        notional: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.notional = Some(extract_money(&notional)?);
        Ok(slf)
    }

    fn strike_variance<'py>(mut slf: PyRefMut<'py, Self>, variance: f64) -> PyRefMut<'py, Self> {
        slf.strike_variance = Some(variance);
        slf
    }

    fn start_date<'py>(
        mut slf: PyRefMut<'py, Self>,
        date: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.start_date = Some(py_to_date(&date)?);
        Ok(slf)
    }

    fn maturity<'py>(
        mut slf: PyRefMut<'py, Self>,
        date: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.maturity = Some(py_to_date(&date)?);
        Ok(slf)
    }

    fn observation_freq<'py>(
        mut slf: PyRefMut<'py, Self>,
        freq: &str,
    ) -> PyResult<PyRefMut<'py, Self>> {
        // Frequency labels map to Tenor constructors; Tenor::from_str
        // expects "1D"/"3M" style strings, not "daily"/"quarterly".
        let tenor = match freq.to_ascii_lowercase().as_str() {
            "daily" => Tenor::daily(),
            "weekly" => Tenor::weekly(),
            "monthly" => Tenor::monthly(),
            "quarterly" => Tenor::quarterly(),
            other => {
                return Err(PyValueError::new_err(format!(
                    "Unknown observation frequency: {other}. Expected 'daily', 'weekly', 'monthly', or 'quarterly'"
                )))
            }
        };
        slf.observation_freq = Some(tenor);
        Ok(slf)
    }

    fn realized_method<'py>(
        mut slf: PyRefMut<'py, Self>,
        method: FxRealizedVarMethodArg,
    ) -> PyRefMut<'py, Self> {
        slf.realized_var_method = method.0.inner;
        slf
    }

    fn open_series_id<'py>(mut slf: PyRefMut<'py, Self>, series_id: String) -> PyRefMut<'py, Self> {
        slf.open_series_id = Some(series_id);
        slf
    }

    fn high_series_id<'py>(mut slf: PyRefMut<'py, Self>, series_id: String) -> PyRefMut<'py, Self> {
        slf.high_series_id = Some(series_id);
        slf
    }

    fn low_series_id<'py>(mut slf: PyRefMut<'py, Self>, series_id: String) -> PyRefMut<'py, Self> {
        slf.low_series_id = Some(series_id);
        slf
    }

    fn close_series_id<'py>(
        mut slf: PyRefMut<'py, Self>,
        series_id: String,
    ) -> PyRefMut<'py, Self> {
        slf.close_series_id = Some(series_id);
        slf
    }

    fn side<'py>(mut slf: PyRefMut<'py, Self>, direction: FxPayReceiveArg) -> PyRefMut<'py, Self> {
        slf.side = direction.0.inner;
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

    fn vol_surface<'py>(mut slf: PyRefMut<'py, Self>, surface_id: &str) -> PyRefMut<'py, Self> {
        slf.vol_surface_id = Some(CurveId::new(surface_id));
        slf
    }

    fn day_count<'py>(mut slf: PyRefMut<'py, Self>, dc: &PyDayCount) -> PyRefMut<'py, Self> {
        slf.day_count = dc.inner;
        slf
    }

    fn build(slf: PyRef<'_, Self>) -> PyResult<PyFxVarianceSwap> {
        let inner = slf.validate_and_build()?;
        Ok(PyFxVarianceSwap::new(inner))
    }

    fn __repr__(&self) -> String {
        format!("FxVarianceSwapBuilder(id='{}')", self.instrument_id)
    }
}

#[pymethods]
impl PyFxVarianceSwap {
    /// Create a builder for an FX variance swap.
    ///
    /// Parameters
    /// ----------
    /// instrument_id : str
    ///     Unique instrument identifier (e.g., "FXVAR-EURUSD-1Y")
    ///
    /// Returns
    /// -------
    /// FxVarianceSwapBuilder
    ///     Builder instance for fluent configuration
    #[classmethod]
    fn builder(_cls: &Bound<'_, PyType>, instrument_id: &str) -> PyFxVarianceSwapBuilder {
        PyFxVarianceSwapBuilder::new_with_id(InstrumentId::new(instrument_id))
    }

    /// Instrument identifier.
    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    /// Instrument type.
    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::FxVarianceSwap)
    }

    /// Base currency (foreign).
    #[getter]
    fn base_currency(&self) -> PyCurrency {
        PyCurrency::new(self.inner.base_currency)
    }

    /// Quote currency (domestic).
    #[getter]
    fn quote_currency(&self) -> PyCurrency {
        PyCurrency::new(self.inner.quote_currency)
    }

    /// Variance notional.
    #[getter]
    fn notional(&self) -> PyMoney {
        PyMoney::new(self.inner.notional)
    }

    /// Strike variance (annualized).
    #[getter]
    fn strike_variance(&self) -> f64 {
        self.inner.strike_variance
    }

    /// Start date of observation period.
    #[getter]
    fn start_date<'py>(&self, py: Python<'py>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.start_date)
    }

    /// Maturity/settlement date.
    #[getter]
    fn maturity<'py>(&self, py: Python<'py>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.maturity)
    }

    /// Variance direction (pay or receive).
    #[getter]
    fn side(&self) -> PyFxPayReceive {
        PyFxPayReceive::new(self.inner.side)
    }

    /// Observation frequency label (e.g., "daily", "weekly").
    #[getter]
    fn observation_freq(&self) -> String {
        if let Some(days) = self.inner.observation_freq.days() {
            match days {
                1 => "daily".to_string(),
                7 => "weekly".to_string(),
                14 => "biweekly".to_string(),
                other => format!("{}d", other),
            }
        } else if let Some(months) = self.inner.observation_freq.months() {
            match months {
                1 => "monthly".to_string(),
                3 => "quarterly".to_string(),
                6 => "semiannual".to_string(),
                12 => "annual".to_string(),
                other => format!("{}m", other),
            }
        } else {
            "daily".to_string()
        }
    }

    /// Realized variance calculation method.
    #[getter]
    fn realized_var_method(&self) -> PyFxRealizedVarMethod {
        PyFxRealizedVarMethod {
            inner: self.inner.realized_var_method,
        }
    }

    /// Spot identifier used for historical series lookup.
    #[getter]
    fn spot_id(&self) -> Option<String> {
        self.inner.spot_id.clone()
    }

    /// Open price series identifier (required for OHLC-based estimators).
    #[getter]
    fn open_series_id(&self) -> Option<String> {
        self.inner.open_series_id.clone()
    }

    /// High price series identifier (required for OHLC-based estimators).
    #[getter]
    fn high_series_id(&self) -> Option<String> {
        self.inner.high_series_id.clone()
    }

    /// Low price series identifier (required for OHLC-based estimators).
    #[getter]
    fn low_series_id(&self) -> Option<String> {
        self.inner.low_series_id.clone()
    }

    /// Close price series identifier. Defaults to spot_id when not set.
    #[getter]
    fn close_series_id(&self) -> Option<String> {
        self.inner.close_series_id.clone()
    }

    /// Domestic discount curve identifier.
    #[getter]
    fn domestic_discount_curve(&self) -> String {
        self.inner.domestic_discount_curve_id.as_str().to_string()
    }

    /// Foreign discount curve identifier.
    #[getter]
    fn foreign_discount_curve(&self) -> String {
        self.inner.foreign_discount_curve_id.as_str().to_string()
    }

    /// Volatility surface identifier.
    #[getter]
    fn vol_surface(&self) -> String {
        self.inner.vol_surface_id.as_str().to_string()
    }

    /// Day count convention label.
    #[getter]
    fn day_count(&self) -> PyDayCount {
        PyDayCount::new(self.inner.day_count)
    }

    /// Calculate present value of the FX variance swap.
    ///
    /// Parameters
    /// ----------
    /// market : MarketContext
    ///     Market data including discount curves, FX rates, and volatility surface
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
        let date = py_to_date(&as_of)?;
        let value = py
            .detach(|| self.inner.value(&market.inner, date))
            .map_err(core_to_py)?;
        Ok(PyMoney::new(value))
    }

    /// Calculate payoff given realized variance.
    ///
    /// Parameters
    /// ----------
    /// realized_variance : float
    ///     Realized variance (annualized)
    ///
    /// Returns
    /// -------
    /// Money
    ///     Payoff = notional × (realized_variance - strike_variance) × side_sign
    fn payoff(&self, realized_variance: f64) -> PyMoney {
        PyMoney::new(self.inner.payoff(realized_variance))
    }

    /// Get observation dates based on frequency.
    ///
    /// Returns
    /// -------
    /// list[Date]
    ///     List of observation dates from start to maturity
    fn observation_dates(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let dates = self.inner.observation_dates();
        let py_dates: PyResult<Vec<Py<PyAny>>> =
            dates.into_iter().map(|d| date_to_py(py, d)).collect();
        Ok(PyList::new(py, py_dates?)?.into())
    }

    /// Calculate annualization factor based on observation frequency.
    ///
    /// Returns
    /// -------
    /// float
    ///     Annualization factor (e.g., 252 for daily, 12 for monthly)
    fn annualization_factor(&self) -> f64 {
        self.inner.annualization_factor()
    }

    /// Calculate partial realized variance for the elapsed observation period.
    ///
    /// Parameters
    /// ----------
    /// market : MarketContext
    ///     Market data including historical price series
    /// as_of : Date
    ///     Valuation date
    ///
    /// Returns
    /// -------
    /// float
    ///     Annualized realized variance for the elapsed period
    fn partial_realized_variance(
        &self,
        py: Python<'_>,
        market: &PyMarketContext,
        as_of: Bound<'_, PyAny>,
    ) -> PyResult<f64> {
        let date = py_to_date(&as_of)?;
        py.detach(|| self.inner.partial_realized_variance(&market.inner, date))
            .map_err(core_to_py)
    }

    /// Calculate implied forward variance for the remaining observation period.
    ///
    /// Parameters
    /// ----------
    /// market : MarketContext
    ///     Market data including volatility surface and discount curves
    /// as_of : Date
    ///     Valuation date
    ///
    /// Returns
    /// -------
    /// float
    ///     Annualized implied forward variance for the remaining period
    fn remaining_forward_variance(
        &self,
        py: Python<'_>,
        market: &PyMarketContext,
        as_of: Bound<'_, PyAny>,
    ) -> PyResult<f64> {
        let date = py_to_date(&as_of)?;
        py.detach(|| self.inner.remaining_forward_variance(&market.inner, date))
            .map_err(core_to_py)
    }

    fn __repr__(&self) -> String {
        format!(
            "FxVarianceSwap(id='{}', {}/{}, strike_var={}, side={}, maturity={})",
            self.inner.id,
            self.inner.base_currency,
            self.inner.quote_currency,
            self.inner.strike_variance,
            match self.inner.side {
                PayReceive::Pay => "pay",
                PayReceive::Receive => "receive",
            },
            self.inner.maturity
        )
    }
}

impl fmt::Display for PyFxVarianceSwap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "FxVarianceSwap({}, {}/{})",
            self.inner.id, self.inner.base_currency, self.inner.quote_currency
        )
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyFxPayReceive>()?;
    module.add_class::<PyFxRealizedVarMethod>()?;
    module.add_class::<PyFxVarianceSwap>()?;
    module.add_class::<PyFxVarianceSwapBuilder>()?;
    Ok(vec![
        "FxVarianceDirection",
        "FxRealizedVarianceMethod",
        "FxVarianceSwap",
        "FxVarianceSwapBuilder",
    ])
}
