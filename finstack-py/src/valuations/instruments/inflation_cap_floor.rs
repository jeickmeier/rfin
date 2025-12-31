use crate::core::dates::calendar::PyBusinessDayConvention;
use crate::core::dates::daycount::PyDayCount;
use crate::core::dates::schedule::PyStubKind;
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::core::money::PyMoney;
use crate::errors::PyContext;
use crate::valuations::common::PyInstrumentType;
use finstack_core::currency::Currency;
use finstack_core::dates::{BusinessDayConvention, DayCount, StubKind, Tenor};
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::PricingOverrides;
use finstack_valuations::instruments::rates::inflation_cap_floor::{InflationCapFloor, InflationCapFloorType};
use finstack_valuations::instruments::Attributes;
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::{Bound, Py, PyRef, PyRefMut};
use std::fmt;

/// Year-on-year inflation cap or floor instrument.
///
/// Prices YoY inflation caps/floors using Black-76 (lognormal) or Bachelier (normal)
/// volatility models on the forward YoY inflation rate.
///
/// Examples
/// --------
/// Create an inflation cap::
///
///     from finstack import Money, Date
///     from finstack.valuations.instruments import InflationCapFloor
///
///     cap = (
///         InflationCapFloor.builder("INFLATION_CAP_001")
///         .option_type("cap")
///         .notional(10_000_000, "USD")
///         .strike_rate(0.025)  # 2.5% strike
///         .start_date(Date(2024, 1, 1))
///         .end_date(Date(2029, 1, 1))
///         .frequency("annual")
///         .inflation_index_id("US-CPI-U")
///         .disc_id("USD-OIS")
///         .vol_surface_id("USD-INFLATION-VOL")
///         .build()
///     )
///
/// See Also
/// --------
/// InflationSwap : Vanilla inflation swap
/// InterestRateOption : Interest rate caps/floors
#[pyclass(module = "finstack.valuations.instruments", name = "InflationCapFloor", frozen)]
#[derive(Clone, Debug)]
pub struct PyInflationCapFloor {
    pub(crate) inner: InflationCapFloor,
}

impl PyInflationCapFloor {
    pub(crate) fn new(inner: InflationCapFloor) -> Self {
        Self { inner }
    }
}

#[pyclass(
    module = "finstack.valuations.instruments",
    name = "InflationCapFloorBuilder",
    unsendable
)]
pub struct PyInflationCapFloorBuilder {
    instrument_id: InstrumentId,
    option_type: InflationCapFloorType,
    pending_notional: Option<Money>,
    strike_rate: f64,
    start_date: Option<time::Date>,
    end_date: Option<time::Date>,
    frequency: Tenor,
    day_count: DayCount,
    stub_kind: StubKind,
    bdc: BusinessDayConvention,
    calendar_id: Option<String>,
    inflation_index_id: Option<CurveId>,
    discount_curve_id: Option<CurveId>,
    vol_surface_id: Option<CurveId>,
}

impl PyInflationCapFloorBuilder {
    fn new_with_id(id: InstrumentId) -> Self {
        Self {
            instrument_id: id,
            option_type: InflationCapFloorType::Cap,
            pending_notional: None,
            strike_rate: 0.0,
            start_date: None,
            end_date: None,
            frequency: Tenor::annual(),
            day_count: DayCount::Act365F,
            stub_kind: StubKind::None,
            bdc: BusinessDayConvention::Following,
            calendar_id: None,
            inflation_index_id: None,
            discount_curve_id: None,
            vol_surface_id: None,
        }
    }

    fn parse_option_type(value: &str) -> PyResult<InflationCapFloorType> {
        match value.to_lowercase().as_str() {
            "cap" => Ok(InflationCapFloorType::Cap),
            "floor" => Ok(InflationCapFloorType::Floor),
            "caplet" => Ok(InflationCapFloorType::Caplet),
            "floorlet" => Ok(InflationCapFloorType::Floorlet),
            other => Err(PyValueError::new_err(format!(
                "option_type() expects 'cap', 'floor', 'caplet', or 'floorlet', got '{}'",
                other
            ))),
        }
    }

