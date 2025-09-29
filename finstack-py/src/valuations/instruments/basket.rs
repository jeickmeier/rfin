use crate::valuations::common::PyInstrumentType;
use finstack_valuations::instruments::basket::Basket;
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyModule, PyType};
use std::fmt;

fn parse_json(value: &Bound<'_, PyAny>) -> PyResult<Basket> {
    if let Ok(json_str) = value.extract::<&str>() {
        serde_json::from_str(json_str).map_err(|err| PyValueError::new_err(err.to_string()))
    } else {
        let json_str: String = value.str()?.to_string_lossy().into_owned();
        serde_json::from_str(&json_str)
            .map_err(|_| PyTypeError::new_err("Expected JSON string or dict convertible to JSON"))
    }
}

/// Basket instrument wrapper parsed from JSON definitions.
#[pyclass(module = "finstack.valuations.instruments", name = "Basket", frozen)]
#[derive(Clone, Debug)]
pub struct PyBasket {
    pub(crate) inner: Basket,
}

impl PyBasket {
    pub(crate) fn new(inner: Basket) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyBasket {
    #[classmethod]
    #[pyo3(text_signature = "(cls, data)")]
    /// Parse a basket definition from a JSON string or dictionary matching the Rust schema.
    fn from_json(_cls: &Bound<'_, PyType>, data: Bound<'_, PyAny>) -> PyResult<Self> {
        let basket = parse_json(&data)?;
        if basket.id.as_str().is_empty() {
            return Err(PyValueError::new_err(
                "Basket JSON must include a non-empty 'id' field",
            ));
        }
        Ok(Self::new(basket))
    }

    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::Basket)
    }

    #[pyo3(text_signature = "(self)")]
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string_pretty(&self.inner)
            .map_err(|err| PyValueError::new_err(err.to_string()))
    }
}

impl fmt::Display for PyBasket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Basket({}, constituents={})",
            self.inner.id,
            self.inner.constituents.len()
        )
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyBasket>()?;
    Ok(vec!["Basket"])
}
