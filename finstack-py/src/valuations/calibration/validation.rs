use crate::core::error::core_to_py;
use crate::core::market_data::surfaces::PyVolSurface;
use crate::core::market_data::term_structures::{
    PyDiscountCurve, PyForwardCurve, PyHazardCurve, PyInflationCurve,
};
use finstack_valuations::calibration::{CurveValidator, SurfaceValidator};
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
use pyo3::Bound;

#[pyfunction]
#[pyo3(text_signature = "(curve)")]
fn validate_discount_curve(curve: &PyDiscountCurve) -> PyResult<()> {
    curve.inner.validate().map_err(core_to_py)
}

#[pyfunction]
#[pyo3(text_signature = "(curve)")]
fn validate_forward_curve(curve: &PyForwardCurve) -> PyResult<()> {
    curve.inner.validate().map_err(core_to_py)
}

#[pyfunction]
#[pyo3(text_signature = "(curve)")]
fn validate_hazard_curve(curve: &PyHazardCurve) -> PyResult<()> {
    curve.inner.validate().map_err(core_to_py)
}

#[pyfunction]
#[pyo3(text_signature = "(curve)")]
fn validate_inflation_curve(curve: &PyInflationCurve) -> PyResult<()> {
    curve.inner.validate().map_err(core_to_py)
}

#[pyfunction]
#[pyo3(text_signature = "(surface)")]
fn validate_vol_surface(surface: &PyVolSurface) -> PyResult<()> {
    surface.inner.validate().map_err(core_to_py)
}

pub(crate) fn register<'py>(
    py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_function(pyo3::wrap_pyfunction!(validate_discount_curve, module)?)?;
    module.add_function(pyo3::wrap_pyfunction!(validate_forward_curve, module)?)?;
    module.add_function(pyo3::wrap_pyfunction!(validate_hazard_curve, module)?)?;
    module.add_function(pyo3::wrap_pyfunction!(validate_inflation_curve, module)?)?;
    module.add_function(pyo3::wrap_pyfunction!(validate_vol_surface, module)?)?;

    let exports = [
        "validate_discount_curve",
        "validate_forward_curve",
        "validate_hazard_curve",
        "validate_inflation_curve",
        "validate_vol_surface",
    ];
    module.setattr("__all__", PyList::new(py, &exports)?)?;
    Ok(exports.to_vec())
}
