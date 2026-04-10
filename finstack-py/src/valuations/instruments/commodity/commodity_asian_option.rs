use super::common::{
    default_attributes, default_pricing_overrides, ensure_non_empty, ensure_positive,
    required_value, validated_clone,
};
use crate::core::common::args::{parse_day_count, CurrencyArg};
use crate::core::currency::PyCurrency;
use crate::core::dates::daycount::PyDayCount;
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::errors::PyContext;
use crate::valuations::common::PyInstrumentType;
use finstack_core::dates::DayCount;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::commodity::commodity_asian_option::CommodityAsianOption;
use finstack_valuations::instruments::exotics::asian_option::AveragingMethod;
use finstack_valuations::instruments::CommodityUnderlyingParams;
use finstack_valuations::instruments::OptionType;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyList, PyModule, PyTuple, PyType};
use pyo3::{Bound, Py, PyRefMut};
use std::fmt;
use std::str::FromStr;
use std::sync::Arc;

/// Commodity Asian option: option on the arithmetic or geometric average of
/// commodity prices.
///
/// This is the dominant option type in commodity markets. The average is
/// typically computed over commodity forward/futures prices for specific
/// delivery periods.
///
/// Examples:
///     >>> option = (
///     ...     CommodityAsianOption.builder("WTI-ASIAN-6M")
///     ...     .commodity_type("Energy")
///     ...     .ticker("CL")
///     ...     .unit("BBL")
///     ...     .currency("USD")
///     ...     .strike(75.0)
///     ...     .option_type("call")
///     ...     .averaging_method("arithmetic")
///     ...     .fixing_dates([date(2025, 1, 31), date(2025, 2, 28)])
///     ...     .quantity(1000.0)
///     ...     .expiry(date(2025, 7, 2))
///     ...     .forward_curve_id("CL-FORWARD")
///     ...     .discount_curve_id("USD-OIS")
///     ...     .vol_surface_id("CL-VOL")
///     ...     .build()
///     ... )
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "CommodityAsianOption",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyCommodityAsianOption {
    pub(crate) inner: Arc<CommodityAsianOption>,
}

impl PyCommodityAsianOption {
    pub(crate) fn new(inner: CommodityAsianOption) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

#[pyclass(
    module = "finstack.valuations.instruments",
    name = "CommodityAsianOptionBuilder"
)]
pub struct PyCommodityAsianOptionBuilder {
    instrument_id: InstrumentId,
    commodity_type: Option<String>,
    ticker: Option<String>,
    unit: Option<String>,
    currency: Option<finstack_core::currency::Currency>,
    strike: Option<f64>,
    option_type: Option<OptionType>,
    averaging_method: Option<AveragingMethod>,
    fixing_dates: Option<Vec<time::Date>>,
    realized_fixings: Vec<(time::Date, f64)>,
    quantity: Option<f64>,
    expiry: Option<time::Date>,
    forward_curve_id: Option<CurveId>,
    discount_curve_id: Option<CurveId>,
    vol_surface_id: Option<CurveId>,
    day_count: DayCount,
}

impl PyCommodityAsianOptionBuilder {
    fn new_with_id(id: InstrumentId) -> Self {
        Self {
            instrument_id: id,
            commodity_type: None,
            ticker: None,
            unit: None,
            currency: None,
            strike: None,
            option_type: None,
            averaging_method: None,
            fixing_dates: None,
            realized_fixings: Vec::new(),
            quantity: None,
            expiry: None,
            forward_curve_id: None,
            discount_curve_id: None,
            vol_surface_id: None,
            day_count: DayCount::Act365F,
        }
    }

