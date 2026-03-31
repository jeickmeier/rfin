//! Python bindings for cashflow aggregation functions.

use finstack_core::dates::Period;
use finstack_core::money::Money;
use pyo3::prelude::*;
use pyo3::types::PyModule;

use crate::core::currency::PyCurrency;
use crate::core::dates::periods::{PyPeriod, PyPeriodPlan};
use crate::core::dates::utils::py_to_date;
use crate::core::money::{extract_money, PyMoney};
use crate::errors::core_to_py;

/// Extract periods from either a PeriodPlan or a list of Period objects.
fn extract_periods(periods: &Bound<'_, pyo3::types::PyAny>) -> PyResult<Vec<Period>> {
    if let Ok(plan) = periods.extract::<PyRef<PyPeriodPlan>>() {
        Ok(plan.periods.clone())
    } else if let Ok(periods_list) = periods.extract::<Vec<PyRef<PyPeriod>>>() {
        Ok(periods_list.iter().map(|p| p.inner.clone()).collect())
    } else {
        Err(pyo3::exceptions::PyTypeError::new_err(
            "periods must be PeriodPlan or list[Period]",
        ))
    }
}

/// Extract dated flows from Python list of (date, Money) tuples.
fn extract_dated_flows(
    flows: Vec<(Bound<'_, pyo3::types::PyAny>, Bound<'_, pyo3::types::PyAny>)>,
) -> PyResult<Vec<(finstack_core::dates::Date, Money)>> {
    let mut out = Vec::with_capacity(flows.len());
    for (d, m) in flows {
        out.push((py_to_date(&d)?, extract_money(&m)?));
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// aggregate_by_period
// ---------------------------------------------------------------------------

/// Aggregate dated cashflows by period with currency preservation.
///
/// Groups cashflows into periods and sums amounts per currency. Periods
/// with no cashflows are omitted from the result.
///
/// Args:
///     flows: List of (date, Money) tuples. Inputs do not need to be pre-sorted.
///     periods: Reporting periods (PeriodPlan or list[Period]) using
///         half-open intervals ``[start, end)``.
///
/// Returns:
///     Dictionary mapping period code strings to inner dictionaries of
///     ``{currency_str: Money}``. Only periods with at least one flow
///     are included.
#[pyfunction]
#[pyo3(name = "aggregate_by_period", text_signature = "(flows, periods)")]
fn py_aggregate_by_period(
    py: Python<'_>,
    flows: Vec<(Bound<'_, pyo3::types::PyAny>, Bound<'_, pyo3::types::PyAny>)>,
    periods: Bound<'_, pyo3::types::PyAny>,
) -> PyResult<Py<pyo3::types::PyDict>> {
    let rust_flows = extract_dated_flows(flows)?;
    let rust_periods = extract_periods(&periods)?;

    let result =
        finstack_valuations::cashflow::aggregation::aggregate_by_period(&rust_flows, &rust_periods);

    let dict = pyo3::types::PyDict::new(py);
    for (period_id, ccy_map) in &result {
        let inner_dict = pyo3::types::PyDict::new(py);
        for (&ccy, &money) in ccy_map {
            let py_ccy = PyCurrency { inner: ccy };
            inner_dict.set_item(
                py_ccy.into_pyobject(py)?,
                PyMoney::new(money).into_pyobject(py)?,
            )?;
        }
        dict.set_item(period_id.to_string(), inner_dict)?;
    }
    Ok(dict.unbind())
}

// ---------------------------------------------------------------------------
// aggregate_cashflows_precise_checked
// ---------------------------------------------------------------------------

/// Single-currency precision aggregation of dated cashflows.
///
/// Uses compensated summation for numerical precision. All flows must
/// match the target currency.
///
/// Args:
///     flows: List of (date, Money) tuples.
///     target: Required currency for every flow and the returned total.
///
/// Returns:
///     A single Money total in the target currency.
///
/// Raises:
///     FinstackError: If any flow's currency differs from ``target``.
#[pyfunction]
#[pyo3(
    name = "aggregate_cashflows_precise_checked",
    text_signature = "(flows, target)"
)]
fn py_aggregate_cashflows_precise_checked(
    flows: Vec<(Bound<'_, pyo3::types::PyAny>, Bound<'_, pyo3::types::PyAny>)>,
    target: &PyCurrency,
) -> PyResult<PyMoney> {
    let rust_flows = extract_dated_flows(flows)?;
    let result = finstack_valuations::cashflow::aggregation::aggregate_cashflows_precise_checked(
        &rust_flows,
        target.inner,
    )
    .map_err(core_to_py)?;
    Ok(PyMoney::new(result))
}

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_function(wrap_pyfunction!(py_aggregate_by_period, module)?)?;
    module.add_function(wrap_pyfunction!(
        py_aggregate_cashflows_precise_checked,
        module
    )?)?;
    Ok(vec![
        "aggregate_by_period",
        "aggregate_cashflows_precise_checked",
    ])
}
