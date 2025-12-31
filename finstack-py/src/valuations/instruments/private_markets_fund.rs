use crate::core::currency::PyCurrency;
use crate::core::dates::utils::date_to_py;
use crate::core::money::PyMoney;
use crate::errors::core_to_py;
use crate::valuations::common::PyInstrumentType;
use finstack_valuations::instruments::equity::pe_fund::PrivateMarketsFund;
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyModule, PyType};
use pyo3::Bound;
use std::fmt;

fn parse_pmf_json(value: &Bound<'_, PyAny>) -> PyResult<PrivateMarketsFund> {
    if let Ok(json_str) = value.extract::<&str>() {
        return serde_json::from_str(json_str)
            .map_err(|err| PyValueError::new_err(err.to_string()));
    }
    if let Ok(dict) = value.downcast::<PyDict>() {
        use crate::errors::PyContext;
        let py = dict.py();
        let json = pyo3::types::PyModule::import(py, "json")?
            .call_method1("dumps", (dict,))?
            .extract::<String>()
            .context("json dumps")?;
        return serde_json::from_str(&json).map_err(|err| PyValueError::new_err(err.to_string()));
    }
    Err(PyTypeError::new_err(
        "Expected JSON string or dict convertible to JSON",
    ))
}

/// Private markets fund instrument wrapper parsed from JSON definitions.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "PrivateMarketsFund",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyPrivateMarketsFund {
    pub(crate) inner: PrivateMarketsFund,
}

impl PyPrivateMarketsFund {
    pub(crate) fn new(inner: PrivateMarketsFund) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyPrivateMarketsFund {
    #[classmethod]
    #[pyo3(text_signature = "(cls, data)")]
    fn from_json(_cls: &Bound<'_, PyType>, data: Bound<'_, PyAny>) -> PyResult<Self> {
        let fund = parse_pmf_json(&data)?;
        Ok(Self::new(fund))
    }

    #[pyo3(text_signature = "(self)")]
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string_pretty(&self.inner)
            .map_err(|err| PyValueError::new_err(err.to_string()))
    }

    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::PrivateMarketsFund)
    }

    #[getter]
    fn currency(&self) -> PyCurrency {
        PyCurrency::new(self.inner.currency)
    }

    #[getter]
    fn discount_curve(&self) -> Option<String> {
        self.inner
            .discount_curve_id
            .as_ref()
            .map(|id| id.as_str().to_string())
    }

    #[pyo3(text_signature = "(self)")]
    fn lp_cashflows(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let flows = self.inner.lp_cashflows().map_err(core_to_py)?;
        let items: PyResult<Vec<(Py<PyAny>, PyMoney)>> = flows
            .into_iter()
            .map(|(date, amt)| {
                let py_date = date_to_py(py, date)?;
                let money = PyMoney::new(amt);
                Ok((py_date, money))
            })
            .collect();
        Ok(PyList::new(py, items?)?.into())
    }

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!(
            "PrivateMarketsFund(id='{}', events={})",
            self.inner.id,
            self.inner.events.len()
        ))
    }
}

impl fmt::Display for PyPrivateMarketsFund {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "PrivateMarketsFund({}, events={})",
            self.inner.id,
            self.inner.events.len()
        )
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyPrivateMarketsFund>()?;
    module.setattr(
        "__doc__",
        "Private markets fund instrument parsed from JSON definitions (LP cashflows, waterfall specs).",
    )?;
    Ok(vec!["PrivateMarketsFund"])
}
