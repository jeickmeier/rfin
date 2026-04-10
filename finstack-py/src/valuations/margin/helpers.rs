use crate::core::currency::PyCurrency;
use finstack_core::currency::Currency;
use finstack_margin::{ImMethodology, MarginTenor};
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;

use super::csa::{PyImMethodology, PyMarginTenor};

pub(super) fn parse_currency(ccy: &Bound<'_, PyAny>) -> PyResult<Currency> {
    if let Ok(py_ccy) = ccy.extract::<PyRef<PyCurrency>>() {
        Ok(py_ccy.inner)
    } else if let Ok(s) = ccy.extract::<String>() {
        s.parse().map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!("Invalid currency: {}", e))
        })
    } else {
        Err(PyTypeError::new_err("Expected Currency or string"))
    }
}

pub(super) fn parse_margin_tenor(v: &Bound<'_, PyAny>) -> PyResult<MarginTenor> {
    if let Ok(py) = v.extract::<PyRef<PyMarginTenor>>() {
        Ok(py.inner)
    } else if let Ok(s) = v.extract::<String>() {
        s.parse().map_err(pyo3::exceptions::PyValueError::new_err)
    } else {
        Err(PyTypeError::new_err("Expected MarginTenor or string"))
    }
}

pub(super) fn parse_im_methodology(v: &Bound<'_, PyAny>) -> PyResult<ImMethodology> {
    if let Ok(py) = v.extract::<PyRef<PyImMethodology>>() {
        Ok(py.inner)
    } else if let Ok(s) = v.extract::<String>() {
        s.parse().map_err(pyo3::exceptions::PyValueError::new_err)
    } else {
        Err(PyTypeError::new_err("Expected ImMethodology or string"))
    }
}
