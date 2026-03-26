use crate::core::common::args::{CurrencyArg, DayCountArg};
use crate::core::common::labels::normalize_label;
use crate::core::currency::PyCurrency;
use crate::core::dates::daycount::PyDayCount;
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::errors::{core_to_py, PyContext};
use crate::valuations::common::PyInstrumentType;
use finstack_core::dates::DayCount;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::commodity::commodity_spread_option::CommoditySpreadOption;
use finstack_valuations::instruments::OptionType;
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::{Bound, Py, PyRefMut};
use std::fmt;
use std::sync::Arc;

fn parse_option_type(label: &str) -> PyResult<OptionType> {
    match normalize_label(label).as_str() {
        "call" => Ok(OptionType::Call),
        "put" => Ok(OptionType::Put),
        other => Err(PyValueError::new_err(format!(
            "Invalid option_type: '{other}'. Must be 'call' or 'put'"
        ))),
    }
}

/// Option on the spread between two commodity forward prices.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "CommoditySpreadOption",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyCommoditySpreadOption {
    pub(crate) inner: Arc<CommoditySpreadOption>,
}

impl PyCommoditySpreadOption {
    pub(crate) fn new(inner: CommoditySpreadOption) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

#[pyclass(
    module = "finstack.valuations.instruments",
    name = "CommoditySpreadOptionBuilder",
    unsendable
)]
pub struct PyCommoditySpreadOptionBuilder {
    instrument_id: InstrumentId,
    currency: Option<finstack_core::currency::Currency>,
    option_type: OptionType,
    expiry: Option<time::Date>,
    strike: Option<f64>,
    notional: Option<f64>,
    leg1_forward_curve_id: Option<CurveId>,
    leg2_forward_curve_id: Option<CurveId>,
    leg1_vol_surface_id: Option<CurveId>,
    leg2_vol_surface_id: Option<CurveId>,
    discount_curve_id: Option<CurveId>,
    correlation: Option<f64>,
    day_count: DayCount,
}

impl PyCommoditySpreadOptionBuilder {
    fn new_with_id(id: InstrumentId) -> Self {
        Self {
            instrument_id: id,
            currency: None,
            option_type: OptionType::Call,
            expiry: None,
            strike: None,
            notional: None,
            leg1_forward_curve_id: None,
            leg2_forward_curve_id: None,
            leg1_vol_surface_id: None,
            leg2_vol_surface_id: None,
            discount_curve_id: None,
            correlation: None,
            day_count: DayCount::Act365F,
        }
    }

    fn ensure_ready(&self) -> PyResult<()> {
        if self.currency.is_none() {
            return Err(PyValueError::new_err("currency() is required."));
        }
        if self.expiry.is_none() {
            return Err(PyValueError::new_err("expiry() is required."));
        }
        if self.strike.is_none() {
            return Err(PyValueError::new_err("strike() is required."));
        }
        if self.notional.is_none() {
            return Err(PyValueError::new_err("notional() is required."));
        }
        if self.leg1_forward_curve_id.is_none() {
            return Err(PyValueError::new_err(
                "leg1_forward_curve_id() is required.",
            ));
        }
        if self.leg2_forward_curve_id.is_none() {
            return Err(PyValueError::new_err(
                "leg2_forward_curve_id() is required.",
            ));
        }
        if self.leg1_vol_surface_id.is_none() {
            return Err(PyValueError::new_err("leg1_vol_surface_id() is required."));
        }
        if self.leg2_vol_surface_id.is_none() {
            return Err(PyValueError::new_err("leg2_vol_surface_id() is required."));
        }
        if self.discount_curve_id.is_none() {
            return Err(PyValueError::new_err("discount_curve_id() is required."));
        }
        if self.correlation.is_none() {
            return Err(PyValueError::new_err("correlation() is required."));
        }
        Ok(())
    }
}

#[pymethods]
impl PyCommoditySpreadOptionBuilder {
    #[new]
    #[pyo3(text_signature = "(instrument_id)")]
    fn new_py(instrument_id: &str) -> Self {
        Self::new_with_id(InstrumentId::new(instrument_id))
    }

