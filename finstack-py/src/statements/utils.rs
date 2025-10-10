//! Utility functions for statements bindings.

use pyo3::prelude::*;
use pyo3::types::{PyBool, PyDict, PyFloat, PyList, PyString};

/// Helper to convert serde_json::Value to PyObject
pub(crate) fn json_to_py(py: Python<'_>, value: &serde_json::Value) -> PyObject {
    match value {
        serde_json::Value::Null => py.None(),
        serde_json::Value::Bool(b) => PyBool::new(py, *b).as_any().clone().unbind(),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                i.into_pyobject(py).unwrap().into_any().clone().unbind()
            } else if let Some(f) = n.as_f64() {
                PyFloat::new(py, f).into()
            } else {
                py.None()
            }
        }
        serde_json::Value::String(s) => PyString::new(py, s).into(),
        serde_json::Value::Array(arr) => {
            let items: Vec<PyObject> = arr.iter().map(|v| json_to_py(py, v)).collect();
            PyList::new(py, items).unwrap().into()
        }
        serde_json::Value::Object(obj) => {
            let dict = PyDict::new(py);
            for (k, v) in obj {
                dict.set_item(k, json_to_py(py, v)).ok();
            }
            dict.into()
        }
    }
}

/// Helper to convert PyAny to serde_json::Value
pub(crate) fn py_to_json(value: &Bound<'_, PyAny>) -> pyo3::PyResult<serde_json::Value> {
    if value.is_none() {
        Ok(serde_json::Value::Null)
    } else if let Ok(s) = value.extract::<String>() {
        Ok(serde_json::json!(s))
    } else if let Ok(i) = value.extract::<i64>() {
        Ok(serde_json::json!(i))
    } else if let Ok(f) = value.extract::<f64>() {
        Ok(serde_json::json!(f))
    } else if let Ok(b) = value.extract::<bool>() {
        Ok(serde_json::json!(b))
    } else if let Ok(v) = value.extract::<Vec<f64>>() {
        Ok(serde_json::json!(v))
    } else if let Ok(v) = value.extract::<Vec<String>>() {
        Ok(serde_json::json!(v))
    } else {
        Ok(serde_json::Value::Null)
    }
}

