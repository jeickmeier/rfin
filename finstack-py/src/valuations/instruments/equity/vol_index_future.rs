//! Python bindings for VolatilityIndexFuture.

use crate::core::currency::PyCurrency;
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::core::market_data::PyMarketContext;
use crate::core::money::PyMoney;
use crate::errors::{core_to_py, PyContext};
use crate::valuations::common::PyInstrumentType;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::equity::vol_index_future::{
    VolIndexContractSpecs, VolatilityIndexFuture,
};
use finstack_valuations::instruments::rates::ir_future::Position;
use finstack_valuations::prelude::Instrument;
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::{Bound, Py, PyRef, PyRefMut};
use std::fmt;
use std::sync::Arc;

fn parse_position(label: Option<&str>) -> PyResult<Position> {
    match label {
        None => Ok(Position::Long),
        Some(s) => s.parse().map_err(|e: String| PyValueError::new_err(e)),
    }
}

/// Volatility index future wrapper (e.g., VIX futures).
///
/// Parameters
/// ----------
/// instrument_id : str
///     Unique identifier for the instrument.
/// notional : Money
///     Notional amount (e.g., $100,000 USD).
/// quoted_price : float
///     Quoted future price (e.g., 18.50 for VIX at 18.50).
/// expiry : date
///     Expiry date of the future.
/// discount_curve : str
///     ID of the discount curve for NPV calculations.
/// vol_index_curve : str
///     ID of the volatility index curve for forward levels.
/// position : str, optional
///     Position type: "long" (default) or "short".
/// multiplier : float, optional
///     Contract multiplier (default: 1000 for VIX).
/// tick_size : float, optional
///     Minimum price movement (default: 0.05).
/// tick_value : float, optional
///     Dollar value per tick (default: 50).
/// index_id : str, optional
///     Index identifier (default: "VIX").
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "VolatilityIndexFuture",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyVolatilityIndexFuture {
    pub(crate) inner: Arc<VolatilityIndexFuture>,
}

impl PyVolatilityIndexFuture {
    pub(crate) fn new(inner: VolatilityIndexFuture) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

#[pyclass(
    module = "finstack.valuations.instruments",
    name = "VolatilityIndexFutureBuilder",
    unsendable
)]
pub struct PyVolatilityIndexFutureBuilder {
    instrument_id: InstrumentId,
    pending_notional_amount: Option<f64>,
    pending_currency: Option<finstack_core::currency::Currency>,
    quoted_price: Option<f64>,
    expiry: Option<time::Date>,
    discount_curve_id: Option<CurveId>,
    vol_index_curve_id: Option<CurveId>,
    position: Position,
    multiplier: f64,
    tick_size: f64,
    tick_value: f64,
    index_id: String,
}

impl PyVolatilityIndexFutureBuilder {
    fn new_with_id(id: InstrumentId) -> Self {
        Self {
            instrument_id: id,
            pending_notional_amount: None,
            pending_currency: None,
            quoted_price: None,
            expiry: None,
            discount_curve_id: None,
            vol_index_curve_id: None,
            position: Position::Long,
            multiplier: 1000.0,
            tick_size: 0.05,
            tick_value: 50.0,
            index_id: "VIX".to_string(),
        }
    }

    fn notional_money(&self) -> Option<Money> {
        match (self.pending_notional_amount, self.pending_currency) {
            (Some(amount), Some(currency)) => Some(Money::new(amount, currency)),
            _ => None,
        }
    }

    fn ensure_ready(&self) -> PyResult<()> {
        if self.notional_money().is_none() {
            return Err(PyValueError::new_err(
                "Both notional() and currency() must be provided before build().",
            ));
        }
        if self.quoted_price.is_none() {
            return Err(PyValueError::new_err("quoted_price() is required."));
        }
        if self.expiry.is_none() {
            return Err(PyValueError::new_err("expiry() is required."));
        }
        if self.discount_curve_id.is_none() {
            return Err(PyValueError::new_err("disc_id() is required."));
        }
        if self.vol_index_curve_id.is_none() {
            return Err(PyValueError::new_err("vol_index_curve_id() is required."));
        }
        Ok(())
    }

    fn parse_currency(value: &Bound<'_, PyAny>) -> PyResult<finstack_core::currency::Currency> {
        if let Ok(py_ccy) = value.extract::<PyRef<PyCurrency>>() {
            Ok(py_ccy.inner)
        } else if let Ok(code) = value.extract::<&str>() {
            code.parse::<finstack_core::currency::Currency>()
                .map_err(|_| PyValueError::new_err("Invalid currency code"))
        } else {
            Err(PyTypeError::new_err("currency() expects str or Currency"))
        }
    }
}

#[pymethods]
impl PyVolatilityIndexFutureBuilder {
    #[new]
    #[pyo3(text_signature = "(instrument_id)")]
    fn new_py(instrument_id: &str) -> Self {
        Self::new_with_id(InstrumentId::new(instrument_id))
    }

