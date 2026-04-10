mod credit;
mod market;
mod parametric;
mod rates;

pub use credit::{PyBaseCorrelationCurve, PyCreditIndexData, PyHazardCurve, PyInflationCurve};
pub use market::{PyForwardVarianceCurve, PyPriceCurve, PyVolatilityIndexCurve};
pub use parametric::{PyBasisSpreadCurve, PyFlatCurve, PyNelsonSiegelModel, PyNsVariant};
pub use rates::{PyDiscountCurve, PyForwardCurve};

use crate::core::common::args::DayCountArg;
use crate::core::dates::PyDayCount;
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
use pyo3::{Bound, PyRef};

pub(crate) fn parse_day_count(
    dc: Option<Bound<'_, PyAny>>,
) -> PyResult<Option<finstack_core::dates::DayCount>> {
    match dc {
        None => Ok(None),
        Some(value) => {
            if let Ok(dc) = value.extract::<PyRef<PyDayCount>>() {
                return Ok(Some(dc.inner));
            }
            if let Ok(DayCountArg(inner)) = value.extract::<DayCountArg>() {
                return Ok(Some(inner));
            }
            Err(pyo3::exceptions::PyTypeError::new_err(
                "day_count must be DayCount or string",
            ))
        }
    }
}

pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "term_structures")?;
    module.setattr(
        "__doc__",
        "One-dimensional market curves: discount, forward, hazard, inflation, base correlation, volatility index, and credit index aggregates.",
    )?;
    module.add_class::<PyDiscountCurve>()?;
    module.add_class::<PyForwardCurve>()?;
    module.add_class::<PyHazardCurve>()?;
    module.add_class::<PyInflationCurve>()?;
    module.add_class::<PyBaseCorrelationCurve>()?;
    module.add_class::<PyCreditIndexData>()?;
    module.add_class::<PyVolatilityIndexCurve>()?;
    module.add_class::<PyPriceCurve>()?;
    module.add_class::<PyFlatCurve>()?;
    module.add_class::<PyBasisSpreadCurve>()?;
    module.add_class::<PyNsVariant>()?;
    module.add_class::<PyNelsonSiegelModel>()?;
    module.add_class::<PyForwardVarianceCurve>()?;

    let exports = [
        "DiscountCurve",
        "ForwardCurve",
        "HazardCurve",
        "InflationCurve",
        "BaseCorrelationCurve",
        "CreditIndexData",
        "VolatilityIndexCurve",
        "PriceCurve",
        "FlatCurve",
        "BasisSpreadCurve",
        "NsVariant",
        "NelsonSiegelModel",
        "ForwardVarianceCurve",
    ];
    module.setattr("__all__", PyList::new(py, exports)?)?;
    parent.add_submodule(&module)?;
    Ok(exports.to_vec())
}
