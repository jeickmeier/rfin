use crate::core::currency::PyCurrency;
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::core::money::PyMoney;
use crate::errors::PyContext;
use crate::valuations::common::PyInstrumentType;
use finstack_core::currency::Currency;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::fixed_income::bond_future::{
    BondFuture, BondFutureSpecs, DeliverableBond, Position,
};
use finstack_valuations::instruments::Attributes;
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyList, PyModule, PyType};
use pyo3::{Bound, Py, PyRef, PyRefMut};
use std::fmt;
use std::sync::Arc;

/// Bond future contract instrument.
///
/// A standardized contract to buy or sell a government bond at a specified price
/// on a future date. The contract has a basket of deliverable bonds, each with a
/// conversion factor. The short position holder chooses which bond to deliver
/// (typically the Cheapest-to-Deliver or CTD bond).
///
/// Examples
/// --------
/// Create a UST 10-year future::
///
///     from finstack import Money, Currency, Date
///     from finstack.valuations.instruments import BondFuture
///
///     future = (
///         BondFuture.builder("TYH5")
///         .notional(1_000_000, "USD")
///         .expiry_date(Date(2025, 3, 20))
///         .delivery_start(Date(2025, 3, 21))
///         .delivery_end(Date(2025, 3, 31))
///         .quoted_price(125.50)
///         .position("long")
///         .contract_specs(BondFuture.ust_10y_specs())
///         .deliverable_basket([
///             {"bond_id": "US912828XG33", "conversion_factor": 0.8234},
///         ])
///         .ctd_bond_id("US912828XG33")
///         .disc_id("USD-TREASURY")
///         .build()
///     )
///
/// See Also
/// --------
/// Bond : Plain vanilla fixed income bond
/// InterestRateFuture : Short-term interest rate futures
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "BondFuture",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyBondFuture {
    pub(crate) inner: Arc<BondFuture>,
}

impl PyBondFuture {
    pub(crate) fn new(inner: BondFuture) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

/// Bond future contract specifications.
///
/// Defines standard parameters for a bond future contract including contract size,
/// tick size, and notional bond parameters for conversion factor calculations.
///
/// Examples
/// --------
/// Use standard market specifications::
///
///     ust_10y = BondFuture.ust_10y_specs()
///     bund = BondFuture.bund_specs()
///     gilt = BondFuture.gilt_specs()
#[pyclass(module = "finstack.valuations.instruments", name = "BondFutureSpecs")]
#[derive(Clone, Debug)]
pub struct PyBondFutureSpecs {
    pub(crate) inner: BondFutureSpecs,
}

#[pymethods]
impl PyBondFutureSpecs {
    /// Create custom bond future specifications.
    ///
    /// Parameters
    /// ----------
    /// contract_size : float
    ///     Face value of a single contract (e.g., 100,000 for UST)
    /// tick_size : float
    ///     Minimum price increment (e.g., 0.03125 = 1/32 for UST)
    /// tick_value : float
    ///     Value of one tick in currency units
    /// standard_coupon : float
    ///     Standard coupon rate for conversion factor calculation
    /// standard_maturity_years : float
    ///     Standard maturity in years
    /// settlement_days : int, optional
    ///     Number of business days for settlement after expiry (default: 2)
    /// calendar_id : str, optional
    ///     Holiday calendar identifier (default: "nyse")
    #[new]
    #[pyo3(signature = (contract_size, tick_size, tick_value, standard_coupon, standard_maturity_years, settlement_days=2, calendar_id=String::from("nyse")))]
    fn new_py(
        contract_size: f64,
        tick_size: f64,
        tick_value: f64,
        standard_coupon: f64,
        standard_maturity_years: f64,
        settlement_days: u32,
        calendar_id: String,
    ) -> Self {
        Self {
            inner: BondFutureSpecs {
                contract_size,
                tick_size,
                tick_value,
                standard_coupon,
                standard_maturity_years,
                settlement_days,
                calendar_id,
            },
        }
    }

    /// Contract size (face value per contract).
    #[getter]
    fn contract_size(&self) -> f64 {
        self.inner.contract_size
    }

    /// Tick size (minimum price increment).
    #[getter]
    fn tick_size(&self) -> f64 {
        self.inner.tick_size
    }

    /// Tick value in currency units.
    #[getter]
    fn tick_value(&self) -> f64 {
        self.inner.tick_value
    }

    /// Standard coupon rate for conversion factor calculation.
    #[getter]
    fn standard_coupon(&self) -> f64 {
        self.inner.standard_coupon
    }

    /// Standard maturity in years.
    #[getter]
    fn standard_maturity_years(&self) -> f64 {
        self.inner.standard_maturity_years
    }