    fn parse_frequency(value: &Bound<'_, PyAny>) -> PyResult<Tenor> {
        if let Ok(name) = value.extract::<&str>() {
            let normalized = name.to_lowercase();
            return match normalized.as_str() {
                "annual" | "1y" | "yearly" => Ok(Tenor::annual()),
                "semiannual" | "semi_annual" | "6m" | "semi" => Ok(Tenor::semi_annual()),
                "quarterly" | "qtr" | "3m" => Ok(Tenor::quarterly()),
                "monthly" | "1m" => Ok(Tenor::monthly()),
                other => Err(PyValueError::new_err(format!(
                    "Unsupported frequency '{}'",
                    other
                ))),
            };
        }
        if let Ok(payments) = value.extract::<u32>() {
            return Tenor::from_payments_per_year(payments)
                .map_err(|msg| PyValueError::new_err(msg.to_string()));
        }
        Err(PyTypeError::new_err(
            "frequency() expects str or int payments_per_year",
        ))
    }

    fn parse_day_count(value: &Bound<'_, PyAny>) -> PyResult<DayCount> {
        if let Ok(py_dc) = value.extract::<PyRef<PyDayCount>>() {
            return Ok(py_dc.inner);
        }
        if let Ok(name) = value.extract::<&str>() {
            return match name.to_lowercase().as_str() {
                "act_360" | "act/360" => Ok(DayCount::Act360),
                "act_365f" | "act/365f" | "act365f" => Ok(DayCount::Act365F),
                "act_act" | "act/act" | "actact" => Ok(DayCount::ActAct),
                "thirty_360" | "30/360" | "30e/360" => Ok(DayCount::Thirty360),
                other => Err(PyValueError::new_err(format!(
                    "Unsupported day count '{}'",
                    other
                ))),
            };
        }
        Err(PyTypeError::new_err("day_count() expects DayCount or str"))
    }

    fn parse_bdc(value: &Bound<'_, PyAny>) -> PyResult<BusinessDayConvention> {
        if let Ok(py_bdc) = value.extract::<PyRef<PyBusinessDayConvention>>() {
            return Ok(py_bdc.inner);
        }
        if let Ok(name) = value.extract::<&str>() {
            return match name.to_lowercase().as_str() {
                "following" => Ok(BusinessDayConvention::Following),
                "modified_following" | "mod_following" => {
                    Ok(BusinessDayConvention::ModifiedFollowing)
                }
                "preceding" => Ok(BusinessDayConvention::Preceding),
                "modified_preceding" | "mod_preceding" => {
                    Ok(BusinessDayConvention::ModifiedPreceding)
                }
                "unadjusted" => Ok(BusinessDayConvention::Unadjusted),
                other => Err(PyValueError::new_err(format!(
                    "Unsupported business day convention '{}'",
                    other
                ))),
            };
        }
        Err(PyTypeError::new_err(
            "bdc() expects BusinessDayConvention or str",
        ))
    }

    fn parse_stub(value: &Bound<'_, PyAny>) -> PyResult<StubKind> {
        if let Ok(py_stub) = value.extract::<PyRef<PyStubKind>>() {
            return Ok(py_stub.inner);
        }
        if let Ok(name) = value.extract::<&str>() {
            return match name.to_lowercase().as_str() {
                "none" => Ok(StubKind::None),
                "short_front" => Ok(StubKind::ShortFront),
                "short_back" => Ok(StubKind::ShortBack),
                "long_front" => Ok(StubKind::LongFront),
                "long_back" => Ok(StubKind::LongBack),
                other => Err(PyValueError::new_err(format!(
                    "Unsupported stub kind '{}'",
                    other
                ))),
            };
        }
        Err(PyTypeError::new_err("stub() expects StubKind or str"))
    }
}

#[pymethods]
impl PyInflationCapFloorBuilder {
    #[new]
    #[pyo3(text_signature = "(instrument_id)")]
    fn new_py(instrument_id: &str) -> Self {
        Self::new_with_id(InstrumentId::new(instrument_id))
    }

