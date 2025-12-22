//! Python bindings for CommoditySwap instrument.

use crate::core::common::args::CurrencyArg;
use crate::core::currency::PyCurrency;
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::valuations::common::PyInstrumentType;
use finstack_core::dates::{BusinessDayConvention, Tenor, TenorUnit};
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::commodity_swap::CommoditySwap;
use finstack_valuations::instruments::common::traits::Attributes;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyType};
use pyo3::{Bound, Py};
use std::fmt;

/// Commodity swap (fixed-for-floating commodity price exchange).
///
/// One party pays a fixed price per unit, the other pays a floating price
/// determined by an index or average of spot prices over the period.
///
/// Examples:
///     >>> swap = CommoditySwap.create(
///     ...     "NG-SWAP-2025",
///     ...     commodity_type="Energy",
///     ...     ticker="NG",
///     ...     unit="MMBTU",
///     ...     currency="USD",
///     ...     notional_quantity=10000.0,
///     ...     fixed_price=3.50,
///     ...     floating_index_id="NG-SPOT-AVG",
///     ...     pay_fixed=True,
///     ...     start_date=Date(2025, 1, 1),
///     ...     end_date=Date(2025, 12, 31),
///     ...     payment_frequency="1M",
///     ...     discount_curve_id="USD-OIS"
///     ... )
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "CommoditySwap",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyCommoditySwap {
    pub(crate) inner: CommoditySwap,
}

