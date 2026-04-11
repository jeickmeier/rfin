//! Python bindings for the `finstack-valuations` crate.
//!
//! Exposes the [`PyValuationResult`] envelope for pricing output,
//! JSON-based instrument loading, and the standard pricer pipeline.

mod pricing;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyList;

/// Parse an ISO 8601 date string into a `time::Date`.
fn parse_date(s: &str) -> PyResult<time::Date> {
    let format = time::format_description::well_known::Iso8601::DEFAULT;
    time::Date::parse(s, &format)
        .map_err(|e| PyValueError::new_err(format!("Invalid date '{s}': {e}")))
}

// ---------------------------------------------------------------------------
// ValuationResult
// ---------------------------------------------------------------------------

#[pyclass(
    name = "ValuationResult",
    module = "finstack.valuations",
    skip_from_py_object
)]
#[derive(Clone)]
struct PyValuationResult {
    pub(crate) inner: finstack_valuations::results::ValuationResult,
}

#[pymethods]
impl PyValuationResult {
    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        let inner: finstack_valuations::results::ValuationResult =
            serde_json::from_str(json).map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(Self { inner })
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string_pretty(&self.inner).map_err(|e| PyValueError::new_err(e.to_string()))
    }

    #[getter]
    fn instrument_id(&self) -> &str {
        &self.inner.instrument_id
    }

    #[getter]
    fn get_price(&self) -> f64 {
        self.inner.value.amount()
    }

    #[getter]
    fn currency(&self) -> String {
        self.inner.value.currency().to_string()
    }

    fn get_metric(&self, key: &str) -> Option<f64> {
        self.inner.metric_str(key)
    }

    fn metric_keys(&self) -> Vec<String> {
        self.inner.measures.keys().map(|k| k.to_string()).collect()
    }

    fn metric_count(&self) -> usize {
        self.inner.measures.len()
    }

    fn all_covenants_passed(&self) -> bool {
        self.inner.all_covenants_passed()
    }

    fn failed_covenants(&self) -> Vec<String> {
        self.inner
            .failed_covenants()
            .into_iter()
            .map(String::from)
            .collect()
    }

    fn __repr__(&self) -> String {
        format!(
            "ValuationResult(id={:?}, price={:.4}, currency={}, metrics={})",
            self.inner.instrument_id,
            self.inner.value.amount(),
            self.inner.value.currency(),
            self.inner.measures.len()
        )
    }
}

// ---------------------------------------------------------------------------
// InstrumentJson — tagged-union loader
// ---------------------------------------------------------------------------

#[pyfunction]
fn validate_instrument_json(json: &str) -> PyResult<String> {
    let parsed: finstack_valuations::instruments::InstrumentJson = serde_json::from_str(json)
        .map_err(|e| PyValueError::new_err(format!("invalid instrument JSON: {e}")))?;
    serde_json::to_string_pretty(&parsed).map_err(|e| PyValueError::new_err(e.to_string()))
}

// ---------------------------------------------------------------------------
// Module registration
// ---------------------------------------------------------------------------

pub fn register(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(py, "valuations")?;
    m.setattr(
        "__doc__",
        "Instrument pricing: bonds, swaps, options, calibration, attribution.",
    )?;

    m.add_class::<PyValuationResult>()?;
    m.add_function(wrap_pyfunction!(validate_instrument_json, &m)?)?;
    pricing::register(py, &m)?;

    let all = PyList::new(
        py,
        [
            "ValuationResult",
            "validate_instrument_json",
            "price_instrument",
            "price_instrument_with_metrics",
            "list_standard_metrics",
        ],
    )?;
    m.setattr("__all__", all)?;
    parent.add_submodule(&m)?;

    let pkg: String = match parent.getattr("__package__") {
        Ok(attr) => match attr.extract::<String>() {
            Ok(s) => s,
            Err(_) => "finstack".to_string(),
        },
        Err(_) => "finstack".to_string(),
    };
    let qual = format!("{pkg}.valuations");
    m.setattr("__package__", &qual)?;
    let sys = PyModule::import(py, "sys")?;
    let modules = sys.getattr("modules")?;
    modules.set_item(&qual, &m)?;

    Ok(())
}