    /// Settlement days after expiry.
    #[getter]
    fn settlement_days(&self) -> u32 {
        self.inner.settlement_days
    }

    /// Holiday calendar identifier.
    #[getter]
    fn calendar_id(&self) -> String {
        self.inner.calendar_id.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "BondFutureSpecs(contract_size={}, tick_size={}, standard_coupon={})",
            self.inner.contract_size, self.inner.tick_size, self.inner.standard_coupon
        )
    }
}

#[pyclass(
    module = "finstack.valuations.instruments",
    name = "BondFutureBuilder",
    unsendable
)]
pub struct PyBondFutureBuilder {
    instrument_id: InstrumentId,
    pending_notional_amount: Option<f64>,
    pending_currency: Option<Currency>,
    expiry_date: Option<time::Date>,
    delivery_start: Option<time::Date>,
    delivery_end: Option<time::Date>,
    quoted_price: Option<f64>,
    position: Position,
    contract_specs: BondFutureSpecs,
    deliverable_basket: Vec<DeliverableBond>,
    ctd_bond_id: Option<InstrumentId>,
    discount_curve: Option<CurveId>,
}

impl PyBondFutureBuilder {
    fn new_with_id(id: InstrumentId) -> Self {
        Self {
            instrument_id: id,
            pending_notional_amount: None,
            pending_currency: None,
            expiry_date: None,
            delivery_start: None,
            delivery_end: None,
            quoted_price: None,
            position: Position::Long,
            contract_specs: BondFutureSpecs::default(),
            deliverable_basket: Vec::new(),
            ctd_bond_id: None,
            discount_curve: None,
        }
    }

    fn notional_money(&self) -> Option<Money> {
        match (self.pending_notional_amount, self.pending_currency) {
            (Some(amount), Some(currency)) => Some(Money::new(amount, currency)),
            _ => None,
        }
    }

    fn parse_currency(value: &Bound<'_, PyAny>) -> PyResult<Currency> {
        if let Ok(py_ccy) = value.extract::<PyRef<PyCurrency>>() {
            Ok(py_ccy.inner)
        } else if let Ok(code) = value.extract::<&str>() {
            code.parse::<Currency>()
                .map_err(|_| PyValueError::new_err("Invalid currency code"))
        } else {
            Err(PyTypeError::new_err("currency() expects str or Currency"))
        }
    }

    fn parse_position(value: &Bound<'_, PyAny>) -> PyResult<Position> {
        if let Ok(name) = value.extract::<&str>() {
            match name.to_lowercase().as_str() {
                "long" => Ok(Position::Long),
                "short" => Ok(Position::Short),
                other => Err(PyValueError::new_err(format!(
                    "position() expects 'long' or 'short', got '{}'",
                    other
                ))),
            }
        } else {
            Err(PyTypeError::new_err("position() expects str"))
        }
    }
}

#[pymethods]
impl PyBondFutureBuilder {
    #[new]
    #[pyo3(text_signature = "(instrument_id)")]
    fn new_py(instrument_id: &str) -> Self {
        Self::new_with_id(InstrumentId::new(instrument_id))
    }