impl PyCommoditySwap {
    pub(crate) fn new(inner: CommoditySwap) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyCommoditySwap {
    #[classmethod]
    #[pyo3(
        text_signature = "(cls, instrument_id, *, commodity_type, ticker, unit, currency, notional_quantity, fixed_price, floating_index_id, pay_fixed, start_date, end_date, payment_frequency, discount_curve_id, calendar_id=None, bdc=None, index_lag_days=None)"
    )]
    #[pyo3(
        signature = (
            instrument_id,
            *,
            commodity_type,
            ticker,
            unit,
            currency,
            notional_quantity,
            fixed_price,
            floating_index_id,
            pay_fixed,
            start_date,
            end_date,
            payment_frequency,
            discount_curve_id,
            calendar_id = None,
            bdc = None,
            index_lag_days = None
        )
    )]
    /// Create a commodity swap.
    ///
    /// Args:
    ///     instrument_id: Unique identifier for this instrument.
    ///     commodity_type: Commodity type (e.g., "Energy", "Metal", "Agricultural").
    ///     ticker: Ticker or symbol (e.g., "CL" for WTI, "NG" for Natural Gas).
    ///     unit: Unit of measurement (e.g., "BBL", "MMBTU", "MT").
    ///     currency: Currency for pricing and settlement.
    ///     notional_quantity: Notional quantity per period.
    ///     fixed_price: Fixed price per unit.
    ///     floating_index_id: Floating index ID for price lookups.
    ///     pay_fixed: True if paying fixed (receiving floating).
    ///     start_date: Start date of the swap.
    ///     end_date: End date of the swap.
    ///     payment_frequency: Payment frequency as tenor string (e.g., "1M", "3M").
    ///     discount_curve_id: Discount curve ID.
    ///     calendar_id: Optional calendar ID for date adjustments.
    ///     bdc: Business day convention ("following", "modified_following", etc.).
    ///     index_lag_days: Optional index lag in days.
    ///
    /// Returns:
    ///     CommoditySwap: Configured commodity swap instrument.
    fn create(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        commodity_type: &str,
        ticker: &str,
        unit: &str,
        currency: Bound<'_, PyAny>,
        notional_quantity: f64,
        fixed_price: f64,
        floating_index_id: &str,
        pay_fixed: bool,
        start_date: Bound<'_, PyAny>,
        end_date: Bound<'_, PyAny>,
        payment_frequency: &str,
        discount_curve_id: &str,
        calendar_id: Option<&str>,
        bdc: Option<&str>,
        index_lag_days: Option<i32>,
    ) -> PyResult<Self> {
        use crate::errors::PyContext;

        let id = InstrumentId::new(instrument_id.extract::<&str>().context("instrument_id")?);
        let CurrencyArg(ccy) = currency.extract().context("currency")?;
        let start = py_to_date(&start_date).context("start_date")?;
        let end = py_to_date(&end_date).context("end_date")?;

        // Parse frequency tenor
        let freq = parse_tenor(payment_frequency).map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!(
                "Invalid payment_frequency '{}': {}",
                payment_frequency, e
            ))
        })?;

        // Parse BDC
        let bdc_enum = match bdc {
            Some("following") | Some("Following") => Some(BusinessDayConvention::Following),
            Some("modified_following") | Some("ModifiedFollowing") => {
                Some(BusinessDayConvention::ModifiedFollowing)
            }
            Some("preceding") | Some("Preceding") => Some(BusinessDayConvention::Preceding),
            Some("modified_preceding") | Some("ModifiedPreceding") => {
                Some(BusinessDayConvention::ModifiedPreceding)
            }
            Some("unadjusted") | Some("Unadjusted") | Some("none") | Some("None") => {
                Some(BusinessDayConvention::Unadjusted)
            }
            None => None,
            Some(other) => {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "Invalid bdc: '{}'. Must be 'following', 'modified_following', 'preceding', 'modified_preceding', or 'unadjusted'",
                    other
                )));
            }
        };

        let mut builder = CommoditySwap::builder()
            .id(id)
            .commodity_type(commodity_type.to_string())
            .ticker(ticker.to_string())
            .unit(unit.to_string())
            .currency(ccy)
            .notional_quantity(notional_quantity)
            .fixed_price(fixed_price)
            .floating_index_id(CurveId::new(floating_index_id))
            .pay_fixed(pay_fixed)
            .start_date(start)
            .end_date(end)
            .payment_frequency(freq)
            .discount_curve_id(CurveId::new(discount_curve_id))
            .attributes(Attributes::new());

        if let Some(cal) = calendar_id {
            builder = builder.calendar_id_opt(Some(cal.to_string()));
        }
        if let Some(b) = bdc_enum {
            builder = builder.bdc_opt(Some(b));
        }
        if let Some(lag) = index_lag_days {
            builder = builder.index_lag_days_opt(Some(lag));
        }

        let swap = builder
            .build()
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("{}", e)))?;

        Ok(Self::new(swap))
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

    /// Unit of measurement.
    #[getter]
    fn unit(&self) -> &str {
        &self.inner.unit
    }

    /// Currency.
    #[getter]
    fn currency(&self) -> PyCurrency {
        PyCurrency::new(self.inner.currency)
    }

    /// Notional quantity per period.
    #[getter]
    fn notional_quantity(&self) -> f64 {
        self.inner.notional_quantity
    }

    /// Fixed price per unit.
    #[getter]
    fn fixed_price(&self) -> f64 {
        self.inner.fixed_price
    }

    /// Whether paying fixed (receiving floating).
    #[getter]
    fn pay_fixed(&self) -> bool {
        self.inner.pay_fixed
    }

    /// Start date.
    #[getter]
    fn start_date(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.start_date)
    }

    /// End date.
    #[getter]
    fn end_date(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.end_date)
    }

    /// Floating index ID.
    #[getter]
    fn floating_index_id(&self) -> &str {
        self.inner.floating_index_id.as_str()
    }

    /// Discount curve ID.
    #[getter]
    fn discount_curve_id(&self) -> &str {
        self.inner.discount_curve_id.as_str()
    }

    /// Instrument type key.
    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        use finstack_valuations::instruments::common::traits::Instrument;
        PyInstrumentType::new(self.inner.key())
    }

    fn __repr__(&self) -> String {
        format!(
            "CommoditySwap(id='{}', ticker='{}', fixed_price={}, pay_fixed={})",
            self.inner.id.as_str(),
            self.inner.ticker,
            self.inner.fixed_price,
            self.inner.pay_fixed
        )
    }
}

impl fmt::Display for PyCommoditySwap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CommoditySwap({}, {}, {})",
            self.inner.id.as_str(),
            self.inner.ticker,
            self.inner.fixed_price
        )
    }
}

/// Parse a tenor string like "1M", "3M", "1Y" into a Tenor.
fn parse_tenor(s: &str) -> Result<Tenor, String> {
    let s = s.trim().to_uppercase();
    if s.is_empty() {
        return Err("Empty tenor string".to_string());
    }

    // Find the split point between number and unit
    let unit_start = s.find(|c: char| c.is_alphabetic()).ok_or("No unit found")?;
    let count_str = &s[..unit_start];
    let unit_str = &s[unit_start..];

    let count: u32 = count_str
        .parse()
        .map_err(|_| format!("Invalid count: {}", count_str))?;

    let unit = match unit_str {
        "D" => TenorUnit::Days,
        "W" => TenorUnit::Weeks,
        "M" => TenorUnit::Months,
        "Y" => TenorUnit::Years,
        _ => return Err(format!("Unknown unit: {}", unit_str)),
    };

    Ok(Tenor::new(count, unit))
}

/// Export module items for registration.
pub fn register_module(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    parent.add_class::<PyCommoditySwap>()?;
    Ok(())
}
