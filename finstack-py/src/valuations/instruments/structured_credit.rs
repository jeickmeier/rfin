use crate::valuations::common::PyInstrumentType;
use finstack_valuations::instruments::structured_credit::StructuredCredit;
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyModule, PyType};
use std::fmt;

fn parse_structured_credit_json(value: &Bound<'_, PyAny>) -> PyResult<StructuredCredit> {
    if let Ok(json_str) = value.extract::<&str>() {
        serde_json::from_str(json_str).map_err(|err| PyValueError::new_err(err.to_string()))
    } else {
        // Accept Python dict-like object; convert to string via __repr__ and parse
        let json_str: String = value.str()?.to_string_lossy().into_owned();
        serde_json::from_str(&json_str)
            .map_err(|_| PyTypeError::new_err("Expected JSON string or dict convertible to JSON"))
    }
}

/// Unified structured credit instrument wrapper (ABS, CLO, CMBS, RMBS).
///
/// This single Python class provides a cleaner API with deal type discrimination.
///
/// Examples:
///     >>> deal = StructuredCredit.from_json(json.dumps({...}))
///     >>> deal.instrument_id
///     'clo_2024_1'
///     >>> deal.deal_type
///     'CLO'
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "StructuredCredit",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyStructuredCredit {
    pub(crate) inner: StructuredCredit,
}

impl PyStructuredCredit {
    pub(crate) fn new(inner: StructuredCredit) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyStructuredCredit {
    #[classmethod]
    #[pyo3(text_signature = "(cls, data)")]
    /// Parse a JSON payload into a structured credit instrument.
    ///
    /// Args:
    ///     data: JSON string or dict describing the structured credit deal.
    ///
    /// Returns:
    ///     StructuredCredit: Parsed structured credit instrument wrapper.
    ///
    /// Raises:
    ///     ValueError: If the JSON cannot be parsed.
    ///     TypeError: If ``data`` is neither a string nor dict-like object.
    fn from_json(_cls: &Bound<'_, PyType>, data: Bound<'_, PyAny>) -> PyResult<Self> {
        let deal = parse_structured_credit_json(&data)?;
        Ok(Self::new(deal))
    }

    /// Instrument identifier.
    ///
    /// Returns:
    ///     str: Unique identifier assigned to the deal.
    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    /// Deal type classification (ABS, CLO, CMBS, or RMBS).
    ///
    /// Returns:
    ///     str: Deal type string.
    #[getter]
    fn deal_type(&self) -> String {
        format!("{:?}", self.inner.deal_type)
    }

    /// Instrument type enumeration.
    ///
    /// Returns:
    ///     InstrumentType: Enumeration value ``InstrumentType.StructuredCredit``.
    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::StructuredCredit)
    }

    #[pyo3(text_signature = "(self)")]
    /// Serialize the structured credit definition back to JSON.
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

    /// Number of tranches in the structure.
    ///
    /// Returns:
    ///     int: Count of tranches.
    #[getter]
    fn tranche_count(&self) -> usize {
        self.inner.tranches.tranches.len()
    }
}

impl fmt::Display for PyStructuredCredit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "StructuredCredit({:?}, id={}, tranches={})",
            self.inner.deal_type,
            self.inner.id,
            self.inner.tranches.tranches.len()
        )
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyStructuredCredit>()?;
    Ok(vec!["StructuredCredit"])
}