    /// Set notional amount and currency.
    ///
    /// Parameters
    /// ----------
    /// amount : float
    ///     Notional amount
    /// currency : str or Currency
    ///     Currency code (e.g., "USD") or Currency object
    #[pyo3(text_signature = "($self, amount, currency)")]
    fn notional<'py>(
        mut slf: PyRefMut<'py, Self>,
        amount: f64,
        currency: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        if amount <= 0.0 {
            return Err(PyValueError::new_err("notional must be positive"));
        }
        slf.pending_notional_amount = Some(amount);
        slf.pending_currency = Some(Self::parse_currency(&currency)?);
        Ok(slf)
    }

    /// Set expiry date (last trading day).
    #[pyo3(text_signature = "($self, date)")]
    fn expiry_date<'py>(
        mut slf: PyRefMut<'py, Self>,
        date: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.expiry_date = Some(py_to_date(&date).context("expiry_date")?);
        Ok(slf)
    }

    /// Set first delivery date.
    #[pyo3(text_signature = "($self, date)")]
    fn delivery_start<'py>(
        mut slf: PyRefMut<'py, Self>,
        date: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.delivery_start = Some(py_to_date(&date).context("delivery_start")?);
        Ok(slf)
    }

    /// Set last delivery date.
    #[pyo3(text_signature = "($self, date)")]
    fn delivery_end<'py>(
        mut slf: PyRefMut<'py, Self>,
        date: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.delivery_end = Some(py_to_date(&date).context("delivery_end")?);
        Ok(slf)
    }

    /// Set quoted futures price (e.g., 125.50 for 125-16/32).
    #[pyo3(text_signature = "($self, price)")]
    fn quoted_price(mut slf: PyRefMut<'_, Self>, price: f64) -> PyResult<PyRefMut<'_, Self>> {
        if price <= 0.0 {
            return Err(PyValueError::new_err("quoted_price must be positive"));
        }
        slf.quoted_price = Some(price);
        Ok(slf)
    }

    /// Set position side (long or short).
    ///
    /// Parameters
    /// ----------
    /// position : str
    ///     Either "long" or "short"
    #[pyo3(text_signature = "($self, position)")]
    fn position<'py>(
        mut slf: PyRefMut<'py, Self>,
        position: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.position = Self::parse_position(&position)?;
        Ok(slf)
    }

    /// Set contract specifications.
    ///
    /// Parameters
    /// ----------
    /// specs : BondFutureSpecs
    ///     Contract specifications (use BondFuture.ust_10y_specs(), etc.)
    #[pyo3(text_signature = "($self, specs)")]
    fn contract_specs<'py>(
        mut slf: PyRefMut<'py, Self>,
        specs: PyRef<PyBondFutureSpecs>,
    ) -> PyRefMut<'py, Self> {
        slf.contract_specs = specs.inner.clone();
        slf
    }

    /// Set deliverable basket of bonds with conversion factors.
    ///
    /// Parameters
    /// ----------
    /// basket : list of dict
    ///     Each dict must have "bond_id" (str) and "conversion_factor" (float)
    #[pyo3(text_signature = "($self, basket)")]
    fn deliverable_basket<'py>(
        mut slf: PyRefMut<'py, Self>,
        basket: Bound<'py, PyList>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let mut bonds = Vec::new();
        for item in basket.iter() {
            let dict = item.downcast::<pyo3::types::PyDict>()?;
            let bond_id = dict
                .get_item("bond_id")?
                .ok_or_else(|| PyValueError::new_err("Each basket item must have 'bond_id'"))?
                .extract::<String>()?;
            let conversion_factor = dict
                .get_item("conversion_factor")?
                .ok_or_else(|| {
                    PyValueError::new_err("Each basket item must have 'conversion_factor'")
                })?
                .extract::<f64>()?;

            bonds.push(DeliverableBond {
                bond_id: InstrumentId::new(&bond_id),
                conversion_factor,
            });
        }
        slf.deliverable_basket = bonds;
        Ok(slf)
    }

    /// Set Cheapest-to-Deliver (CTD) bond identifier.
    ///
    /// Parameters
    /// ----------
    /// bond_id : str
    ///     Identifier for the CTD bond (must exist in deliverable basket)
    #[pyo3(text_signature = "($self, bond_id)")]
    fn ctd_bond_id(mut slf: PyRefMut<'_, Self>, bond_id: String) -> PyRefMut<'_, Self> {
        slf.ctd_bond_id = Some(InstrumentId::new(&bond_id));
        slf
    }

    /// Set discount curve identifier.
    #[pyo3(text_signature = "($self, curve_id)")]
    fn disc_id(mut slf: PyRefMut<'_, Self>, curve_id: String) -> PyRefMut<'_, Self> {
        slf.discount_curve = Some(CurveId::new(&curve_id));
        slf
    }

    /// Build the BondFuture instrument.
    ///
    /// Returns
    /// -------
    /// BondFuture
    ///     Validated bond future instrument
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If required fields are missing or validation fails
    #[pyo3(text_signature = "($self)")]
    fn build(slf: PyRefMut<'_, Self>) -> PyResult<PyBondFuture> {
        let notional = slf
            .notional_money()
            .ok_or_else(|| PyValueError::new_err("Both notional() must be provided"))?;

        let expiry_date = slf
            .expiry_date
            .ok_or_else(|| PyValueError::new_err("expiry_date() must be provided"))?;

        let delivery_start = slf
            .delivery_start
            .ok_or_else(|| PyValueError::new_err("delivery_start() must be provided"))?;

        let delivery_end = slf
            .delivery_end
            .ok_or_else(|| PyValueError::new_err("delivery_end() must be provided"))?;

        let quoted_price = slf
            .quoted_price
            .ok_or_else(|| PyValueError::new_err("quoted_price() must be provided"))?;

        let ctd_bond_id = slf
            .ctd_bond_id
            .clone()
            .ok_or_else(|| PyValueError::new_err("ctd_bond_id() must be provided"))?;

        let discount_curve_id = slf
            .discount_curve
            .clone()
            .ok_or_else(|| PyValueError::new_err("disc_id() must be provided"))?;

        if slf.deliverable_basket.is_empty() {
            return Err(PyValueError::new_err(
                "deliverable_basket() cannot be empty",
            ));
        }

        let bond_future = BondFuture {
            id: slf.instrument_id.clone(),
            notional,
            expiry_date,
            delivery_start,
            delivery_end,
            quoted_price,
            position: slf.position,
            contract_specs: slf.contract_specs.clone(),
            deliverable_basket: slf.deliverable_basket.clone(),
            ctd_bond_id,
            ctd_bond: None,
            discount_curve_id,
            attributes: Attributes::new(),
        };

        Ok(PyBondFuture::new(bond_future))
    }
}

#[pymethods]
impl PyBondFuture {
    /// Create a builder for constructing BondFuture instruments.
    ///
    /// Parameters
    /// ----------
    /// instrument_id : str
    ///     Unique identifier for the contract (e.g., "TYH5")
    ///
    /// Returns
    /// -------
    /// BondFutureBuilder
    ///     Builder instance for fluent construction
    #[classmethod]
    #[pyo3(text_signature = "(cls, instrument_id)")]
    fn builder(_cls: &Bound<'_, PyType>, instrument_id: &str) -> PyBondFutureBuilder {
        PyBondFutureBuilder::new_with_id(InstrumentId::new(instrument_id))
    }

    /// UST 10-year futures contract specifications.
    ///
    /// Returns standard specifications for U.S. Treasury 10-Year Note Futures (CBOT):
    /// - Contract size: $100,000
    /// - Tick size: 1/32 of a point (0.03125)
    /// - Tick value: $31.25
    /// - Standard coupon: 6%
    /// - Standard maturity: 10 years
    /// - Settlement: T+2 business days
    #[classmethod]
    #[pyo3(text_signature = "(cls)")]
    fn ust_10y_specs(_cls: &Bound<'_, PyType>) -> PyBondFutureSpecs {
        PyBondFutureSpecs {
            inner: BondFutureSpecs::ust_10y(),
        }
    }

    /// UST 5-year futures contract specifications.
    #[classmethod]
    #[pyo3(text_signature = "(cls)")]
    fn ust_5y_specs(_cls: &Bound<'_, PyType>) -> PyBondFutureSpecs {
        PyBondFutureSpecs {
            inner: BondFutureSpecs::ust_5y(),
        }
    }

    /// UST 2-year futures contract specifications.
    #[classmethod]
    #[pyo3(text_signature = "(cls)")]
    fn ust_2y_specs(_cls: &Bound<'_, PyType>) -> PyBondFutureSpecs {
        PyBondFutureSpecs {
            inner: BondFutureSpecs::ust_2y(),
        }
    }

    /// German Bund futures contract specifications (Eurex).
    #[classmethod]
    #[pyo3(text_signature = "(cls)")]
    fn bund_specs(_cls: &Bound<'_, PyType>) -> PyBondFutureSpecs {
        PyBondFutureSpecs {
            inner: BondFutureSpecs::bund(),
        }
    }

    /// UK Gilt futures contract specifications (LIFFE).
    #[classmethod]
    #[pyo3(text_signature = "(cls)")]
    fn gilt_specs(_cls: &Bound<'_, PyType>) -> PyBondFutureSpecs {
        PyBondFutureSpecs {
            inner: BondFutureSpecs::gilt(),
        }
    }

    /// Instrument identifier.
    #[getter]
    fn id(&self) -> String {
        self.inner.id.as_str().to_string()
    }

    /// Notional exposure.
    #[getter]
    fn notional(&self) -> PyMoney {
        PyMoney::new(self.inner.notional)
    }

    /// Future expiry date (last trading day).
    #[getter]
    fn expiry_date(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.expiry_date)
    }

    /// First delivery date.
    #[getter]
    fn delivery_start(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.delivery_start)
    }

    /// Last delivery date.
    #[getter]
    fn delivery_end(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.delivery_end)
    }

    /// Quoted futures price.
    #[getter]
    fn quoted_price(&self) -> f64 {
        self.inner.quoted_price
    }

    /// Position side ("long" or "short").
    #[getter]
    fn position(&self) -> String {
        match self.inner.position {
            Position::Long => "long".to_string(),
            Position::Short => "short".to_string(),
        }
    }

    /// Discount curve identifier.
    #[getter]
    fn discount_curve(&self) -> String {
        self.inner.discount_curve_id.as_str().to_string()
    }

    /// Instrument type enum.
    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::BondFuture)
    }

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!(
            "BondFuture(id='{}', position='{}', expiry='{}')",
            self.inner.id,
            self.position(),
            self.inner.expiry_date
        ))
    }
}

impl fmt::Display for PyBondFuture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BondFuture({})", self.inner.id)
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyBondFuture>()?;
    module.add_class::<PyBondFutureBuilder>()?;
    module.add_class::<PyBondFutureSpecs>()?;
    Ok(vec!["BondFuture", "BondFutureBuilder", "BondFutureSpecs"])
}
