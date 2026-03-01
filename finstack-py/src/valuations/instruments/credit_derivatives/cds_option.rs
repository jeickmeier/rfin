#![allow(clippy::unwrap_used)]

use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::core::money::{extract_money, PyMoney};
use crate::errors::PyContext;
use crate::valuations::common::PyInstrumentType;
use finstack_core::types::InstrumentId;
use finstack_valuations::instruments::credit_derivatives::cds_option::CDSOption;
use finstack_valuations::instruments::credit_derivatives::cds_option::CDSOptionParams;
use finstack_valuations::instruments::{CreditParams, OptionType};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::{Bound, Py, PyRef, PyRefMut};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use std::fmt;
use std::sync::Arc;

const STANDARD_RECOVERY_SENIOR: f64 = 0.40;

fn parse_option_type(label: Option<&str>) -> PyResult<OptionType> {
    match label {
        None => Ok(OptionType::Call),
        Some(s) => s.parse().map_err(|e: String| PyValueError::new_err(e)),
    }
}

/// Option on CDS spread with simplified constructor.
///
/// Examples:
///     >>> opt = (
///     ...     CDSOption.builder("opt_xyz")
///     ...     .money(Money("USD", 5_000_000))
///     ...     .strike(0.015)
///     ...     .expiry(date(2024, 6, 20))
///     ...     .cds_maturity(date(2029, 6, 20))
///     ...     .discount_curve("usd_discount")
///     ...     .credit_curve("xyz_credit")
///     ...     .vol_surface("cds_vol_surface")
///     ...     .build()
///     ... )
///     >>> opt.strike
///     0.015
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "CdsOption",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyCDSOption {
    pub(crate) inner: Arc<CDSOption>,
}

impl PyCDSOption {
    pub(crate) fn new(inner: CDSOption) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

#[pyclass(
    module = "finstack.valuations.instruments",
    name = "CdsOptionBuilder",
    unsendable
)]
pub struct PyCDSOptionBuilder {
    instrument_id: InstrumentId,
    notional: Option<finstack_core::money::Money>,
    strike: Option<f64>,
    expiry: Option<time::Date>,
    cds_maturity: Option<time::Date>,
    discount_curve: Option<String>,
    credit_curve: Option<String>,
    vol_surface: Option<String>,
    option_type: OptionType,
    recovery_rate: f64,
    underlying_is_index: bool,
    index_factor: Option<f64>,
    forward_adjust: f64,
}

impl PyCDSOptionBuilder {
    fn new_with_id(id: InstrumentId) -> Self {
        Self {
            instrument_id: id,
            notional: None,
            strike: None,
            expiry: None,
            cds_maturity: None,
            discount_curve: None,
            credit_curve: None,
            vol_surface: None,
            option_type: OptionType::Call,
            recovery_rate: STANDARD_RECOVERY_SENIOR,
            underlying_is_index: false,
            index_factor: None,
            forward_adjust: 0.0,
        }
    }

    fn ensure_ready(&self) -> PyResult<()> {
        if self.notional.is_none() {
            return Err(PyValueError::new_err("notional() is required."));
        }
        if self.strike.is_none() {
            return Err(PyValueError::new_err("strike() is required."));
        }
        if self.expiry.is_none() {
            return Err(PyValueError::new_err("expiry() is required."));
        }
        if self.cds_maturity.is_none() {
            return Err(PyValueError::new_err("cds_maturity() is required."));
        }
        if self.discount_curve.as_deref().unwrap_or("").is_empty() {
            return Err(PyValueError::new_err("discount_curve() is required."));
        }
        if self.credit_curve.as_deref().unwrap_or("").is_empty() {
            return Err(PyValueError::new_err("credit_curve() is required."));
        }
        if self.vol_surface.as_deref().unwrap_or("").is_empty() {
            return Err(PyValueError::new_err("vol_surface() is required."));
        }
        if !(0.0..=1.0).contains(&self.recovery_rate) {
            return Err(PyValueError::new_err(
                "recovery_rate must be between 0 and 1",
            ));
        }
        Ok(())
    }
}

#[pymethods]
impl PyCDSOptionBuilder {
    #[new]
    #[pyo3(text_signature = "(instrument_id)")]
    fn new_py(instrument_id: &str) -> Self {
        Self::new_with_id(InstrumentId::new(instrument_id))
    }