    #[pyo3(text_signature = "($self, amount)")]
    fn notional(mut slf: PyRefMut<'_, Self>, amount: f64) -> PyResult<PyRefMut<'_, Self>> {
        if amount <= 0.0 {
            return Err(PyValueError::new_err("notional must be positive"));
        }
        slf.pending_notional_amount = Some(amount);
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, currency)")]
    fn currency<'py>(
        mut slf: PyRefMut<'py, Self>,
        currency: &Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.pending_currency = Some(Self::parse_currency(currency)?);
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, money)")]
    fn money<'py>(mut slf: PyRefMut<'py, Self>, money: PyRef<'py, PyMoney>) -> PyRefMut<'py, Self> {
        slf.pending_notional_amount = Some(money.inner.amount());
        slf.pending_currency = Some(money.inner.currency());
        slf
    }

    #[pyo3(text_signature = "($self, quoted_price)")]
    fn quoted_price(mut slf: PyRefMut<'_, Self>, quoted_price: f64) -> PyRefMut<'_, Self> {
        slf.quoted_price = Some(quoted_price);
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

    #[pyo3(text_signature = "($self, curve_id)")]
    fn disc_id(mut slf: PyRefMut<'_, Self>, curve_id: String) -> PyRefMut<'_, Self> {
        slf.discount_curve_id = Some(CurveId::new(curve_id.as_str()));
        slf
    }

    #[pyo3(text_signature = "($self, curve_id)")]
    fn vol_index_curve_id(mut slf: PyRefMut<'_, Self>, curve_id: String) -> PyRefMut<'_, Self> {
        slf.vol_index_curve_id = Some(CurveId::new(curve_id.as_str()));
        slf
    }

    #[pyo3(text_signature = "($self, position)")]
    fn position(
        mut slf: PyRefMut<'_, Self>,
        position: Option<String>,
    ) -> PyResult<PyRefMut<'_, Self>> {
        slf.position = parse_position(position.as_deref())?;
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, multiplier)")]
    fn multiplier(mut slf: PyRefMut<'_, Self>, multiplier: f64) -> PyRefMut<'_, Self> {
        slf.multiplier = multiplier;
        slf
    }

    #[pyo3(text_signature = "($self, tick_size)")]
    fn tick_size(mut slf: PyRefMut<'_, Self>, tick_size: f64) -> PyRefMut<'_, Self> {
        slf.tick_size = tick_size;
        slf
    }

    #[pyo3(text_signature = "($self, tick_value)")]
    fn tick_value(mut slf: PyRefMut<'_, Self>, tick_value: f64) -> PyRefMut<'_, Self> {
        slf.tick_value = tick_value;
        slf
    }

    #[pyo3(text_signature = "($self, index_id)")]
    fn index_id(mut slf: PyRefMut<'_, Self>, index_id: String) -> PyRefMut<'_, Self> {
        slf.index_id = index_id;
        slf
    }

    #[pyo3(text_signature = "($self)")]
    fn build(slf: PyRefMut<'_, Self>) -> PyResult<PyVolatilityIndexFuture> {
        slf.ensure_ready()?;
        let notional = slf.notional_money().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "VolatilityIndexFutureBuilder internal error: missing notional after validation",
            )
        })?;
        let quoted_price = slf.quoted_price.ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "VolatilityIndexFutureBuilder internal error: missing quoted_price after validation",
            )
        })?;
        let expiry = slf.expiry.ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "VolatilityIndexFutureBuilder internal error: missing expiry after validation",
            )
        })?;
        let discount_curve_id = slf.discount_curve_id.clone().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "VolatilityIndexFutureBuilder internal error: missing discount curve after validation",
            )
        })?;
        let vol_index_curve_id = slf.vol_index_curve_id.clone().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "VolatilityIndexFutureBuilder internal error: missing vol index curve after validation",
            )
        })?;

        let specs = VolIndexContractSpecs {
            multiplier: slf.multiplier,
            tick_size: slf.tick_size,
            tick_value: slf.tick_value,
            index_id: slf.index_id.clone(),
        };

        let future = VolatilityIndexFuture::builder()
            .id(slf.instrument_id.clone())
            .notional(notional)
            .quoted_price(quoted_price)
            .expiry(expiry)
            .discount_curve_id(discount_curve_id)
            .vol_index_curve_id(vol_index_curve_id)
            .position(slf.position)
            .contract_specs(specs)
            .attributes(Default::default())
            .build()
            .map_err(core_to_py)?;

        Ok(PyVolatilityIndexFuture::new(future))
    }

    fn __repr__(&self) -> String {
        "VolatilityIndexFutureBuilder(...)".to_string()
    }
}

