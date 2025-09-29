use crate::valuations::common::PyInstrumentType;
use finstack_valuations::instruments::abs::Abs;
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyModule, PyType};
use std::fmt;

fn parse_abs_json(value: &Bound<'_, PyAny>) -> PyResult<Abs> {
    if let Ok(json_str) = value.extract::<&str>() {
        serde_json::from_str(json_str).map_err(|err| PyValueError::new_err(err.to_string()))
    } else {
        // Accept Python dict-like object; convert to string via __repr__ and parse
        let json_str: String = value.str()?.to_string_lossy().into_owned();
        serde_json::from_str(&json_str)
            .map_err(|_| PyTypeError::new_err("Expected JSON string or dict convertible to JSON"))
    }
}

/// ABS instrument wrapper parsed from JSON definitions.
///
/// Examples:
///     >>> deal = Abs.from_json(json.dumps({...}))
///     >>> deal.instrument_id
///     'abs_portfolio_001'
#[pyclass(module = "finstack.valuations.instruments", name = "Abs", frozen)]
#[derive(Clone, Debug)]
pub struct PyAbs {
    pub(crate) inner: Abs,
}

impl PyAbs {
    pub(crate) fn new(inner: Abs) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyAbs {
    #[classmethod]
    #[pyo3(text_signature = "(cls, data)")]
    /// Parse a JSON payload into an ABS instrument.
    ///
    /// Args:
    ///     data: JSON string or dict describing the ABS structure.
    ///
    /// Returns:
    ///     Abs: Parsed ABS instrument wrapper.
    ///
    /// Raises:
    ///     ValueError: If the JSON cannot be parsed.
    ///     TypeError: If ``data`` is neither a string nor dict-like object.
    fn from_json(_cls: &Bound<'_, PyType>, data: Bound<'_, PyAny>) -> PyResult<Self> {
        let deal = parse_abs_json(&data)?;
        Ok(Self::new(deal))
    }

    /// Instrument identifier.
    ///
    /// Returns:
    ///     str: Unique identifier assigned to the ABS deal.
    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    /// Instrument type enumeration.
    ///
    /// Returns:
    ///     InstrumentType: Enumeration value ``InstrumentType.ABS``.
    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::ABS)
    }

    #[pyo3(text_signature = "(self)")]
    /// Serialize the ABS definition back to JSON.
    ///
    /// Returns:
    ///     str: Pretty-printed JSON representation of the instrument.
    ///
    /// Raises:
    ///     ValueError: If serialization fails.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string_pretty(&self.inner)
            .map_err(|err| PyValueError::new_err(err.to_string()))
    }
}

impl fmt::Display for PyAbs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Abs({}, tranches={})",
            self.inner.id,
            self.inner.tranches.tranches.len()
        )
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyAbs>()?;
    Ok(vec!["Abs"])
}