    #[pyo3(text_signature = "($self, currency)")]
    fn currency<'py>(
        mut slf: PyRefMut<'py, Self>,
        currency: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let CurrencyArg(ccy) = currency.extract().context("currency")?;
        slf.currency = Some(ccy);
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, option_type)")]
    fn option_type(
        mut slf: PyRefMut<'_, Self>,
        option_type: String,
    ) -> PyResult<PyRefMut<'_, Self>> {
        slf.option_type = parse_option_type(&option_type)?;
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, expiry)")]
    fn expiry<'py>(
        mut slf: PyRefMut<'py, Self>,
        expiry: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.expiry = Some(py_to_date(&expiry).context("expiry")?);
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, strike)")]
    fn strike(mut slf: PyRefMut<'_, Self>, strike: f64) -> PyRefMut<'_, Self> {
        slf.strike = Some(strike);
        slf
    }

    #[pyo3(text_signature = "($self, notional)")]
    fn notional(mut slf: PyRefMut<'_, Self>, notional: f64) -> PyResult<PyRefMut<'_, Self>> {
        if notional <= 0.0 {
            return Err(PyValueError::new_err("notional must be positive"));
        }
        slf.notional = Some(notional);
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, curve_id)")]
    fn leg1_forward_curve_id(mut slf: PyRefMut<'_, Self>, curve_id: String) -> PyRefMut<'_, Self> {
        slf.leg1_forward_curve_id = Some(CurveId::new(curve_id.as_str()));
        slf
    }

    #[pyo3(text_signature = "($self, curve_id)")]
    fn leg2_forward_curve_id(mut slf: PyRefMut<'_, Self>, curve_id: String) -> PyRefMut<'_, Self> {
        slf.leg2_forward_curve_id = Some(CurveId::new(curve_id.as_str()));
        slf
    }

    #[pyo3(text_signature = "($self, curve_id)")]
    fn leg1_vol_surface_id(mut slf: PyRefMut<'_, Self>, curve_id: String) -> PyRefMut<'_, Self> {
        slf.leg1_vol_surface_id = Some(CurveId::new(curve_id.as_str()));
        slf
    }

    #[pyo3(text_signature = "($self, curve_id)")]
    fn leg2_vol_surface_id(mut slf: PyRefMut<'_, Self>, curve_id: String) -> PyRefMut<'_, Self> {
        slf.leg2_vol_surface_id = Some(CurveId::new(curve_id.as_str()));
        slf
    }

    #[pyo3(text_signature = "($self, curve_id)")]
    fn discount_curve_id(mut slf: PyRefMut<'_, Self>, curve_id: String) -> PyRefMut<'_, Self> {
        slf.discount_curve_id = Some(CurveId::new(curve_id.as_str()));
        slf
    }

    #[pyo3(text_signature = "($self, correlation)")]
    fn correlation(mut slf: PyRefMut<'_, Self>, correlation: f64) -> PyRefMut<'_, Self> {
        slf.correlation = Some(correlation);
        slf
    }

    #[pyo3(text_signature = "($self, day_count)")]
    fn day_count<'py>(
        mut slf: PyRefMut<'py, Self>,
        day_count: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let DayCountArg(value) = day_count.extract().context("day_count")?;
        slf.day_count = value;
        Ok(slf)
    }

    #[pyo3(text_signature = "($self)")]
    fn build(slf: PyRefMut<'_, Self>) -> PyResult<PyCommoditySpreadOption> {
        slf.ensure_ready()?;
        let currency = slf.currency.ok_or_else(|| {
            PyRuntimeError::new_err(
                "CommoditySpreadOptionBuilder internal error: missing currency after validation",
            )
        })?;
        let expiry = slf.expiry.ok_or_else(|| {
            PyRuntimeError::new_err(
                "CommoditySpreadOptionBuilder internal error: missing expiry after validation",
            )
        })?;
        let strike = slf.strike.ok_or_else(|| {
            PyRuntimeError::new_err(
                "CommoditySpreadOptionBuilder internal error: missing strike after validation",
            )
        })?;
        let notional = slf.notional.ok_or_else(|| {
            PyRuntimeError::new_err(
                "CommoditySpreadOptionBuilder internal error: missing notional after validation",
            )
        })?;
        let leg1_forward_curve_id = slf.leg1_forward_curve_id.clone().ok_or_else(|| {
            PyRuntimeError::new_err(
                "CommoditySpreadOptionBuilder internal error: missing leg1 forward curve after validation",
            )
        })?;
        let leg2_forward_curve_id = slf.leg2_forward_curve_id.clone().ok_or_else(|| {
            PyRuntimeError::new_err(
                "CommoditySpreadOptionBuilder internal error: missing leg2 forward curve after validation",
            )
        })?;
        let leg1_vol_surface_id = slf.leg1_vol_surface_id.clone().ok_or_else(|| {
            PyRuntimeError::new_err(
                "CommoditySpreadOptionBuilder internal error: missing leg1 vol surface after validation",
            )
        })?;
        let leg2_vol_surface_id = slf.leg2_vol_surface_id.clone().ok_or_else(|| {
            PyRuntimeError::new_err(
                "CommoditySpreadOptionBuilder internal error: missing leg2 vol surface after validation",
            )
        })?;
        let discount_curve_id = slf.discount_curve_id.clone().ok_or_else(|| {
            PyRuntimeError::new_err(
                "CommoditySpreadOptionBuilder internal error: missing discount curve after validation",
            )
        })?;
        let correlation = slf.correlation.ok_or_else(|| {
            PyRuntimeError::new_err(
                "CommoditySpreadOptionBuilder internal error: missing correlation after validation",
            )
        })?;

        CommoditySpreadOption::builder()
            .id(slf.instrument_id.clone())
            .currency(currency)
            .option_type(slf.option_type)
            .expiry(expiry)
            .strike(strike)
            .notional(notional)
            .leg1_forward_curve_id(leg1_forward_curve_id)
            .leg2_forward_curve_id(leg2_forward_curve_id)
            .leg1_vol_surface_id(leg1_vol_surface_id)
            .leg2_vol_surface_id(leg2_vol_surface_id)
            .discount_curve_id(discount_curve_id)
            .correlation(correlation)
            .day_count(slf.day_count)
            .build()
            .map(PyCommoditySpreadOption::new)
            .map_err(core_to_py)
    }

    fn __repr__(&self) -> String {
        "CommoditySpreadOptionBuilder(...)".to_string()
    }
}

#[pymethods]
impl PyCommoditySpreadOption {
    #[classmethod]
    #[pyo3(text_signature = "(cls, instrument_id)")]
    fn builder<'py>(
        cls: &Bound<'py, PyType>,
        instrument_id: &str,
    ) -> PyResult<Py<PyCommoditySpreadOptionBuilder>> {
        Py::new(
            cls.py(),
            PyCommoditySpreadOptionBuilder::new_with_id(InstrumentId::new(instrument_id)),
        )
    }

    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    #[getter]
    fn currency(&self) -> PyCurrency {
        PyCurrency::new(self.inner.currency)
    }

    #[getter]
    fn option_type(&self) -> &'static str {
        match self.inner.option_type {
            OptionType::Call => "call",
            OptionType::Put => "put",
        }
    }

    #[getter]
    fn expiry(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.expiry)
    }

    #[getter]
    fn strike(&self) -> f64 {
        self.inner.strike
    }

    #[getter]
    fn notional(&self) -> f64 {
        self.inner.notional
    }

    #[getter]
    fn leg1_forward_curve_id(&self) -> String {
        self.inner.leg1_forward_curve_id.as_str().to_string()
    }

    #[getter]
    fn leg2_forward_curve_id(&self) -> String {
        self.inner.leg2_forward_curve_id.as_str().to_string()
    }

    #[getter]
    fn leg1_vol_surface_id(&self) -> String {
        self.inner.leg1_vol_surface_id.as_str().to_string()
    }

    #[getter]
    fn leg2_vol_surface_id(&self) -> String {
        self.inner.leg2_vol_surface_id.as_str().to_string()
    }

    #[getter]
    fn discount_curve_id(&self) -> String {
        self.inner.discount_curve_id.as_str().to_string()
    }

    #[getter]
    fn correlation(&self) -> f64 {
        self.inner.correlation
    }

    #[getter]
    fn day_count(&self) -> PyDayCount {
        PyDayCount::new(self.inner.day_count)
    }

    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::CommoditySpreadOption)
    }

    fn __repr__(&self) -> String {
        format!(
            "CommoditySpreadOption(id='{}', strike={}, option_type='{}')",
            self.inner.id.as_str(),
            self.inner.strike,
            self.option_type()
        )
    }
}

impl fmt::Display for PyCommoditySpreadOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CommoditySpreadOption({}, strike={}, option_type={})",
            self.inner.id.as_str(),
            self.inner.strike,
            match self.inner.option_type {
                OptionType::Call => "call",
                OptionType::Put => "put",
            }
        )
    }
}

pub(crate) fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    parent.add_class::<PyCommoditySpreadOption>()?;
    parent.add_class::<PyCommoditySpreadOptionBuilder>()?;
    Ok(())
}