    #[pyo3(text_signature = "($self, notional)")]
    fn notional<'py>(
        mut slf: PyRefMut<'py, Self>,
        notional: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.notional = Some(extract_money(&notional).context("notional")?);
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, money)")]
    fn money<'py>(mut slf: PyRefMut<'py, Self>, money: PyRef<'py, PyMoney>) -> PyRefMut<'py, Self> {
        slf.notional = Some(money.inner);
        slf
    }

    /// Set strike spread as a decimal rate (e.g., 0.015 for 150bp).
    #[pyo3(text_signature = "($self, strike)")]
    fn strike(mut slf: PyRefMut<'_, Self>, strike: f64) -> PyRefMut<'_, Self> {
        slf.strike = Some(strike);
        slf
    }

    /// Set strike spread in basis points (e.g., 150.0 for 150bp).
    /// Deprecated: prefer `strike()` with decimal rate.
    #[pyo3(text_signature = "($self, strike_spread_bp)")]
    fn strike_spread_bp(mut slf: PyRefMut<'_, Self>, strike_spread_bp: f64) -> PyRefMut<'_, Self> {
        slf.strike = Some(strike_spread_bp / 10000.0);
        slf
    }

    #[pyo3(text_signature = "($self, expiry)")]
    fn expiry<'py>(
        mut slf: PyRefMut<'py, Self>,
        expiry: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.expiry = Some(py_to_date(&expiry).context("expiry")?);
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, cds_maturity)")]
    fn cds_maturity<'py>(
        mut slf: PyRefMut<'py, Self>,
        cds_maturity: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.cds_maturity = Some(py_to_date(&cds_maturity).context("cds_maturity")?);
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, discount_curve)")]
    fn discount_curve(mut slf: PyRefMut<'_, Self>, discount_curve: String) -> PyRefMut<'_, Self> {
        slf.discount_curve = Some(discount_curve);
        slf
    }

    #[pyo3(text_signature = "($self, credit_curve)")]
    fn credit_curve(mut slf: PyRefMut<'_, Self>, credit_curve: String) -> PyRefMut<'_, Self> {
        slf.credit_curve = Some(credit_curve);
        slf
    }

    #[pyo3(text_signature = "($self, vol_surface)")]
    fn vol_surface(mut slf: PyRefMut<'_, Self>, vol_surface: String) -> PyRefMut<'_, Self> {
        slf.vol_surface = Some(vol_surface);
        slf
    }

    #[pyo3(text_signature = "($self, option_type)")]
    fn option_type(
        mut slf: PyRefMut<'_, Self>,
        option_type: Option<String>,
    ) -> PyResult<PyRefMut<'_, Self>> {
        slf.option_type = parse_option_type(option_type.as_deref())?;
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, recovery_rate)")]
    fn recovery_rate(mut slf: PyRefMut<'_, Self>, recovery_rate: f64) -> PyRefMut<'_, Self> {
        slf.recovery_rate = recovery_rate;
        slf
    }

    #[pyo3(text_signature = "($self, underlying_is_index)")]
    fn underlying_is_index(
        mut slf: PyRefMut<'_, Self>,
        underlying_is_index: bool,
    ) -> PyRefMut<'_, Self> {
        slf.underlying_is_index = underlying_is_index;
        slf
    }

    #[pyo3(text_signature = "($self, index_factor=None)", signature = (index_factor=None))]
    fn index_factor(mut slf: PyRefMut<'_, Self>, index_factor: Option<f64>) -> PyRefMut<'_, Self> {
        slf.index_factor = index_factor;
        slf
    }

    /// Set forward spread adjustment as a decimal rate (e.g., 0.0025 for 25bp).
    #[pyo3(text_signature = "($self, forward_adjust)")]
    fn forward_adjust(mut slf: PyRefMut<'_, Self>, forward_adjust: f64) -> PyRefMut<'_, Self> {
        slf.forward_adjust = forward_adjust;
        slf
    }

    /// Set forward spread adjustment in basis points (e.g., 25.0 for 25bp).
    /// Deprecated: prefer `forward_adjust()` with decimal rate.
    #[pyo3(text_signature = "($self, forward_adjust_bp)")]
    fn forward_adjust_bp(
        mut slf: PyRefMut<'_, Self>,
        forward_adjust_bp: f64,
    ) -> PyRefMut<'_, Self> {
        slf.forward_adjust = forward_adjust_bp / 10000.0;
        slf
    }

    #[pyo3(text_signature = "($self)")]
    fn build(slf: PyRefMut<'_, Self>) -> PyResult<PyCDSOption> {
        slf.ensure_ready()?;
        let notional = slf.notional.ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "CDSOptionBuilder internal error: missing notional after validation",
            )
        })?;
        let strike_f64 = slf.strike.ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "CDSOptionBuilder internal error: missing strike after validation",
            )
        })?;
        let strike = Decimal::try_from(strike_f64)
            .map_err(|e| PyValueError::new_err(format!("Invalid strike value: {}", e)))?;
        let expiry = slf.expiry.ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "CDSOptionBuilder internal error: missing expiry after validation",
            )
        })?;
        let cds_maturity = slf.cds_maturity.ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "CDSOptionBuilder internal error: missing cds_maturity after validation",
            )
        })?;
        let discount = slf.discount_curve.clone().unwrap();
        let credit = slf.credit_curve.clone().unwrap();
        let vol_surface = slf.vol_surface.clone().unwrap();

        let mut option_params =
            CDSOptionParams::new(strike, expiry, cds_maturity, notional, slf.option_type)
                .map_err(|e| PyValueError::new_err(e.to_string()))?;
        if slf.underlying_is_index {
            let factor = slf.index_factor.unwrap_or(1.0);
            option_params = option_params
                .as_index(factor)
                .map_err(|e| PyValueError::new_err(e.to_string()))?;
        }
        if slf.forward_adjust != 0.0 {
            let adjust = Decimal::try_from(slf.forward_adjust)
                .map_err(|e| PyValueError::new_err(format!("Invalid forward_adjust: {}", e)))?;
            option_params = option_params.with_forward_spread_adjust(adjust);
        }

        let credit_params = CreditParams::new("CDS_OPTION", slf.recovery_rate, credit.as_str());
        let option = CDSOption::new(
            slf.instrument_id.clone(),
            &option_params,
            &credit_params,
            discount.as_str(),
            vol_surface.as_str(),
        )
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(PyCDSOption::new(option))
    }

    fn __repr__(&self) -> String {
        "CDSOptionBuilder(...)".to_string()
    }
}

