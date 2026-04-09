use super::cds::normalize_cds_side;
use super::cds::PyCdsConvention;
use super::cds::PyCdsPayReceive;
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::core::market_data::context::PyMarketContext;
use crate::core::money::{extract_money, PyMoney};
use crate::errors::core_to_py;
use crate::errors::PyContext;
use crate::valuations::common::PyInstrumentType;
use finstack_core::types::InstrumentId;
use finstack_valuations::instruments::credit_derivatives::cds::{CDSConvention, PayReceive};
use finstack_valuations::instruments::credit_derivatives::cds_index::CDSIndex;
use finstack_valuations::instruments::credit_derivatives::cds_index::{
    CDSIndexConstructionParams, CDSIndexParams,
};
use finstack_valuations::instruments::CreditParams;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::{Bound, Py, PyRef, PyRefMut};
use rust_decimal::prelude::ToPrimitive;
use std::fmt;
use std::sync::Arc;

use finstack_valuations::instruments::credit_derivatives::cds::RECOVERY_SENIOR_UNSECURED;

/// CDS index instrument binding exposing a simplified constructor.
///
/// Examples:
///     >>> itraxx = (
///     ...     CDSIndex.builder("itraxx_main")
///     ...     .index_name("iTraxx Europe")
///     ...     .series(38)
///     ...     .version(1)
///     ...     .money(Money("EUR", 10_000_000))
///     ...     .fixed_coupon_bp(100.0)
///     ...     .start_date(date(2024, 3, 20))
///     ...     .maturity(date(2029, 3, 20))
///     ...     .discount_curve("eur_discount")
///     ...     .credit_curve("itraxx_credit")
///     ...     .build()
///     ... )
///     >>> itraxx.fixed_coupon_bp
///     100.0
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "CDSIndex",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyCdsIndex {
    pub(crate) inner: Arc<CDSIndex>,
}

impl PyCdsIndex {
    pub(crate) fn new(inner: CDSIndex) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

/// Constituent in a CDS index.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "CDSIndexConstituent",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyCdsIndexConstituent {
    credit_curve: String,
    recovery_rate: f64,
    weight: f64,
    defaulted: bool,
}

#[pymethods]
impl PyCdsIndexConstituent {
    /// Credit curve identifier for this constituent.
    #[getter]
    fn credit_curve(&self) -> &str {
        &self.credit_curve
    }

    /// Recovery rate for this constituent.
    #[getter]
    fn recovery_rate(&self) -> f64 {
        self.recovery_rate
    }

    /// Weight of this constituent in the index.
    #[getter]
    fn weight(&self) -> f64 {
        self.weight
    }

    /// Whether this constituent has defaulted.
    #[getter]
    fn defaulted(&self) -> bool {
        self.defaulted
    }

    fn __repr__(&self) -> String {
        format!(
            "CDSIndexConstituent(credit_curve='{}', weight={:.4}, defaulted={})",
            self.credit_curve, self.weight, self.defaulted
        )
    }
}

#[pyclass(module = "finstack.valuations.instruments", name = "CDSIndexBuilder")]
pub struct PyCdsIndexBuilder {
    instrument_id: InstrumentId,
    index_name: Option<String>,
    series: Option<u16>,
    version: Option<u16>,
    notional: Option<finstack_core::money::Money>,
    fixed_coupon_bp: Option<f64>,
    start_date: Option<time::Date>,
    maturity: Option<time::Date>,
    discount_curve: Option<String>,
    credit_curve: Option<String>,
    side: PayReceive,
    recovery_rate: f64,
    index_factor: Option<f64>,
    convention: CDSConvention,
}

impl PyCdsIndexBuilder {
    fn new_with_id(id: InstrumentId) -> Self {
        Self {
            instrument_id: id,
            index_name: None,
            series: None,
            version: None,
            notional: None,
            fixed_coupon_bp: None,
            start_date: None,
            maturity: None,
            discount_curve: None,
            credit_curve: None,
            side: PayReceive::PayFixed,
            recovery_rate: RECOVERY_SENIOR_UNSECURED,
            index_factor: None,
            convention: CDSConvention::IsdaNa,
        }
    }

