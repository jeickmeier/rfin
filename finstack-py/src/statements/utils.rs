//! Utility functions for statements bindings.

use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::{PyBool, PyDict, PyFloat, PyList, PyString};
use pyo3::IntoPyObjectExt;

/// Helper to convert serde_json::Value to a Python object.
///
/// This is intentionally **strict** and will surface Python allocation/errors
/// instead of silently dropping data.
pub(crate) fn json_to_py(py: Python<'_>, value: &serde_json::Value) -> PyResult<Py<PyAny>> {
    match value {
        serde_json::Value::Null => Ok(py.None()),
        serde_json::Value::Bool(b) => Ok(PyBool::new(py, *b).as_any().clone().unbind()),
        serde_json::Value::Number(n) => {
            // serde_json::Number can be i64, u64, or f64.
            if let Some(i) = n.as_i64() {
                Ok(i.into_py_any(py)?)
            } else if let Some(u) = n.as_u64() {
                Ok(u.into_py_any(py)?)
            } else if let Some(f) = n.as_f64() {
                Ok(PyFloat::new(py, f).into())
            } else {
                // Should be unreachable, but don't panic inside bindings.
                Ok(py.None())
            }
        }
        serde_json::Value::String(s) => Ok(PyString::new(py, s).into()),
        serde_json::Value::Array(arr) => {
            let mut items = Vec::with_capacity(arr.len());
            for v in arr {
                items.push(json_to_py(py, v)?);
            }
            Ok(PyList::new(py, items)?.into())
        }
        serde_json::Value::Object(obj) => {
            let dict = PyDict::new(py);
            for (k, v) in obj {
                dict.set_item(k, json_to_py(py, v)?)?;
            }
            Ok(dict.into())
        }
    }
}

/// Helper to convert PyAny to serde_json::Value
pub(crate) fn py_to_json(value: &Bound<'_, PyAny>) -> pyo3::PyResult<serde_json::Value> {
    if value.is_none() {
        Ok(serde_json::Value::Null)
    } else if let Ok(b) = value.extract::<bool>() {
        // Check bool before numbers (bool is a subtype of int in Python)
        Ok(serde_json::json!(b))
    } else if let Ok(i) = value.extract::<i64>() {
        Ok(serde_json::json!(i))
    } else if let Ok(f) = value.extract::<f64>() {
        Ok(serde_json::json!(f))
    } else if let Ok(s) = value.extract::<String>() {
        Ok(serde_json::json!(s))
    } else if let Ok(dict) = value.downcast::<PyDict>() {
        // Handle dictionaries recursively
        let mut map = serde_json::Map::new();
        for (key, val) in dict.iter() {
            let key_str = key.extract::<String>()?;
            let val_json = py_to_json(&val)?;
            map.insert(key_str, val_json);
        }
        Ok(serde_json::Value::Object(map))
    } else if let Ok(list) = value.downcast::<PyList>() {
        // Handle lists recursively
        let mut arr = Vec::new();
        for item in list.iter() {
            arr.push(py_to_json(&item)?);
        }
        Ok(serde_json::Value::Array(arr))
    } else {
        let type_name = value.get_type().name()?.to_string();
        Err(PyTypeError::new_err(format!(
            "Value is not JSON-serializable (got {type_name})"
        )))
    }
}
