use crate::valuations::common::PyInstrumentType;
use finstack_valuations::instruments::basket::Basket;
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyModule, PyType};
use std::fmt;

fn parse_json(value: &Bound<'_, PyAny>) -> PyResult<Basket> {
    if let Ok(json_str) = value.extract::<&str>() {
        return serde_json::from_str(json_str)
            .map_err(|err| PyValueError::new_err(err.to_string()));
    }
    if let Ok(dict) = value.downcast::<PyDict>() {
        let py = dict.py();
        let json = pyo3::types::PyModule::import(py, "json")?
            .call_method1("dumps", (dict,))?
            .extract::<String>()?;
        return serde_json::from_str(&json).map_err(|err| PyValueError::new_err(err.to_string()));
    }
    Err(PyTypeError::new_err(
        "Expected JSON string or dict convertible to JSON",
    ))
}

/// Basket instrument wrapper parsed from JSON definitions.
///
/// Examples:
///     >>> basket = Basket.from_json(json.dumps({...}))
///     >>> basket.instrument_type.name
///     'basket'
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
    /// Parse a basket definition from a JSON string or dictionary.
    ///
    /// Args:
    ///     data: JSON string or dict describing the basket constituents.
    ///
    /// Returns:
    ///     Basket: Parsed basket instrument.
    ///
    /// Raises:
    ///     ValueError: If parsing fails or the basket ID is missing.
    ///     TypeError: If ``data`` is neither a string nor dict-like object.
    fn from_json(_cls: &Bound<'_, PyType>, data: Bound<'_, PyAny>) -> PyResult<Self> {
        let basket = parse_json(&data)?;
        if basket.id.as_str().is_empty() {
            return Err(PyValueError::new_err(
                "Basket JSON must include a non-empty 'id' field",
            ));
        }
        Ok(Self::new(basket))
    }

    /// Instrument identifier.
    ///
    /// Returns:
    ///     str: Unique identifier assigned to the basket.
    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    /// Instrument type enumeration.
    ///
    /// Returns:
    ///     InstrumentType: Enumeration value ``InstrumentType.Basket``.
    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::Basket)
    }

    #[pyo3(text_signature = "(self)")]
    /// Serialize the basket definition to a JSON string.
    ///
    /// Returns:
    ///     str: Pretty-printed JSON representation.
    ///
    /// Raises:
    ///     ValueError: If serialization fails.
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
