use crate::core::error::core_to_py;
use crate::core::money::{extract_money, PyMoney};
use crate::core::utils::{date_to_py, py_to_date};
use crate::valuations::common::{
    extract_curve_id, extract_instrument_id, leak_str, PyInstrumentType,
};
use finstack_core::dates::DayCount;
use finstack_valuations::instruments::inflation_swap::{InflationSwap, PayReceiveInflation};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::Bound;
use std::fmt;

fn parse_side(label: Option<&str>) -> PyResult<PayReceiveInflation> {
    match label {
        None => Ok(PayReceiveInflation::PayFixed),
        Some(s) => s.parse().map_err(|e: String| PyValueError::new_err(e)),
    }
}

/// Zero-coupon inflation swap binding.
///
/// Examples:
///     >>> swap = InflationSwap.create(
///     ...     "zciis_usd",
///     ...     Money("USD", 10_000_000),
///     ...     0.02,
///     ...     date(2024, 1, 1),
///     ...     date(2034, 1, 1),
///     ...     "usd_discount",
///     ...     inflation_index="us_cpi"
///     ... )
///     >>> swap.fixed_rate
///     0.02
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "InflationSwap",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyInflationSwap {
    pub(crate) inner: InflationSwap,
}

impl PyInflationSwap {
    pub(crate) fn new(inner: InflationSwap) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyInflationSwap {
    #[classmethod]
    #[pyo3(
        text_signature = "(cls, instrument_id, notional, fixed_rate, start_date, maturity, discount_curve, inflation_index=None, /, *, side='pay_fixed', day_count='act_act', inflation_id=None, lag_override=None, inflation_curve=None)",
        signature = (
            instrument_id,
            notional,
            fixed_rate,
            start_date,
            maturity,
            discount_curve,
            inflation_index = None,
            *,
            side = None,
            day_count = None,
            inflation_id = None,
            lag_override = None,
            inflation_curve = None,
        )
    )]
    #[allow(clippy::too_many_arguments)]
    /// Create an inflation swap fixing against the supplied inflation index.
    ///
    /// Args:
    ///     instrument_id: Instrument identifier or string-like object.
    ///     notional: Notional principal as :class:`finstack.core.money.Money`.
    ///     fixed_rate: Fixed leg rate expressed as decimal.
    ///     start_date: Start date of the swap.
    ///     maturity: Maturity date of the swap.
    ///     discount_curve: Discount curve identifier.
    ///     inflation_index: Optional inflation index identifier.
    ///     side: Optional pay/receive label (defaults to pay fixed).
    ///     day_count: Optional day-count convention label.
    ///     inflation_id: Optional explicit inflation curve identifier.
    ///     lag_override: Optional lag override label.
    ///     inflation_curve: Optional curve identifier used when ``inflation_id`` omitted.
    ///
    /// Returns:
    ///     InflationSwap: Configured inflation swap instrument.
    ///
    /// Raises:
    ///     ValueError: If required indexes are missing or labels are invalid.
    ///     RuntimeError: When the underlying builder detects invalid input.
    fn create(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        notional: Bound<'_, PyAny>,
        fixed_rate: f64,
        start_date: Bound<'_, PyAny>,
        maturity: Bound<'_, PyAny>,
        discount_curve: Bound<'_, PyAny>,
        inflation_index: Option<&str>,
        side: Option<&str>,
        day_count: Option<&str>,
        inflation_id: Option<&str>,
        lag_override: Option<&str>,
        inflation_curve: Option<&str>,
    ) -> PyResult<Self> {
        let id = extract_instrument_id(&instrument_id)?;
        let notional_money = extract_money(&notional)?;
        let start = py_to_date(&start_date)?;
        let end = py_to_date(&maturity)?;
        let disc_id = extract_curve_id(&discount_curve)?;
        let side_value = parse_side(side)?;
        let dc = if let Some(name) = day_count {
            match crate::core::common::labels::normalize_label(name).as_str() {
                "act_act" | "actact" => DayCount::ActAct,
                "act_360" | "act360" => DayCount::Act360,
                "act_365f" | "act365f" | "act_365_fixed" => DayCount::Act365F,
                other => {
                    return Err(PyValueError::new_err(format!(
                        "Unsupported day_count: {other}",
                    )))
                }
            }
        } else {
            DayCount::ActAct
        };
        let inflation_identifier = if let Some(explicit) = inflation_id {
            leak_str(explicit)
        } else if let Some(label) = inflation_index.or(inflation_curve) {
            leak_str(label)
        } else {
            return Err(PyValueError::new_err(
                "inflation_index or inflation_curve must be provided",
            ));
        };

        let mut builder = InflationSwap::builder();
        builder = builder.id(id);
        builder = builder.notional(notional_money);
        builder = builder.fixed_rate(fixed_rate);
        builder = builder.start(start);
        builder = builder.maturity(end);
        builder = builder.disc_id(disc_id);
        builder = builder.inflation_id(inflation_identifier);
        builder = builder.dc(dc);
        builder = builder.side(side_value);
        if let Some(lag) = lag_override {
            let normalized = crate::core::common::labels::normalize_label(lag);
            use finstack_core::market_data::scalars::inflation_index::InflationLag;
            let lag_value = match normalized.as_str() {
                "none" => InflationLag::None,
                "3m" | "three_months" => InflationLag::Months(3),
                "8m" | "eight_months" => InflationLag::Months(8),
                other => {
                    return Err(PyValueError::new_err(format!(
                        "Unsupported lag override: {other}",
                    )))
                }
            };
            builder = builder.lag_override_opt(Some(lag_value));
        }
        builder = builder.attributes(Default::default());

        let swap = builder.build().map_err(core_to_py)?;
        Ok(Self::new(swap))
    }

    /// Instrument identifier.
    ///
    /// Returns:
    ///     str: Unique identifier assigned to the swap.
    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    /// Notional principal amount.
    ///
    /// Returns:
    ///     Money: Notional wrapped as :class:`finstack.core.money.Money`.
    #[getter]
    fn notional(&self) -> PyMoney {
        PyMoney::new(self.inner.notional)
    }

    /// Fixed leg rate in decimal form.
    ///
    /// Returns:
    ///     float: Fixed rate of the swap.
    #[getter]
    fn fixed_rate(&self) -> f64 {
        self.inner.fixed_rate
    }

    /// Maturity date of the swap.
    ///
    /// Returns:
    ///     datetime.date: Maturity converted to Python.
    #[getter]
    fn maturity(&self, py: Python<'_>) -> PyResult<PyObject> {
        date_to_py(py, self.inner.maturity)
    }

    /// Instrument type enumeration.
    ///
    /// Returns:
    ///     InstrumentType: Enumeration value ``InstrumentType.INFLATION_SWAP``.
    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::InflationSwap)
    }

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!(
            "InflationSwap(id='{}', fixed_rate={:.4})",
            self.inner.id, self.inner.fixed_rate
        ))
    }
}

impl fmt::Display for PyInflationSwap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "InflationSwap({}, fixed_rate={:.4})",
            self.inner.id, self.inner.fixed_rate
        )
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyInflationSwap>()?;
    Ok(vec!["InflationSwap"])
}
