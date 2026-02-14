#![allow(clippy::unwrap_used)]

use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::core::money::{extract_money, PyMoney};
use crate::errors::core_to_py;
use crate::valuations::common::PyInstrumentType;
use finstack_core::dates::DayCount;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::rates::inflation_swap::{InflationSwap, PayReceive};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::{Bound, Py, PyRefMut};
use std::fmt;
use std::sync::Arc;

fn parse_side(label: Option<&str>) -> PyResult<PayReceive> {
    match label {
        None => Ok(PayReceive::PayFixed),
        Some(s) => s.parse().map_err(|e: String| PyValueError::new_err(e)),
    }
}

/// Zero-coupon inflation swap binding.
///
/// Examples:
///     >>> swap = (
///     ...     InflationSwap.builder("zciis_usd")
///     ...     .notional(Money("USD", 10_000_000))
///     ...     .fixed_rate(0.02)
///     ...     .start_date(date(2024, 1, 1))
///     ...     .maturity(date(2034, 1, 1))
///     ...     .discount_curve("usd_discount")
///     ...     .inflation_curve("us_cpi")
///     ...     .build()
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
    pub(crate) inner: Arc<InflationSwap>,
}

impl PyInflationSwap {
    pub(crate) fn new(inner: InflationSwap) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

#[pyclass(
    module = "finstack.valuations.instruments",
    name = "InflationSwapBuilder",
    unsendable
)]
pub struct PyInflationSwapBuilder {
    instrument_id: InstrumentId,
    notional: Option<finstack_core::money::Money>,
    fixed_rate: Option<f64>,
    start_date: Option<time::Date>,
    maturity: Option<time::Date>,
    discount_curve: Option<CurveId>,
    inflation_index_id: Option<String>,
    side: PayReceive,
    day_count: DayCount,
    lag_override: Option<finstack_core::market_data::scalars::InflationLag>,
}

impl PyInflationSwapBuilder {
    fn new_with_id(id: InstrumentId) -> Self {
        Self {
            instrument_id: id,
            notional: None,
            fixed_rate: None,
            start_date: None,
            maturity: None,
            discount_curve: None,
            inflation_index_id: None,
            side: PayReceive::PayFixed,
            day_count: DayCount::ActAct,
            lag_override: None,
        }
    }

    fn ensure_ready(&self) -> PyResult<()> {
        if self.notional.is_none() {
            return Err(PyValueError::new_err("notional() is required."));
        }
        if self.fixed_rate.is_none() {
            return Err(PyValueError::new_err("fixed_rate() is required."));
        }
        if self.start_date.is_none() {
            return Err(PyValueError::new_err("start_date() is required."));
        }
        if self.maturity.is_none() {
            return Err(PyValueError::new_err("maturity() is required."));
        }
        if self.discount_curve.is_none() {
            return Err(PyValueError::new_err("discount_curve() is required."));
        }
        if self.inflation_index_id.as_deref().unwrap_or("").is_empty() {
            return Err(PyValueError::new_err("inflation_index_id() is required."));
        }
        Ok(())
    }
}