    fn validate_and_build(&self) -> PyResult<CommodityAsianOption> {
        let commodity_type = validated_clone(
            "CommodityAsianOptionBuilder",
            "commodity_type",
            self.commodity_type.as_ref(),
        )?;
        let ticker = validated_clone(
            "CommodityAsianOptionBuilder",
            "ticker",
            self.ticker.as_ref(),
        )?;
        let unit = validated_clone("CommodityAsianOptionBuilder", "unit", self.unit.as_ref())?;
        let currency = required_value(self.currency, "currency is required")?;
        let strike = required_value(self.strike, "strike is required")?;
        let option_type = required_value(self.option_type, "option_type is required")?;
        let averaging_method =
            required_value(self.averaging_method, "averaging_method is required")?;
        let fixing_dates = validated_clone(
            "CommodityAsianOptionBuilder",
            "fixing_dates",
            self.fixing_dates.as_ref(),
        )?;
        let quantity = ensure_positive(
            required_value(self.quantity, "quantity is required")?,
            "quantity must be positive",
        )?;
        let expiry = required_value(self.expiry, "expiry is required")?;
        let forward_curve_id = validated_clone(
            "CommodityAsianOptionBuilder",
            "forward_curve_id",
            self.forward_curve_id.as_ref(),
        )?;
        let discount_curve_id = validated_clone(
            "CommodityAsianOptionBuilder",
            "discount_curve_id",
            self.discount_curve_id.as_ref(),
        )?;
        let vol_surface_id = validated_clone(
            "CommodityAsianOptionBuilder",
            "vol_surface_id",
            self.vol_surface_id.as_ref(),
        )?;

        ensure_non_empty(&fixing_dates, "fixing_dates must not be empty")?;

        let mut builder = CommodityAsianOption::builder()
            .id(self.instrument_id.clone())
            .underlying(CommodityUnderlyingParams::new(
                commodity_type,
                ticker,
                unit,
                currency,
            ))
            .strike(strike)
            .option_type(option_type)
            .averaging_method(averaging_method)
            .fixing_dates(fixing_dates)
            .quantity(quantity)
            .expiry(expiry)
            .forward_curve_id(forward_curve_id)
            .discount_curve_id(discount_curve_id)
            .vol_surface_id(vol_surface_id)
            .day_count(self.day_count)
            .pricing_overrides(default_pricing_overrides())
            .attributes(default_attributes());

        if !self.realized_fixings.is_empty() {
            builder = builder.realized_fixings(self.realized_fixings.clone());
        }

        builder.build().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!(
                "Failed to build CommodityAsianOption: {e}"
            ))
        })
    }
}

#[pymethods]
impl PyCommodityAsianOptionBuilder {
    fn commodity_type(mut slf: PyRefMut<'_, Self>, commodity_type: String) -> PyRefMut<'_, Self> {
        slf.commodity_type = Some(commodity_type);
        slf
    }

    fn ticker(mut slf: PyRefMut<'_, Self>, ticker: String) -> PyRefMut<'_, Self> {
        slf.ticker = Some(ticker);
        slf
    }

    fn unit(mut slf: PyRefMut<'_, Self>, unit: String) -> PyRefMut<'_, Self> {
        slf.unit = Some(unit);
        slf
    }

    fn currency<'py>(
        mut slf: PyRefMut<'py, Self>,
        currency: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let CurrencyArg(ccy) = currency.extract().context("currency")?;
        slf.currency = Some(ccy);
        Ok(slf)
    }

    fn strike(mut slf: PyRefMut<'_, Self>, strike: f64) -> PyRefMut<'_, Self> {
        slf.strike = Some(strike);
        slf
    }

    fn option_type<'py>(
        mut slf: PyRefMut<'py, Self>,
        option_type: &str,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.option_type = Some(
            OptionType::from_str(option_type).map_err(|e| PyValueError::new_err(e.to_string()))?,
        );
        Ok(slf)
    }

    fn averaging_method<'py>(
        mut slf: PyRefMut<'py, Self>,
        method: &str,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.averaging_method = Some(
            AveragingMethod::from_str(method).map_err(|e| PyValueError::new_err(e.to_string()))?,
        );
        Ok(slf)
    }

    fn fixing_dates<'py>(
        mut slf: PyRefMut<'py, Self>,
        dates: Bound<'py, PyList>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let mut fixing_dates_vec = Vec::new();
        for item in dates.iter() {
            fixing_dates_vec.push(py_to_date(&item).context("fixing_dates")?);
        }
        slf.fixing_dates = Some(fixing_dates_vec);
        Ok(slf)
    }

