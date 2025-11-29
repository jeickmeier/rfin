use crate::core::common::args::DayCountArg;
use crate::core::dates::utils::py_to_date;
use crate::core::dates::PyDayCount;
use crate::core::market_data::term_structures::PyDiscountCurve;
use crate::core::money::{extract_money, PyMoney};
use crate::errors::{core_to_py, PyContext};
use finstack_core::cashflow::discounting::{npv_static, npv_using_curve_dc};
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
use pyo3::Bound;

fn parse_day_count(dc: Bound<'_, PyAny>) -> PyResult<finstack_core::dates::DayCount> {
    if let Ok(py_dc) = dc.extract::<PyRef<PyDayCount>>() {
        return Ok(py_dc.inner);
    }
    if let Ok(DayCountArg(inner)) = dc.extract::<DayCountArg>() {
        return Ok(inner);
    }
    Err(pyo3::exceptions::PyTypeError::new_err(
        "day_count must be a DayCount or string identifier",
    ))
}

fn parse_flows(
    flows: Vec<(Bound<'_, PyAny>, Bound<'_, PyAny>)>,
) -> PyResult<Vec<(finstack_core::dates::Date, finstack_core::money::Money)>> {
    let mut out = Vec::with_capacity(flows.len());
    for (date_any, amount_any) in flows {
        let date = py_to_date(&date_any).context("flows.date")?;
        let amount = extract_money(&amount_any).context("flows.amount")?;
        out.push((date, amount));
    }
    Ok(out)
}

/// Present-value cashflows using a discount curve and explicit day-count convention.
#[pyfunction(
    name = "npv_static",
    signature = (curve, base_date, day_count, cash_flows),
    text_signature = "(curve, base_date, day_count, cash_flows)"
)]
pub fn py_npv_static(
    curve: PyRef<PyDiscountCurve>,
    base_date: Bound<'_, PyAny>,
    day_count: Bound<'_, PyAny>,
    cash_flows: Vec<(Bound<'_, PyAny>, Bound<'_, PyAny>)>,
) -> PyResult<PyMoney> {
    let base = py_to_date(&base_date).context("base_date")?;
    let dc = parse_day_count(day_count)?;
    let flows = parse_flows(cash_flows)?;
    npv_static(curve.inner.as_ref(), base, dc, &flows)
        .map(PyMoney::new)
        .map_err(core_to_py)
}

/// Present-value cashflows using the curve's internal day-count convention.
#[pyfunction(
    name = "npv_using_curve_dc",
    signature = (curve, base_date, cash_flows),
    text_signature = "(curve, base_date, cash_flows)"
)]
pub fn py_npv_using_curve_dc(
    curve: PyRef<PyDiscountCurve>,
    base_date: Bound<'_, PyAny>,
    cash_flows: Vec<(Bound<'_, PyAny>, Bound<'_, PyAny>)>,
) -> PyResult<PyMoney> {
    let base = py_to_date(&base_date).context("base_date")?;
    let flows = parse_flows(cash_flows)?;
    npv_using_curve_dc(curve.inner.as_ref(), base, &flows)
        .map(PyMoney::new)
        .map_err(core_to_py)
}

pub(crate) fn register<'py>(
    py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.setattr(
        "__doc__",
        "Discounting helpers mirroring finstack_core::cashflow::discounting.",
    )?;
    module.add_function(wrap_pyfunction!(py_npv_static, module)?)?;
    module.add_function(wrap_pyfunction!(py_npv_using_curve_dc, module)?)?;
    let exports = ["npv_static", "npv_using_curve_dc"];
    module.setattr("__all__", PyList::new(py, &exports)?)?;
    Ok(exports.to_vec())
}