#[pymethods]
impl PyInflationSwapBuilder {
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
        use crate::errors::PyContext;
        slf.notional = Some(extract_money(&notional).context("notional")?);
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, fixed_rate)")]
    fn fixed_rate(mut slf: PyRefMut<'_, Self>, fixed_rate: f64) -> PyRefMut<'_, Self> {
        slf.fixed_rate = Some(fixed_rate);
        slf
    }

    #[pyo3(text_signature = "($self, start_date)")]
    fn start_date<'py>(
        mut slf: PyRefMut<'py, Self>,
        start_date: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        use crate::errors::PyContext;
        slf.start_date = Some(py_to_date(&start_date).context("start_date")?);
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, maturity)")]
    fn maturity<'py>(
        mut slf: PyRefMut<'py, Self>,
        maturity: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        use crate::errors::PyContext;
        slf.maturity = Some(py_to_date(&maturity).context("maturity")?);
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, discount_curve)")]
    fn discount_curve(mut slf: PyRefMut<'_, Self>, discount_curve: String) -> PyRefMut<'_, Self> {
        slf.discount_curve = Some(CurveId::new(discount_curve.as_str()));
        slf
    }

    #[pyo3(text_signature = "($self, inflation_index_id)")]
    fn inflation_index_id(
        mut slf: PyRefMut<'_, Self>,
        inflation_index_id: String,
    ) -> PyRefMut<'_, Self> {
        slf.inflation_index_id = Some(inflation_index_id);
        slf
    }

    #[pyo3(text_signature = "($self, inflation_curve)")]
    fn inflation_curve(mut slf: PyRefMut<'_, Self>, inflation_curve: String) -> PyRefMut<'_, Self> {
        slf.inflation_index_id = Some(inflation_curve);
        slf
    }

    #[pyo3(text_signature = "($self, side)")]
    fn side(mut slf: PyRefMut<'_, Self>, side: String) -> PyResult<PyRefMut<'_, Self>> {
        slf.side = parse_side(Some(side.as_str()))?;
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, day_count)")]
    fn day_count(mut slf: PyRefMut<'_, Self>, day_count: String) -> PyResult<PyRefMut<'_, Self>> {
        slf.day_count = match crate::core::common::labels::normalize_label(&day_count).as_str() {
            "act_act" | "actact" => DayCount::ActAct,
            "act_360" | "act360" => DayCount::Act360,
            "act_365f" | "act365f" | "act_365_fixed" => DayCount::Act365F,
            other => {
                return Err(PyValueError::new_err(format!(
                    "Unsupported day_count: {other}",
                )))
            }
        };
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, lag_override=None)", signature = (lag_override=None))]
    fn lag_override(
        mut slf: PyRefMut<'_, Self>,
        lag_override: Option<String>,
    ) -> PyResult<PyRefMut<'_, Self>> {
        if let Some(lag) = lag_override {
            let normalized = crate::core::common::labels::normalize_label(&lag);
            use finstack_core::market_data::scalars::InflationLag;
            slf.lag_override = Some(match normalized.as_str() {
                "none" => InflationLag::None,
                "3m" | "three_months" => InflationLag::Months(3),
                "8m" | "eight_months" => InflationLag::Months(8),
                other => {
                    return Err(PyValueError::new_err(format!(
                        "Unsupported lag override: {other}",
                    )))
                }
            });
        } else {
            slf.lag_override = None;
        }
        Ok(slf)
    }

    #[pyo3(text_signature = "($self)")]
    fn build(slf: PyRefMut<'_, Self>) -> PyResult<PyInflationSwap> {
        slf.ensure_ready()?;

        let mut builder = InflationSwap::builder();
        builder = builder.id(slf.instrument_id.clone());
        builder = builder.notional(slf.notional.unwrap());
        builder = builder.fixed_rate(
            rust_decimal::Decimal::try_from(slf.fixed_rate.unwrap()).unwrap_or_default(),
        );
        builder = builder.start_date(slf.start_date.unwrap());
        builder = builder.maturity(slf.maturity.unwrap());
        builder = builder.discount_curve_id(slf.discount_curve.clone().unwrap());
        builder = builder.inflation_index_id(slf.inflation_index_id.clone().unwrap().into());
        builder = builder.day_count(slf.day_count);
        builder = builder.side(slf.side);
        builder = builder.lag_override_opt(slf.lag_override);
        builder = builder.attributes(Default::default());

        let swap = builder.build().map_err(core_to_py)?;
        Ok(PyInflationSwap::new(swap))
    }

    fn __repr__(&self) -> String {
        "InflationSwapBuilder(...)".to_string()
    }
}

#[pymethods]
impl PyInflationSwap {
    #[classmethod]
    #[pyo3(text_signature = "(cls, instrument_id)")]
    /// Start a fluent builder (builder-only API).
    fn builder<'py>(
        cls: &Bound<'py, PyType>,
        instrument_id: &str,
    ) -> PyResult<Py<PyInflationSwapBuilder>> {
        let py = cls.py();
        let builder = PyInflationSwapBuilder::new_with_id(InstrumentId::new(instrument_id));
        Py::new(py, builder)
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
        rust_decimal::prelude::ToPrimitive::to_f64(&self.inner.fixed_rate).unwrap_or_default()
    }

    /// Maturity date of the swap.
    ///
    /// Returns:
    ///     datetime.date: Maturity converted to Python.
    #[getter]
    fn maturity(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
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
    module.add_class::<PyInflationSwapBuilder>()?;
    Ok(vec!["InflationSwap", "InflationSwapBuilder"])
}
