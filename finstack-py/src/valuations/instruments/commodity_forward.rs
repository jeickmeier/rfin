//! Python bindings for CommodityForward instrument.

use crate::core::common::args::CurrencyArg;
use crate::core::currency::PyCurrency;
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::valuations::common::PyInstrumentType;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::commodity::commodity_forward::{
    CommodityForward, SettlementType,
};
use finstack_valuations::instruments::Attributes;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyType};
use pyo3::{Bound, Py};
use std::fmt;

/// Commodity forward or futures contract.
///
/// Represents a commitment to buy or sell a commodity at a specified future
/// date at a predetermined price. Can be physically settled (delivery) or
/// cash settled (price difference).
///
/// Examples:
///     >>> forward = CommodityForward.create(
///     ...     "WTI-FWD-2025M03",
///     ...     commodity_type="Energy",
///     ...     ticker="CL",
///     ...     quantity=1000.0,
///     ...     unit="BBL",
///     ...     settlement_date=Date(2025, 3, 15),
///     ...     currency="USD",
///     ...     forward_curve_id="WTI-FORWARD",
///     ...     discount_curve_id="USD-OIS"
///     ... )
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "CommodityForward",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyCommodityForward {
    pub(crate) inner: CommodityForward,
}

impl PyCommodityForward {
    pub(crate) fn new(inner: CommodityForward) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyCommodityForward {
    #[classmethod]
    #[pyo3(
        text_signature = "(cls, instrument_id, *, commodity_type, ticker, quantity, unit, settlement_date, currency, forward_curve_id, discount_curve_id, multiplier=1.0, quoted_price=None, spot_price_id=None, settlement_type=None, exchange=None, contract_month=None)"
    )]
    #[pyo3(
        signature = (
            instrument_id,
            *,
            commodity_type,
            ticker,
            quantity,
            unit,
            settlement_date,
            currency,
            forward_curve_id,
            discount_curve_id,
            multiplier = 1.0,
            quoted_price = None,
            spot_price_id = None,
            settlement_type = None,
            exchange = None,
            contract_month = None
        )
    )]
    /// Create a commodity forward contract.
    ///
    /// Args:
    ///     instrument_id: Unique identifier for this instrument.
    ///     commodity_type: Commodity type (e.g., "Energy", "Metal", "Agricultural").
    ///     ticker: Ticker or symbol (e.g., "CL" for WTI, "GC" for Gold).
    ///     quantity: Contract quantity in units.
    ///     unit: Unit of measurement (e.g., "BBL", "MT", "OZ").
    ///     settlement_date: Settlement/delivery date.
    ///     currency: Currency for pricing.
    ///     forward_curve_id: Forward/futures curve ID for price interpolation.
    ///     discount_curve_id: Discount curve ID.
    ///     multiplier: Contract multiplier (default 1.0).
    ///     quoted_price: Optional quoted forward price (overrides curve lookup).
    ///     spot_price_id: Optional spot price ID (for delta calculations).
    ///     settlement_type: Settlement type ("physical" or "cash").
    ///     exchange: Optional exchange identifier (e.g., "NYMEX", "ICE").
    ///     contract_month: Optional contract month (e.g., "2025M03").
    ///
    /// Returns:
    ///     CommodityForward: Configured commodity forward instrument.
    fn create(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        commodity_type: &str,
        ticker: &str,
        quantity: f64,
        unit: &str,
        settlement_date: Bound<'_, PyAny>,
        currency: Bound<'_, PyAny>,
        forward_curve_id: &str,
        discount_curve_id: &str,
        multiplier: f64,
        quoted_price: Option<f64>,
        spot_price_id: Option<&str>,
        settlement_type: Option<&str>,
        exchange: Option<&str>,
        contract_month: Option<&str>,
    ) -> PyResult<Self> {
        use crate::errors::PyContext;

        let id = InstrumentId::new(instrument_id.extract::<&str>().context("instrument_id")?);
        let CurrencyArg(ccy) = currency.extract().context("currency")?;
        let settle_date = py_to_date(&settlement_date).context("settlement_date")?;

        let settlement_type_enum = match settlement_type {
            Some("physical") | Some("Physical") => Some(SettlementType::Physical),
            Some("cash") | Some("Cash") => Some(SettlementType::Cash),
            None => None,
            Some(other) => {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "Invalid settlement_type: '{}'. Must be 'physical' or 'cash'",
                    other
                )));
            }
        };

        let mut builder = CommodityForward::builder()
            .id(id)
            .commodity_type(commodity_type.to_string())
            .ticker(ticker.to_string())
            .quantity(quantity)
            .unit(unit.to_string())
            .multiplier(multiplier)
            .settlement_date(settle_date)
            .currency(ccy)
            .forward_curve_id(CurveId::new(forward_curve_id))
            .discount_curve_id(CurveId::new(discount_curve_id))
            .attributes(Attributes::new());

        if let Some(st) = settlement_type_enum {
            builder = builder.settlement_type_opt(Some(st));
        }
        if let Some(qp) = quoted_price {
            builder = builder.quoted_price_opt(Some(qp));
        }
        if let Some(sp) = spot_price_id {
            builder = builder.spot_price_id_opt(Some(sp.to_string()));
        }
        if let Some(ex) = exchange {
            builder = builder.exchange_opt(Some(ex.to_string()));
        }
        if let Some(cm) = contract_month {
            builder = builder.contract_month_opt(Some(cm.to_string()));
        }

        let forward = builder
            .build()
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("{}", e)))?;

        Ok(Self::new(forward))
    }

    /// Instrument identifier.
    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    /// Commodity type (e.g., "Energy", "Metal").
    #[getter]
    fn commodity_type(&self) -> &str {
        &self.inner.commodity_type
    }

    /// Ticker symbol.
    #[getter]
    fn ticker(&self) -> &str {
        &self.inner.ticker
    }

    /// Contract quantity.
    #[getter]
    fn quantity(&self) -> f64 {
        self.inner.quantity
    }

    /// Unit of measurement.
    #[getter]
    fn unit(&self) -> &str {
        &self.inner.unit
    }

    /// Contract multiplier.
    #[getter]
    fn multiplier(&self) -> f64 {
        self.inner.multiplier
    }

    /// Settlement date.
    #[getter]
    fn settlement_date(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.settlement_date)
    }

    /// Currency.
    #[getter]
    fn currency(&self) -> PyCurrency {
        PyCurrency::new(self.inner.currency)
    }

    /// Optional quoted forward price.
    #[getter]
    fn quoted_price(&self) -> Option<f64> {
        self.inner.quoted_price
    }

    /// Forward curve ID.
    #[getter]
    fn forward_curve_id(&self) -> &str {
        self.inner.forward_curve_id.as_str()
    }

    /// Discount curve ID.
    #[getter]
    fn discount_curve_id(&self) -> &str {
        self.inner.discount_curve_id.as_str()
    }

    /// Optional exchange.
    #[getter]
    fn exchange(&self) -> Option<&str> {
        self.inner.exchange.as_deref()
    }

    /// Optional contract month.
    #[getter]
    fn contract_month(&self) -> Option<&str> {
        self.inner.contract_month.as_deref()
    }

    /// Instrument type key.
    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        use finstack_valuations::instruments::Instrument;
        PyInstrumentType::new(self.inner.key())
    }

    fn __repr__(&self) -> String {
        format!(
            "CommodityForward(id='{}', ticker='{}', quantity={}, settlement_date='{}')",
            self.inner.id.as_str(),
            self.inner.ticker,
            self.inner.quantity,
            self.inner.settlement_date
        )
    }
}

impl fmt::Display for PyCommodityForward {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CommodityForward({}, {}, {})",
            self.inner.id.as_str(),
            self.inner.ticker,
            self.inner.quantity
        )
    }
}

/// Export module items for registration.
pub fn register_module(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    parent.add_class::<PyCommodityForward>()?;
    Ok(())
}