#[pymethods]
impl PyVolatilityIndexFuture {
    #[classmethod]
    #[pyo3(text_signature = "(cls, instrument_id)")]
    /// Start a fluent builder (builder-only API).
    fn builder<'py>(
        cls: &Bound<'py, PyType>,
        instrument_id: &str,
    ) -> PyResult<Py<PyVolatilityIndexFutureBuilder>> {
        let py = cls.py();
        let builder = PyVolatilityIndexFutureBuilder::new_with_id(InstrumentId::new(instrument_id));
        Py::new(py, builder)
    }

    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    #[getter]
    fn notional(&self) -> PyMoney {
        PyMoney::new(self.inner.notional)
    }

    #[getter]
    fn quoted_price(&self) -> f64 {
        self.inner.quoted_price
    }

    #[getter]
    fn expiry(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.expiry)
    }

    #[getter]
    fn position(&self) -> &'static str {
        match self.inner.position {
            Position::Long => "long",
            Position::Short => "short",
            _ => unreachable!("unknown Position variant"),
        }
    }

    #[getter]
    fn discount_curve_id(&self) -> String {
        self.inner.discount_curve_id.as_str().to_string()
    }

    #[getter]
    fn vol_index_curve_id(&self) -> String {
        self.inner.vol_index_curve_id.as_str().to_string()
    }

    #[getter]
    fn multiplier(&self) -> f64 {
        self.inner.contract_specs.multiplier
    }

    #[getter]
    fn tick_size(&self) -> f64 {
        self.inner.contract_specs.tick_size
    }

    #[getter]
    fn tick_value(&self) -> f64 {
        self.inner.contract_specs.tick_value
    }

    #[getter]
    fn index_id(&self) -> &str {
        &self.inner.contract_specs.index_id
    }

    /// Contract specifications as a structured object.
    #[getter]
    fn contract_specs(&self) -> PyVolIndexContractSpecs {
        PyVolIndexContractSpecs {
            inner: VolIndexContractSpecs {
                multiplier: self.inner.contract_specs.multiplier,
                tick_size: self.inner.contract_specs.tick_size,
                tick_value: self.inner.contract_specs.tick_value,
                index_id: self.inner.contract_specs.index_id.clone(),
            },
        }
    }

    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::VolatilityIndexFuture)
    }

    #[pyo3(signature = (market, as_of))]
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

    #[pyo3(signature = (market))]
    fn npv_raw(&self, py: Python<'_>, market: &PyMarketContext) -> PyResult<f64> {
        py.detach(|| self.inner.npv_raw(&market.inner))
            .map_err(core_to_py)
    }

    #[pyo3(signature = (market))]
    fn forward_vol(&self, py: Python<'_>, market: &PyMarketContext) -> PyResult<f64> {
        py.detach(|| self.inner.forward_vol(&market.inner))
            .map_err(core_to_py)
    }

    fn delta_vol(&self) -> f64 {
        self.inner.delta_vol()
    }

    fn num_contracts(&self) -> f64 {
        self.inner.num_contracts()
    }

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!(
            "VolatilityIndexFuture(id='{}', price={:.2})",
            self.inner.id, self.inner.quoted_price
        ))
    }
}

impl fmt::Display for PyVolatilityIndexFuture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "VolatilityIndexFuture({}, price={:.2})",
            self.inner.id, self.inner.quoted_price
        )
    }
}

// ============================================================================
// CONTRACT SPECS WRAPPER
// ============================================================================

/// Volatility index contract specifications.
///
/// Parameters
/// ----------
/// multiplier : float
///     Contract multiplier (e.g., 1000 for VIX futures).
/// tick_size : float
///     Minimum price movement.
/// tick_value : float
///     Dollar value per tick.
/// index_id : str
///     Volatility index identifier (e.g., "VIX").
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "VolIndexContractSpecs",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyVolIndexContractSpecs {
    pub(crate) inner: VolIndexContractSpecs,
}

#[pymethods]
impl PyVolIndexContractSpecs {
    #[new]
    #[pyo3(text_signature = "(multiplier, tick_size, tick_value, index_id)")]
    fn new_py(multiplier: f64, tick_size: f64, tick_value: f64, index_id: String) -> Self {
        Self {
            inner: VolIndexContractSpecs {
                multiplier,
                tick_size,
                tick_value,
                index_id,
            },
        }
    }

    #[getter]
    fn multiplier(&self) -> f64 {
        self.inner.multiplier
    }

    #[getter]
    fn tick_size(&self) -> f64 {
        self.inner.tick_size
    }

    #[getter]
    fn tick_value(&self) -> f64 {
        self.inner.tick_value
    }

    #[getter]
    fn index_id(&self) -> &str {
        &self.inner.index_id
    }

    fn __repr__(&self) -> String {
        format!(
            "VolIndexContractSpecs(multiplier={}, tick_size={}, tick_value={}, index_id='{}')",
            self.inner.multiplier, self.inner.tick_size, self.inner.tick_value, self.inner.index_id
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyVolatilityIndexFuture>()?;
    module.add_class::<PyVolatilityIndexFutureBuilder>()?;
    module.add_class::<PyVolIndexContractSpecs>()?;
    Ok(vec![
        "VolatilityIndexFuture",
        "VolatilityIndexFutureBuilder",
        "VolIndexContractSpecs",
    ])
}