    /// Set option type (cap, floor, caplet, or floorlet).
    ///
    /// Parameters
    /// ----------
    /// option_type : str
    ///     One of: "cap", "floor", "caplet", "floorlet"
    #[pyo3(text_signature = "($self, option_type)")]
    fn option_type<'py>(mut slf: PyRefMut<'py, Self>, option_type: &str) -> PyResult<PyRefMut<'py, Self>> {
        slf.option_type = Self::parse_option_type(option_type)?;
        Ok(slf)
    }

    /// Set notional amount and currency.
    ///
    /// Parameters
    /// ----------
    /// amount : float
    ///     Notional amount
    /// currency : str
    ///     Currency code (e.g., "USD")
    #[pyo3(text_signature = "($self, amount, currency)")]
    fn notional<'py>(
        mut slf: PyRefMut<'py, Self>,
        amount: f64,
        currency: &str,
    ) -> PyResult<PyRefMut<'py, Self>> {
        if amount <= 0.0 {
            return Err(PyValueError::new_err("notional must be positive"));
        }
        let ccy: Currency = currency
            .parse()
            .map_err(|_| PyValueError::new_err("Invalid currency code"))?;
        slf.pending_notional = Some(Money::new(amount, ccy));
        Ok(slf)
    }

    /// Set strike rate (annualized, decimal).
    ///
    /// Parameters
    /// ----------
    /// rate : float
    ///     Strike rate (e.g., 0.025 for 2.5%)
    #[pyo3(text_signature = "($self, rate)")]
    fn strike_rate(mut slf: PyRefMut<'_, Self>, rate: f64) -> PyResult<PyRefMut<'_, Self>> {
        if rate < 0.0 {
            return Err(PyValueError::new_err("strike_rate must be non-negative"));
        }
        slf.strike_rate = rate;
        Ok(slf)
    }

    /// Set start date of the first inflation period.
    #[pyo3(text_signature = "($self, date)")]
    fn start_date<'py>(
        mut slf: PyRefMut<'py, Self>,
        date: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.start_date = Some(py_to_date(&date).context("start_date")?);
        Ok(slf)
    }

    /// Set end date of the final inflation period.
    #[pyo3(text_signature = "($self, date)")]
    fn end_date<'py>(
        mut slf: PyRefMut<'py, Self>,
        date: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.end_date = Some(py_to_date(&date).context("end_date")?);
        Ok(slf)
    }

    /// Set payment frequency (ignored for caplet/floorlet).
    #[pyo3(text_signature = "($self, frequency)")]
    fn frequency<'py>(
        mut slf: PyRefMut<'py, Self>,
        frequency: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.frequency = Self::parse_frequency(&frequency)?;
        Ok(slf)
    }

    /// Set day count convention for accrual and option time.
    #[pyo3(text_signature = "($self, day_count)")]
    fn day_count<'py>(
        mut slf: PyRefMut<'py, Self>,
        day_count: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.day_count = Self::parse_day_count(&day_count)?;
        Ok(slf)
    }

    /// Set business day convention for schedule and payments.
    #[pyo3(text_signature = "($self, bdc)")]
    fn bdc<'py>(
        mut slf: PyRefMut<'py, Self>,
        bdc: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.bdc = Self::parse_bdc(&bdc)?;
        Ok(slf)
    }

    /// Set stub handling convention for irregular periods.
    #[pyo3(text_signature = "($self, stub)")]
    fn stub<'py>(
        mut slf: PyRefMut<'py, Self>,
        stub: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.stub_kind = Self::parse_stub(&stub)?;
        Ok(slf)
    }

    /// Set optional holiday calendar identifier.
    #[pyo3(text_signature = "($self, calendar_id)")]
    fn calendar_id(mut slf: PyRefMut<'_, Self>, calendar_id: Option<String>) -> PyRefMut<'_, Self> {
        slf.calendar_id = calendar_id;
        slf
    }

    /// Set inflation index/curve identifier (e.g., "US-CPI-U").
    #[pyo3(text_signature = "($self, curve_id)")]
    fn inflation_index_id(mut slf: PyRefMut<'_, Self>, curve_id: String) -> PyRefMut<'_, Self> {
        slf.inflation_index_id = Some(CurveId::new(&curve_id));
        slf
    }

    /// Set discount curve identifier.
    #[pyo3(text_signature = "($self, curve_id)")]
    fn disc_id(mut slf: PyRefMut<'_, Self>, curve_id: String) -> PyRefMut<'_, Self> {
        slf.discount_curve_id = Some(CurveId::new(&curve_id));
        slf
    }

    /// Set volatility surface identifier.
    #[pyo3(text_signature = "($self, curve_id)")]
    fn vol_surface_id(mut slf: PyRefMut<'_, Self>, curve_id: String) -> PyRefMut<'_, Self> {
        slf.vol_surface_id = Some(CurveId::new(&curve_id));
        slf
    }

    /// Build the InflationCapFloor instrument.
    ///
    /// Returns
    /// -------
    /// InflationCapFloor
    ///     Validated inflation cap/floor instrument
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If required fields are missing or validation fails
    #[pyo3(text_signature = "($self)")]
    fn build(slf: PyRefMut<'_, Self>) -> PyResult<PyInflationCapFloor> {
        let notional = slf
            .pending_notional
            .ok_or_else(|| PyValueError::new_err("notional() must be provided"))?;

        let start_date = slf
            .start_date
            .ok_or_else(|| PyValueError::new_err("start_date() must be provided"))?;

        let end_date = slf
            .end_date
            .ok_or_else(|| PyValueError::new_err("end_date() must be provided"))?;

        let inflation_index_id = slf
            .inflation_index_id
            .clone()
            .ok_or_else(|| PyValueError::new_err("inflation_index_id() must be provided"))?;

        let discount_curve_id = slf
            .discount_curve_id
            .clone()
            .ok_or_else(|| PyValueError::new_err("disc_id() must be provided"))?;

        let vol_surface_id = slf
            .vol_surface_id
            .clone()
            .ok_or_else(|| PyValueError::new_err("vol_surface_id() must be provided"))?;

        let instrument = InflationCapFloor {
            id: slf.instrument_id.clone(),
            option_type: slf.option_type,
            notional,
            strike_rate: slf.strike_rate,
            start_date,
            end_date,
            frequency: slf.frequency,
            day_count: slf.day_count,
            stub_kind: slf.stub_kind,
            bdc: slf.bdc,
            calendar_id: slf.calendar_id.clone(),
            inflation_index_id,
            discount_curve_id,
            vol_surface_id,
            pricing_overrides: PricingOverrides::default(),
            lag_override: None,
            attributes: Attributes::new(),
        };

        Ok(PyInflationCapFloor::new(instrument))
    }
}

#[pymethods]
impl PyInflationCapFloor {
    /// Create a builder for constructing InflationCapFloor instruments.
    ///
    /// Parameters
    /// ----------
    /// instrument_id : str
    ///     Unique identifier for the instrument
    ///
    /// Returns
    /// -------
    /// InflationCapFloorBuilder
    ///     Builder instance for fluent construction
    #[classmethod]
    #[pyo3(text_signature = "(cls, instrument_id)")]
    fn builder(_cls: &Bound<'_, PyType>, instrument_id: &str) -> PyInflationCapFloorBuilder {
        PyInflationCapFloorBuilder::new_with_id(InstrumentId::new(instrument_id))
    }

    /// Instrument identifier.
    #[getter]
    fn id(&self) -> String {
        self.inner.id.as_str().to_string()
    }

    /// Option type (cap, floor, caplet, or floorlet).
    #[getter]
    fn option_type(&self) -> String {
        self.inner.option_type.to_string()
    }

    /// Notional amount.
    #[getter]
    fn notional(&self) -> PyMoney {
        PyMoney::new(self.inner.notional)
    }

    /// Strike rate (annualized, decimal).
    #[getter]
    fn strike_rate(&self) -> f64 {
        self.inner.strike_rate
    }

    /// Start date of the first inflation period.
    #[getter]
    fn start_date(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.start_date)
    }

    /// End date of the final inflation period.
    #[getter]
    fn end_date(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.end_date)
    }

    /// Inflation index identifier.
    #[getter]
    fn inflation_index_id(&self) -> String {
        self.inner.inflation_index_id.as_str().to_string()
    }

    /// Discount curve identifier.
    #[getter]
    fn discount_curve(&self) -> String {
        self.inner.discount_curve_id.as_str().to_string()
    }

    /// Volatility surface identifier.
    #[getter]
    fn vol_surface_id(&self) -> String {
        self.inner.vol_surface_id.as_str().to_string()
    }

    /// Instrument type enum.
    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::InflationCapFloor)
    }

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!(
            "InflationCapFloor(id='{}', type='{}', strike={:.4})",
            self.inner.id, self.inner.option_type, self.inner.strike_rate
        ))
    }
}

impl fmt::Display for PyInflationCapFloor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "InflationCapFloor({})", self.inner.id)
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyInflationCapFloor>()?;
    module.add_class::<PyInflationCapFloorBuilder>()?;
    Ok(vec!["InflationCapFloor", "InflationCapFloorBuilder"])
}
