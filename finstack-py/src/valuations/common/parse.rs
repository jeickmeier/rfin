//! Parsing helpers for valuations-specific enums.
//!
//! These functions only convert Python strings into strongly typed Rust enums
//! and surface them back as Python wrapper types.

use crate::valuations::common::parameters::{
    PyExerciseStyle, PyOptionType, PyPayReceive, PySettlementType,
};
use finstack_valuations::instruments::{
    legs::PayReceive,
    market::{ExerciseStyle, OptionType, SettlementType},
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
use pyo3::Bound;

#[pyfunction(name = "option_type")]
#[pyo3(text_signature = "(label)")]
pub fn parse_option_type(label: &str) -> PyResult<PyOptionType> {
    label
        .parse::<OptionType>()
        .map(PyOptionType::from)
        .map_err(|e: String| PyValueError::new_err(e))
}

#[pyfunction(name = "exercise_style")]
#[pyo3(text_signature = "(label)")]
pub fn parse_exercise_style(label: &str) -> PyResult<PyExerciseStyle> {
    label
        .parse::<ExerciseStyle>()
        .map(PyExerciseStyle::new)
        .map_err(|e: String| PyValueError::new_err(e))
}

#[pyfunction(name = "settlement_type")]
#[pyo3(text_signature = "(label)")]
pub fn parse_settlement_type(label: &str) -> PyResult<PySettlementType> {
    label
        .parse::<SettlementType>()
        .map(PySettlementType::new)
        .map_err(|e: String| PyValueError::new_err(e))
}

#[pyfunction(name = "pay_receive")]
#[pyo3(text_signature = "(label)")]
pub fn parse_pay_receive(label: &str) -> PyResult<PyPayReceive> {
    label
        .parse::<PayReceive>()
        .map(PyPayReceive::new)
        .map_err(|e: String| PyValueError::new_err(e))
}

pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "parse")?;
    module.setattr(
        "__doc__",
        "Convert string labels into valuations enums without duplicating pricing logic.",
    )?;

    module.add_function(wrap_pyfunction!(parse_option_type, &module)?)?;
    module.add_function(wrap_pyfunction!(parse_exercise_style, &module)?)?;
    module.add_function(wrap_pyfunction!(parse_settlement_type, &module)?)?;
    module.add_function(wrap_pyfunction!(parse_pay_receive, &module)?)?;
    let exports = vec![
        "option_type",
        "exercise_style",
        "settlement_type",
        "pay_receive",
    ];
    module.setattr("__all__", PyList::new(py, &exports)?)?;
    parent.add_submodule(&module)?;
    Ok(exports)
}