    fn ensure_ready(&self) -> PyResult<()> {
        if self.index_name.as_deref().unwrap_or("").is_empty() {
            return Err(PyValueError::new_err("index_name() is required."));
        }
        if self.series.is_none() {
            return Err(PyValueError::new_err("series() is required."));
        }
        if self.version.is_none() {
            return Err(PyValueError::new_err("version() is required."));
        }
        if self.notional.is_none() {
            return Err(PyValueError::new_err("notional() is required."));
        }
        if self.fixed_coupon_bp.is_none() {
            return Err(PyValueError::new_err("fixed_coupon_bp() is required."));
        }
        if self.start_date.is_none() {
            return Err(PyValueError::new_err("start_date() is required."));
        }
        if self.maturity.is_none() {
            return Err(PyValueError::new_err("maturity() is required."));
        }
        if self.discount_curve.as_deref().unwrap_or("").is_empty() {
            return Err(PyValueError::new_err("discount_curve() is required."));
        }
        if self.credit_curve.as_deref().unwrap_or("").is_empty() {
            return Err(PyValueError::new_err("credit_curve() is required."));
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
impl PyCdsIndexBuilder {
    #[new]
    #[pyo3(text_signature = "(instrument_id)")]
    fn new_py(instrument_id: &str) -> Self {
        Self::new_with_id(InstrumentId::new(instrument_id))
    }

    #[pyo3(text_signature = "($self, index_name)")]
    fn index_name(mut slf: PyRefMut<'_, Self>, index_name: String) -> PyRefMut<'_, Self> {
        slf.index_name = Some(index_name);
        slf
    }

    #[pyo3(text_signature = "($self, series)")]
    fn series(mut slf: PyRefMut<'_, Self>, series: u16) -> PyRefMut<'_, Self> {
        slf.series = Some(series);
        slf
    }

    #[pyo3(text_signature = "($self, version)")]
    fn version(mut slf: PyRefMut<'_, Self>, version: u16) -> PyRefMut<'_, Self> {
        slf.version = Some(version);
        slf
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

    #[pyo3(text_signature = "($self, fixed_coupon_bp)")]
    fn fixed_coupon_bp(mut slf: PyRefMut<'_, Self>, fixed_coupon_bp: f64) -> PyRefMut<'_, Self> {
        slf.fixed_coupon_bp = Some(fixed_coupon_bp);
        slf
    }

    #[pyo3(text_signature = "($self, start_date)")]
    fn start_date<'py>(
        mut slf: PyRefMut<'py, Self>,
        start_date: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.start_date = Some(py_to_date(&start_date).context("start_date")?);
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, maturity)")]
    fn maturity<'py>(
        mut slf: PyRefMut<'py, Self>,
        maturity: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.maturity = Some(py_to_date(&maturity).context("maturity")?);
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

    #[pyo3(text_signature = "($self, side)")]
    fn side(mut slf: PyRefMut<'_, Self>, side: String) -> PyResult<PyRefMut<'_, Self>> {
        slf.side = normalize_cds_side(&side).context("side")?;
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, recovery_rate)")]
    fn recovery_rate(mut slf: PyRefMut<'_, Self>, recovery_rate: f64) -> PyRefMut<'_, Self> {
        slf.recovery_rate = recovery_rate;
        slf
    }

    #[pyo3(text_signature = "($self, index_factor=None)", signature = (index_factor=None))]
    fn index_factor(mut slf: PyRefMut<'_, Self>, index_factor: Option<f64>) -> PyRefMut<'_, Self> {
        slf.index_factor = index_factor;
        slf
    }

    #[pyo3(text_signature = "($self, convention)")]
    fn convention(mut slf: PyRefMut<'_, Self>, convention: PyCdsConvention) -> PyRefMut<'_, Self> {
        slf.convention = convention.inner;
        slf
    }

    #[pyo3(text_signature = "($self)")]
    fn build(slf: PyRefMut<'_, Self>) -> PyResult<PyCdsIndex> {
        slf.ensure_ready()?;
        let index_name = slf.index_name.clone().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "CdsIndexBuilder internal error: missing index_name after validation",
            )
        })?;
        let series = slf.series.ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "CdsIndexBuilder internal error: missing series after validation",
            )
        })?;
        let version = slf.version.ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "CdsIndexBuilder internal error: missing version after validation",
            )
        })?;
        let notional = slf.notional.ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "CdsIndexBuilder internal error: missing notional after validation",
            )
        })?;
        let fixed_coupon_bp = slf.fixed_coupon_bp.ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "CdsIndexBuilder internal error: missing fixed_coupon_bp after validation",
            )
        })?;
        let start = slf.start_date.ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "CdsIndexBuilder internal error: missing start_date after validation",
            )
        })?;
        let end = slf.maturity.ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "CdsIndexBuilder internal error: missing maturity after validation",
            )
        })?;
        let disc_curve = slf.discount_curve.clone().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "CdsIndexBuilder internal error: missing discount_curve after validation",
            )
        })?;
        let credit_curve_id = slf.credit_curve.clone().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "CdsIndexBuilder internal error: missing credit_curve after validation",
            )
        })?;

        let mut index_params = CDSIndexParams::new(&index_name, series, version, fixed_coupon_bp);
        if let Some(factor) = slf.index_factor {
            index_params = index_params.with_index_factor(factor);
        }

        let construction = CDSIndexConstructionParams::new(notional, slf.side, slf.convention);
        let credit_params = CreditParams::new(
            index_name.clone(),
            slf.recovery_rate,
            credit_curve_id.as_str(),
        );

        let index = CDSIndex::new_standard(
            slf.instrument_id.clone(),
            &index_params,
            &construction,
            start,
            end,
            &credit_params,
            disc_curve.as_str(),
            credit_curve_id.as_str(),
        );

        Ok(PyCdsIndex::new(index.map_err(core_to_py)?))
    }

    fn __repr__(&self) -> String {
        "CDSIndexBuilder(...)".to_string()
    }
}