#[pymethods]
impl PyCDSOption {
    #[classmethod]
    #[pyo3(text_signature = "(cls, instrument_id)")]
    /// Start a fluent builder (builder-only API).
    fn builder<'py>(
        cls: &Bound<'py, PyType>,
        instrument_id: &str,
    ) -> PyResult<Py<PyCDSOptionBuilder>> {
        let py = cls.py();
        let builder = PyCDSOptionBuilder::new_with_id(InstrumentId::new(instrument_id));
        Py::new(py, builder)
    }

    /// Instrument identifier.
    ///
    /// Returns:
    ///     str: Unique identifier for the CDS option.
    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    /// Notional amount.
    ///
    /// Returns:
    ///     Money: Notional wrapped as :class:`finstack.core.money.Money`.
    #[getter]
    fn notional(&self) -> PyMoney {
        PyMoney::new(self.inner.notional)
    }

    /// Strike spread as a decimal rate (e.g., 0.015 for 150bp).
    ///
    /// Returns:
    ///     float: Strike spread as decimal rate.
    #[getter]
    fn strike(&self) -> f64 {
        self.inner.strike.to_f64().unwrap_or(0.0)
    }

    /// Strike spread in basis points (backward-compatible alias).
    ///
    /// Returns:
    ///     float: Strike spread in basis points.
    #[getter]
    fn strike_spread_bp(&self) -> f64 {
        self.inner.strike.to_f64().unwrap_or(0.0) * 10000.0
    }

    /// Option expiry date.
    ///
    /// Returns:
    ///     datetime.date: Expiry converted to Python.
    #[getter]
    fn expiry(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.expiry)
    }

    /// Maturity date of the underlying CDS.
    ///
    /// Returns:
    ///     datetime.date: Underlying maturity converted to Python.
    #[getter]
    fn cds_maturity(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.cds_maturity)
    }

    /// Discount curve identifier.
    ///
    /// Returns:
    ///     str: Discount curve used for valuation.
    #[getter]
    fn discount_curve(&self) -> String {
        self.inner.discount_curve_id.as_str().to_string()
    }

    /// Credit curve identifier.
    ///
    /// Returns:
    ///     str: Hazard curve for the reference entity or index.
    #[getter]
    fn credit_curve(&self) -> String {
        self.inner.credit_curve_id.as_str().to_string()
    }

    /// Instrument type enumeration.
    ///
    /// Returns:
    ///     InstrumentType: Enumeration value ``InstrumentType.CDS_OPTION``.
    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::CDSOption)
    }

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!(
            "CDSOption(id='{}', strike={}, type='{}')",
            self.inner.id,
            self.inner.strike,
            match self.inner.option_type {
                OptionType::Call => "call",
                OptionType::Put => "put",
            }
        ))
    }
}

impl fmt::Display for PyCDSOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CDSOption({}, strike={})",
            self.inner.id, self.inner.strike
        )
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyCDSOption>()?;
    module.add_class::<PyCDSOptionBuilder>()?;
    Ok(vec!["CDSOption", "CDSOptionBuilder"])
}