    fn realized_fixings<'py>(
        mut slf: PyRefMut<'py, Self>,
        fixings: Bound<'py, PyList>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let mut realized = Vec::new();
        for item in fixings.iter() {
            let tuple: (Py<PyAny>, f64) = item.extract()?;
            let date = py_to_date(tuple.0.bind(fixings.py())).context("realized_fixings date")?;
            realized.push((date, tuple.1));
        }
        slf.realized_fixings = realized;
        Ok(slf)
    }

    fn quantity(mut slf: PyRefMut<'_, Self>, quantity: f64) -> PyRefMut<'_, Self> {
        slf.quantity = Some(quantity);
        slf
    }

    fn expiry<'py>(
        mut slf: PyRefMut<'py, Self>,
        expiry: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.expiry = Some(py_to_date(&expiry).context("expiry")?);
        Ok(slf)
    }

    fn forward_curve_id(mut slf: PyRefMut<'_, Self>, curve_id: String) -> PyRefMut<'_, Self> {
        slf.forward_curve_id = Some(CurveId::new(curve_id.as_str()));
        slf
    }

    fn discount_curve_id(mut slf: PyRefMut<'_, Self>, curve_id: String) -> PyRefMut<'_, Self> {
        slf.discount_curve_id = Some(CurveId::new(curve_id.as_str()));
        slf
    }

    fn vol_surface_id(mut slf: PyRefMut<'_, Self>, surface_id: String) -> PyRefMut<'_, Self> {
        slf.vol_surface_id = Some(CurveId::new(surface_id.as_str()));
        slf
    }

    fn day_count<'py>(
        mut slf: PyRefMut<'py, Self>,
        day_count: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.day_count = parse_day_count(&day_count)?;
        Ok(slf)
    }

    fn build(slf: PyRefMut<'_, Self>) -> PyResult<PyCommodityAsianOption> {
        let inner = slf.validate_and_build()?;
        Ok(PyCommodityAsianOption::new(inner))
    }

    fn __repr__(&self) -> String {
        format!("CommodityAsianOptionBuilder(id='{}')", self.instrument_id)
    }
}

#[pymethods]
impl PyCommodityAsianOption {
    /// Start a fluent builder for a commodity Asian option.
    ///
    /// Parameters
    /// ----------
    /// instrument_id : str
    ///     Unique instrument identifier (e.g., "WTI-ASIAN-6M")
    ///
    /// Returns
    /// -------
    /// CommodityAsianOptionBuilder
    ///     Builder instance for fluent configuration
    #[classmethod]
    fn builder<'py>(
        cls: &Bound<'py, PyType>,
        instrument_id: &str,
    ) -> PyResult<Py<PyCommodityAsianOptionBuilder>> {
        let py = cls.py();
        let builder = PyCommodityAsianOptionBuilder::new_with_id(InstrumentId::new(instrument_id));
        Py::new(py, builder)
    }

    /// Instrument identifier.
    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    /// Instrument type key.
    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::CommodityAsianOption)
    }

    /// Commodity type (e.g., "Energy", "Metal").
    #[getter]
    fn commodity_type(&self) -> &str {
        &self.inner.underlying.commodity_type
    }

    /// Ticker symbol.
    #[getter]
    fn ticker(&self) -> &str {
        &self.inner.underlying.ticker
    }

    /// Unit of measurement.
    #[getter]
    fn unit(&self) -> &str {
        &self.inner.underlying.unit
    }

    /// Currency.
    #[getter]
    fn currency(&self) -> PyCurrency {
        PyCurrency::new(self.inner.underlying.currency)
    }

    /// Strike price per unit.
    #[getter]
    fn strike(&self) -> f64 {
        self.inner.strike
    }

    /// Option type label ("call" or "put").
    #[getter]
    fn option_type(&self) -> &'static str {
        match self.inner.option_type {
            OptionType::Call => "call",
            OptionType::Put => "put",
        }
    }

    /// Averaging method label ("arithmetic" or "geometric").
    #[getter]
    fn averaging_method(&self) -> &'static str {
        match self.inner.averaging_method {
            AveragingMethod::Arithmetic => "arithmetic",
            AveragingMethod::Geometric => "geometric",
        }
    }

    /// List of fixing dates for averaging.
    #[getter]
    fn fixing_dates(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let dates = PyList::empty(py);
        for d in &self.inner.fixing_dates {
            dates.append(date_to_py(py, *d)?)?;
        }
        Ok(dates.into())
    }

    /// Contract quantity in commodity units.
    #[getter]
    fn quantity(&self) -> f64 {
        self.inner.quantity
    }

    /// Option expiry/settlement date.
    #[getter]
    fn expiry(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.expiry)
    }

    /// Forward/futures price curve ID.
    #[getter]
    fn forward_curve_id(&self) -> &str {
        self.inner.forward_curve_id.as_str()
    }

    /// Discount curve ID.
    #[getter]
    fn discount_curve_id(&self) -> &str {
        self.inner.discount_curve_id.as_str()
    }

    /// Volatility surface ID.
    #[getter]
    fn vol_surface_id(&self) -> &str {
        self.inner.vol_surface_id.as_str()
    }

    /// Day count convention.
    #[getter]
    fn day_count(&self) -> PyDayCount {
        PyDayCount::new(self.inner.day_count)
    }

    /// Realized fixings as a list of (date, price) tuples.
    #[getter]
    fn realized_fixings<'py>(&self, py: Python<'py>) -> PyResult<Py<PyAny>> {
        let result = PyList::empty(py);
        for (d, v) in &self.inner.realized_fixings {
            let py_date = date_to_py(py, *d)?;
            let tuple = PyTuple::new(py, [py_date, v.into_pyobject(py)?.into_any().unbind()])?;
            result.append(tuple)?;
        }
        Ok(result.into())
    }

    /// Get accumulated state from realized fixings: (sum, log_product, count).
    #[pyo3(signature = (as_of))]
    fn accumulated_state<'py>(
        &self,
        py: Python<'py>,
        as_of: Bound<'py, PyAny>,
    ) -> PyResult<Py<PyAny>> {
        let date = py_to_date(&as_of).context("as_of")?;
        let (sum, log_prod, count) = self.inner.accumulated_state(date);
        let tuple = PyTuple::new(
            py,
            [
                sum.into_pyobject(py)?.into_any().unbind(),
                log_prod.into_pyobject(py)?.into_any().unbind(),
                count.into_pyobject(py)?.into_any().unbind(),
            ],
        )?;
        Ok(tuple.into())
    }

    fn __repr__(&self) -> String {
        format!(
            "CommodityAsianOption(id='{}', ticker='{}', strike={}, expiry={}, averaging_method='{}')",
            self.inner.id.as_str(),
            self.inner.underlying.ticker,
            self.inner.strike,
            self.inner.expiry,
            match self.inner.averaging_method {
                AveragingMethod::Arithmetic => "arithmetic",
                AveragingMethod::Geometric => "geometric",
            }
        )
    }
}

impl fmt::Display for PyCommodityAsianOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CommodityAsianOption({}, {}, qty={})",
            self.inner.id.as_str(),
            self.inner.underlying.ticker,
            self.inner.quantity
        )
    }
}

pub(crate) fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    parent.add_class::<PyCommodityAsianOption>()?;
    parent.add_class::<PyCommodityAsianOptionBuilder>()?;
    Ok(())
}
