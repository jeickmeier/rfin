use crate::valuations::common::PyInstrumentType;
use finstack_valuations::instruments::cmbs::Cmbs;
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyModule, PyType};
use pyo3::Bound;
use std::fmt;

fn parse_cmbs_json(value: &Bound<'_, PyAny>) -> PyResult<Cmbs> {
    if let Ok(json_str) = value.extract::<&str>() {
        serde_json::from_str(json_str).map_err(|err| PyValueError::new_err(err.to_string()))
    } else {
        let json_str = value.str()?.to_string_lossy().into_owned();
        serde_json::from_str(&json_str)
            .map_err(|_| PyTypeError::new_err("Expected JSON string or dict convertible to JSON"))
    }
}

/// CMBS instrument wrapper parsed from JSON definitions.
#[pyclass(module = "finstack.valuations.instruments", name = "Cmbs", frozen)]
#[derive(Clone, Debug)]
pub struct PyCmbs {
    pub(crate) inner: Cmbs,
}

impl PyCmbs {
    pub(crate) fn new(inner: Cmbs) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyCmbs {
    #[classmethod]
    #[pyo3(text_signature = "(cls, data)")]
    fn from_json(_cls: &Bound<'_, PyType>, data: Bound<'_, PyAny>) -> PyResult<Self> {
        let deal = parse_cmbs_json(&data)?;
        Ok(Self::new(deal))
    }

    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::CMBS)
    }

    #[pyo3(text_signature = "(self)")]
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string_pretty(&self.inner)
            .map_err(|err| PyValueError::new_err(err.to_string()))
    }

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!(
            "Cmbs(id='{}', tranches={})",
            self.inner.id,
            self.inner.tranches.tranches.len()
        ))
    }
}

impl fmt::Display for PyCmbs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Cmbs({}, tranches={})",
            self.inner.id,
            self.inner.tranches.tranches.len()
        )
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyCmbs>()?;
    Ok(vec!["Cmbs"])
}