#[pymethods]
impl PyCdsIndex {
    #[classmethod]
    #[pyo3(text_signature = "(cls, instrument_id)")]
    /// Start a fluent builder (builder-only API).
    fn builder<'py>(
        cls: &Bound<'py, PyType>,
        instrument_id: &str,
    ) -> PyResult<Py<PyCdsIndexBuilder>> {
        let py = cls.py();
        let builder = PyCdsIndexBuilder::new_with_id(InstrumentId::new(instrument_id));
        Py::new(py, builder)
    }

    /// Instrument identifier.
    ///
    /// Returns:
    ///     str: Unique identifier for the CDS index position.
    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    /// Index family name.
    ///
    /// Returns:
    ///     str: Name of the underlying CDS index.
    #[getter]
    fn index_name(&self) -> &str {
        &self.inner.index_name
    }

    /// Notional principal amount.
    ///
    /// Returns:
    ///     Money: Notional wrapped as :class:`finstack.core.money.Money`.
    #[getter]
    fn notional(&self) -> PyMoney {
        PyMoney::new(self.inner.notional)
    }

    /// Fixed coupon in basis points.
    ///
    /// Returns
    /// -------
    /// float
    ///     Coupon spread applied to premium leg in basis points.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If the internal decimal value cannot be represented as float.
    #[getter]
    fn fixed_coupon_bp(&self) -> PyResult<f64> {
        self.inner.premium.spread_bp.to_f64().ok_or_else(|| {
            PyValueError::new_err("fixed_coupon_bp: decimal to f64 conversion failed")
        })
    }

    /// Pay/receive direction for protection.
    #[getter]
    fn side(&self) -> PyCdsPayReceive {
        PyCdsPayReceive::new(self.inner.side)
    }

    /// Discount curve identifier.
    ///
    /// Returns:
    ///     str: Discount curve used for premium leg.
    #[getter]
    fn discount_curve(&self) -> String {
        self.inner.premium.discount_curve_id.as_str().to_string()
    }

    /// Credit curve identifier.
    ///
    /// Returns:
    ///     str: Hazard curve for the index constituents.
    #[getter]
    fn credit_curve(&self) -> String {
        self.inner.protection.credit_curve_id.as_str().to_string()
    }

    /// Maturity date of the index swap.
    ///
    /// Returns:
    ///     datetime.date: Maturity date converted to Python.
    #[getter]
    fn maturity(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.premium.end)
    }

    /// Instrument type enumeration.
    ///
    /// Returns:
    ///     InstrumentType: Enumeration value ``InstrumentType.CDS_INDEX``.
    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::CDSIndex)
    }

    /// Index series number.
    #[getter]
    fn series(&self) -> u16 {
        self.inner.series
    }

    /// Index version number.
    #[getter]
    fn version(&self) -> u16 {
        self.inner.version
    }

    /// Index factor (fraction of surviving notional).
    #[getter]
    fn index_factor(&self) -> f64 {
        self.inner.index_factor
    }

    /// Start date of the index swap.
    #[getter]
    fn start_date(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.premium.start)
    }

    /// Recovery rate for the protection leg.
    #[getter]
    fn recovery_rate(&self) -> f64 {
        self.inner.protection.recovery_rate
    }

    /// ISDA convention used for this index.
    #[getter]
    fn convention(&self) -> PyCdsConvention {
        PyCdsConvention::new(self.inner.convention)
    }

    /// Index constituents (empty for single-curve pricing mode).
    #[getter]
    fn constituents(&self) -> Vec<PyCdsIndexConstituent> {
        self.inner
            .constituents
            .iter()
            .map(|c| PyCdsIndexConstituent {
                credit_curve: c.credit.credit_curve_id.as_str().to_string(),
                recovery_rate: c.credit.recovery_rate,
                weight: c.weight,
                defaulted: c.defaulted,
            })
            .collect()
    }

    /// Pricing mode: "single_curve" or "constituents".
    #[getter]
    fn pricing_mode(&self) -> &'static str {
        match self.inner.pricing {
            finstack_valuations::instruments::credit_derivatives::cds_index::IndexPricing::SingleCurve => "single_curve",
            finstack_valuations::instruments::credit_derivatives::cds_index::IndexPricing::Constituents => "constituents",
        }
    }

    /// Calculate protection leg present value.
    #[pyo3(signature = (market, as_of))]
    fn pv_protection_leg(
        &self,
        py: Python<'_>,
        market: &PyMarketContext,
        as_of: Bound<'_, PyAny>,
    ) -> PyResult<PyMoney> {
        let date = py_to_date(&as_of)?;
        let pv = py
            .detach(|| self.inner.pv_protection_leg(&market.inner, date))
            .map_err(core_to_py)?;
        Ok(PyMoney::new(pv))
    }

    /// Calculate premium leg present value.
    #[pyo3(signature = (market, as_of))]
    fn pv_premium_leg(
        &self,
        py: Python<'_>,
        market: &PyMarketContext,
        as_of: Bound<'_, PyAny>,
    ) -> PyResult<PyMoney> {
        let date = py_to_date(&as_of)?;
        let pv = py
            .detach(|| self.inner.pv_premium_leg(&market.inner, date))
            .map_err(core_to_py)?;
        Ok(PyMoney::new(pv))
    }

    /// Calculate par spread in basis points.
    #[pyo3(signature = (market, as_of))]
    fn par_spread(
        &self,
        py: Python<'_>,
        market: &PyMarketContext,
        as_of: Bound<'_, PyAny>,
    ) -> PyResult<f64> {
        let date = py_to_date(&as_of)?;
        py.detach(|| self.inner.par_spread(&market.inner, date))
            .map_err(core_to_py)
    }

    /// Calculate risky PV01.
    #[pyo3(signature = (market, as_of))]
    fn risky_pv01(
        &self,
        py: Python<'_>,
        market: &PyMarketContext,
        as_of: Bound<'_, PyAny>,
    ) -> PyResult<f64> {
        let date = py_to_date(&as_of)?;
        py.detach(|| self.inner.risky_pv01(&market.inner, date))
            .map_err(core_to_py)
    }

    /// Calculate CS01 (credit spread sensitivity).
    #[pyo3(signature = (market, as_of))]
    fn cs01(
        &self,
        py: Python<'_>,
        market: &PyMarketContext,
        as_of: Bound<'_, PyAny>,
    ) -> PyResult<f64> {
        let date = py_to_date(&as_of)?;
        py.detach(|| self.inner.cs01(&market.inner, date))
            .map_err(core_to_py)
    }

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!(
            "CDSIndex(id='{}', name='{}', series={}, version={})",
            self.inner.id, self.inner.index_name, self.inner.series, self.inner.version
        ))
    }
}

impl fmt::Display for PyCdsIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CDSIndex({}, series={}, version={})",
            self.inner.index_name, self.inner.series, self.inner.version
        )
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyCdsIndex>()?;
    module.add_class::<PyCdsIndexBuilder>()?;
    module.add_class::<PyCdsIndexConstituent>()?;
    Ok(vec!["CDSIndex", "CDSIndexBuilder", "CDSIndexConstituent"])
}
