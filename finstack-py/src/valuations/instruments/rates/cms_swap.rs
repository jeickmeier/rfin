use crate::core::common::args::{DayCountArg, TenorArg};
use crate::core::common::labels::normalize_label;
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::core::money::{extract_money, PyMoney};
use crate::errors::{core_to_py, PyContext};
use crate::valuations::common::PyInstrumentType;
use finstack_core::dates::DayCount;
use finstack_valuations::instruments::rates::cms_swap::{CmsSwap, FundingLegSpec};
use finstack_valuations::instruments::{IRSConvention, PayReceive};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict, PyModule, PyType};
use pyo3::Bound;
use std::sync::Arc;

fn extract_dict_string<'py>(dict: &Bound<'py, PyDict>, key: &str) -> PyResult<String> {
    dict.get_item(key)?
        .ok_or_else(|| PyValueError::new_err(format!("funding_leg['{key}'] is required")))?
        .extract::<String>()
}

fn extract_dict_f64<'py>(dict: &Bound<'py, PyDict>, key: &str) -> PyResult<f64> {
    dict.get_item(key)?
        .ok_or_else(|| PyValueError::new_err(format!("funding_leg['{key}'] is required")))?
        .extract::<f64>()
}

fn extract_dict_day_count<'py>(dict: &Bound<'py, PyDict>, key: &str) -> PyResult<DayCount> {
    let value = dict
        .get_item(key)?
        .ok_or_else(|| PyValueError::new_err(format!("funding_leg['{key}'] is required")))?;
    let DayCountArg(day_count) = value.extract()?;
    Ok(day_count)
}

fn parse_side(label: &str) -> PyResult<PayReceive> {
    match normalize_label(label).as_str() {
        "pay" | "payer" => Ok(PayReceive::Pay),
        "receive" | "receiver" | "rec" => Ok(PayReceive::Receive),
        other => Err(PyValueError::new_err(format!(
            "Invalid side: '{other}'. Must be 'pay' or 'receive'"
        ))),
    }
}

fn parse_funding_leg(spec: &Bound<'_, PyAny>) -> PyResult<FundingLegSpec> {
    let dict = spec
        .cast::<PyDict>()
        .map_err(|_| PyValueError::new_err("funding_leg must be a dict"))?;
    let kind = extract_dict_string(dict, "type")?;
    match normalize_label(&kind).as_str() {
        "fixed" => Ok(FundingLegSpec::Fixed {
            rate: extract_dict_f64(dict, "rate")?,
            day_count: extract_dict_day_count(dict, "day_count")?,
        }),
        "floating" => Ok(FundingLegSpec::Floating {
            spread: extract_dict_f64(dict, "spread")?,
            day_count: extract_dict_day_count(dict, "day_count")?,
            forward_curve_id: finstack_core::types::CurveId::new(&extract_dict_string(
                dict,
                "forward_curve_id",
            )?),
        }),
        other => Err(PyValueError::new_err(format!(
            "Invalid funding_leg type: '{other}'. Must be 'fixed' or 'floating'"
        ))),
    }
}

/// Constant maturity swap instrument.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "CmsSwap",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyCmsSwap {
    pub(crate) inner: Arc<CmsSwap>,
}

impl PyCmsSwap {
    pub(crate) fn new(inner: CmsSwap) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

#[pymethods]
impl PyCmsSwap {
    #[classmethod]
    #[pyo3(
        signature = (instrument_id, start_date, maturity, frequency, cms_tenor, cms_spread, funding_leg, notional, cms_day_count, swap_convention, side, discount_curve, forward_curve, vol_surface),
        text_signature = "(cls, instrument_id, start_date, maturity, frequency, cms_tenor, cms_spread, funding_leg, notional, cms_day_count, swap_convention, side, discount_curve, forward_curve, vol_surface)"
    )]
    #[allow(clippy::too_many_arguments)]
    fn from_schedule(
        _cls: &Bound<'_, PyType>,
        instrument_id: &str,
        start_date: Bound<'_, PyAny>,
        maturity: Bound<'_, PyAny>,
        frequency: Bound<'_, PyAny>,
        cms_tenor: f64,
        cms_spread: f64,
        funding_leg: Bound<'_, PyAny>,
        notional: Bound<'_, PyAny>,
        cms_day_count: Bound<'_, PyAny>,
        swap_convention: &str,
        side: &str,
        discount_curve: &str,
        forward_curve: &str,
        vol_surface: &str,
    ) -> PyResult<Self> {
        let start_date = py_to_date(&start_date).context("start_date")?;
        let maturity = py_to_date(&maturity).context("maturity")?;
        let TenorArg(frequency) = frequency.extract().context("frequency")?;
        let funding_leg = parse_funding_leg(&funding_leg).context("funding_leg")?;
        let notional = extract_money(&notional).context("notional")?;
        let DayCountArg(cms_day_count) = cms_day_count.extract().context("cms_day_count")?;
        let swap_convention = swap_convention
            .parse::<IRSConvention>()
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        let side = parse_side(side)?;

        CmsSwap::from_schedule(
            instrument_id,
            start_date,
            maturity,
            frequency,
            cms_tenor,
            cms_spread,
            funding_leg,
            notional,
            cms_day_count,
            swap_convention,
            side,
            discount_curve,
            forward_curve,
            vol_surface,
        )
        .map(Self::new)
        .map_err(core_to_py)
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
    fn cms_tenor(&self) -> f64 {
        self.inner.cms_tenor
    }

    #[getter]
    fn cms_spread(&self) -> f64 {
        self.inner.cms_spread
    }

    #[getter]
    fn cms_fixing_dates(&self, py: Python<'_>) -> PyResult<Vec<Py<PyAny>>> {
        self.inner
            .cms_fixing_dates
            .iter()
            .map(|date| date_to_py(py, *date))
            .collect()
    }

    #[getter]
    fn cms_payment_dates(&self, py: Python<'_>) -> PyResult<Vec<Py<PyAny>>> {
        self.inner
            .cms_payment_dates
            .iter()
            .map(|date| date_to_py(py, *date))
            .collect()
    }

    #[getter]
    fn discount_curve(&self) -> String {
        self.inner.discount_curve_id.as_str().to_string()
    }

    #[getter]
    fn forward_curve(&self) -> String {
        self.inner.forward_curve_id.as_str().to_string()
    }

    #[getter]
    fn vol_surface(&self) -> String {
        self.inner.vol_surface_id.as_str().to_string()
    }

    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::CmsSwap)
    }

    fn __repr__(&self) -> String {
        format!(
            "CmsSwap(id='{}', cms_tenor={}, discount_curve='{}')",
            self.inner.id.as_str(),
            self.inner.cms_tenor,
            self.inner.discount_curve_id.as_str()
        )
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyCmsSwap>()?;
    Ok(vec!["CmsSwap"])
}
