use crate::core::config::PyRoundingMode;
use crate::core::dates::utils as core_utils;
use crate::core::money::PyMoney;
use finstack_core::config::RoundingContext;
use finstack_valuations::covenants::CovenantReport;
use finstack_valuations::results::{ResultsMeta, ValuationResult};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyModule};
use pyo3::Bound;
use pythonize::pythonize;
use std::fmt;

/// Covenant evaluation outcome attached to a valuation result.
///
/// Examples:
///     >>> report = valuation_result.covenants['ltv']
///     >>> report.passed
///     True
#[pyclass(
    module = "finstack.valuations.results",
    name = "CovenantReport",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyCovenantReport {
    pub(crate) inner: CovenantReport,
}

impl PyCovenantReport {
    pub(crate) fn new(inner: CovenantReport) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyCovenantReport {
    /// Covenant identifier describing the check performed.
    ///
    /// Returns:
    ///     str: Covenant label supplied by the originating configuration.
    #[getter]
    fn covenant_type(&self) -> &str {
        &self.inner.covenant_type
    }

    /// Whether the covenant passed for the evaluated scenario.
    ///
    /// Returns:
    ///     bool: ``True`` when the covenant conditions are satisfied.
    #[getter]
    fn passed(&self) -> bool {
        self.inner.passed
    }

    /// Observed metric value when available.
    ///
    /// Returns:
    ///     float | None: Realized metric value used in the check.
    #[getter]
    fn actual_value(&self) -> Option<f64> {
        self.inner.actual_value
    }

    /// Required threshold for the covenant, when provided.
    ///
    /// Returns:
    ///     float | None: Target threshold or limit for the covenant.
    #[getter]
    fn threshold(&self) -> Option<f64> {
        self.inner.threshold
    }

    /// Additional free-form details attached to the report.
    ///
    /// Returns:
    ///     str | None: Supplemental information captured during evaluation.
    #[getter]
    fn details(&self) -> Option<&str> {
        self.inner.details.as_deref()
    }

    fn __repr__(&self) -> String {
        let status = if self.inner.passed {
            "passed"
        } else {
            "failed"
        };
        format!(
            "CovenantReport(type='{}', status={status})",
            self.inner.covenant_type
        )
    }
}

impl fmt::Display for PyCovenantReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let status = if self.inner.passed {
            "passed"
        } else {
            "failed"
        };
        write!(f, "{} ({status})", self.inner.covenant_type)
    }
}

/// Snapshot describing numeric mode, rounding context, and FX policy applied to results.
///
/// Examples:
///     >>> meta.numeric_mode
///     'f64'
#[pyclass(module = "finstack.valuations.results", name = "ResultsMeta", frozen)]
#[derive(Clone, Debug)]
pub struct PyResultsMeta {
    pub(crate) inner: ResultsMeta,
}

impl PyResultsMeta {
    pub(crate) fn new(inner: ResultsMeta) -> Self {
        Self { inner }
    }

    fn rounding_to_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let dict = PyDict::new(py);
        let mode = PyRoundingMode::new(self.inner.rounding.mode);
        dict.set_item("mode", mode.to_string())?;
        dict.set_item("version", self.inner.rounding.version)?;
        dict.set_item(
            "ingest_scale_by_ccy",
            map_currency_scales(py, &self.inner.rounding, true)?,
        )?;
        dict.set_item(
            "output_scale_by_ccy",
            map_currency_scales(py, &self.inner.rounding, false)?,
        )?;
        Ok(dict)
    }
}

#[pymethods]
impl PyResultsMeta {
    /// Numeric engine mode used by the pricing engine (e.g., ``"f64"``).
    ///
    /// Returns:
    ///     str: Symbol representing the numeric precision.
    #[getter]
    fn numeric_mode(&self) -> &'static str {
        match self.inner.numeric_mode {
            finstack_core::config::NumericMode::F64 => "f64",
            _ => "f64",
        }
    }

    /// Optional FX policy key applied during result aggregation.
    ///
    /// Returns:
    ///     str | None: FX policy identifier or ``None`` when not applied.
    #[getter]
    fn fx_policy_applied(&self) -> Option<&str> {
        self.inner.fx_policy_applied.as_deref()
    }

    /// Timestamp when result was computed (ISO 8601 format).
    ///
    /// Returns:
    ///     str | None: Timestamp of computation for audit trails.
    #[getter]
    fn timestamp(&self) -> Option<String> {
        self.inner.timestamp.map(|t| {
            t.format(&time::format_description::well_known::Iso8601::DEFAULT)
                .unwrap_or_else(|_| "unknown".to_string())
        })
    }

    /// Finstack library version used to produce this result.
    ///
    /// Returns:
    ///     str | None: Library version for reproducibility.
    #[getter]
    fn version(&self) -> Option<&str> {
        self.inner.version.as_deref()
    }

    /// Rounding context snapshot as a dictionary.
    ///
    /// Returns:
    ///     dict: Dictionary containing rounding mode and per-currency scales.
    #[getter]
    fn rounding<'py>(&self, py: Python<'py>) -> PyResult<PyObject> {
        Ok(self.rounding_to_dict(py)?.into())
    }

    #[pyo3(text_signature = "(self)")]
    /// Convert the metadata to a Python dictionary for downstream serialization.
    ///
    /// Returns:
    ///     dict: Serializable snapshot of metadata fields.
    ///
    /// Examples:
    ///     >>> meta.to_dict()['numeric_mode']
    ///     'f64'
    fn to_dict<'py>(&self, py: Python<'py>) -> PyResult<PyObject> {
        let dict = PyDict::new(py);
        dict.set_item("numeric_mode", self.numeric_mode())?;
        dict.set_item("rounding", self.rounding_to_dict(py)?)?;
        if let Some(policy) = &self.inner.fx_policy_applied {
            dict.set_item("fx_policy_applied", policy.clone())?;
        } else {
            dict.set_item("fx_policy_applied", py.None())?;
        }
        if let Some(timestamp) = &self.inner.timestamp {
            let ts_str = timestamp
                .format(&time::format_description::well_known::Iso8601::DEFAULT)
                .unwrap_or_else(|_| "unknown".to_string());
            dict.set_item("timestamp", ts_str)?;
        }
        if let Some(version) = &self.inner.version {
            dict.set_item("version", version.clone())?;
        }
        Ok(dict.into())
    }

    fn __repr__(&self) -> PyResult<String> {
        let policy = self.inner.fx_policy_applied.as_deref().unwrap_or("none");
        Ok(format!(
            "ResultsMeta(mode='{}', fx_policy='{}')",
            self.numeric_mode(),
            policy
        ))
    }
}

impl fmt::Display for PyResultsMeta {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let policy = self.inner.fx_policy_applied.as_deref().unwrap_or("none");
        write!(f, "{} (fx_policy={policy})", self.numeric_mode())
    }
}

/// Complete valuation output including PV, measures, metadata, and covenant reports.
///
/// Examples:
///     >>> result.value.amount
///     123.45
#[pyclass(
    module = "finstack.valuations.results",
    name = "ValuationResult",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyValuationResult {
    pub(crate) inner: ValuationResult,
}

impl PyValuationResult {
    pub(crate) fn new(inner: ValuationResult) -> Self {
        Self { inner }
    }

    fn measures_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let dict = PyDict::new(py);
        for (key, value) in &self.inner.measures {
            dict.set_item(key, *value)?;
        }
        Ok(dict)
    }

    fn covenants_dict<'py>(&self, py: Python<'py>) -> PyResult<Option<PyObject>> {
        if let Some(reports) = &self.inner.covenants {
            let dict = PyDict::new(py);
            for (name, report) in reports {
                dict.set_item(name, PyCovenantReport::new(report.clone()))?;
            }
            Ok(Some(dict.into()))
        } else {
            Ok(None)
        }
    }
}

#[pymethods]
impl PyValuationResult {
    /// Instrument identifier used when stamping the result.
    ///
    /// Returns:
    ///     str: Unique instrument identifier supplied at pricing time.
    #[getter]
    fn instrument_id(&self) -> &str {
        &self.inner.instrument_id
    }

    /// Valuation date associated with the pricing run.
    ///
    /// Returns:
    ///     datetime.date: Effective market date for the valuation.
    #[getter]
    fn as_of(&self, py: Python<'_>) -> PyResult<PyObject> {
        core_utils::date_to_py(py, self.inner.as_of)
    }

    /// Present value expressed as :class:`finstack.core.money.Money`.
    ///
    /// Returns:
    ///     Money: Present value of the instrument.
    #[getter]
    fn value(&self) -> PyMoney {
        PyMoney::new(self.inner.value)
    }

    /// Dictionary of computed measures (e.g., ``{"dv01": 1250.0}``).
    ///
    /// Returns:
    ///     dict[str, float]: Calculated risk measures keyed by metric id.
    #[getter]
    fn measures<'py>(&self, py: Python<'py>) -> PyResult<PyObject> {
        Ok(self.measures_dict(py)?.into())
    }

    /// Metadata describing numeric mode, rounding context, and FX policy.
    ///
    /// Returns:
    ///     ResultsMeta: Snapshot of metadata associated with the valuation.
    #[getter]
    fn meta(&self) -> PyResultsMeta {
        PyResultsMeta::new(self.inner.meta.clone())
    }

    /// Covenant reports (if any) keyed by covenant identifier.
    ///
    /// Returns:
    ///     dict[str, CovenantReport] | None: Covenant evaluations when available.
    #[getter]
    fn covenants<'py>(&self, py: Python<'py>) -> PyResult<Option<PyObject>> {
        self.covenants_dict(py)
    }

    /// Optional explanation trace when explain=True was passed.
    ///
    /// Returns:
    ///     dict | None: Detailed trace of computation steps, or None if explanation was disabled.
    ///
    /// Examples:
    ///     >>> result = pricer.price(bond, market, as_of, explain=True)
    ///     >>> if result.explanation:
    ///     ...     print(result.explanation['type'])
    ///     pricing
    #[getter]
    fn explanation(&self, py: Python<'_>) -> PyResult<Option<PyObject>> {
        match &self.inner.explanation {
            Some(trace) => {
                let bound = pythonize(py, trace).map_err(|e| {
                    PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string())
                })?;
                Ok(Some(bound.unbind()))
            }
            None => Ok(None),
        }
    }

    #[pyo3(text_signature = "(self)")]
    /// Get the explanation trace as pretty-printed JSON.
    ///
    /// Returns:
    ///     str | None: JSON string of the explanation, or None if disabled.
    ///
    /// Examples:
    ///     >>> result = pricer.price(bond, market, as_of, explain=True)
    ///     >>> if result.explanation:
    ///     ...     print(result.explain_json())
    fn explain_json(&self) -> PyResult<Option<String>> {
        match &self.inner.explanation {
            Some(trace) => {
                let json = trace.to_json_pretty().map_err(|e| {
                    PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string())
                })?;
                Ok(Some(json))
            }
            None => Ok(None),
        }
    }

    #[pyo3(text_signature = "(self)")]
    /// Convenience helper returning ``True`` when all covenants passed.
    ///
    /// Returns:
    ///     bool: ``True`` when there are no failing covenant reports.
    ///
    /// Examples:
    ///     >>> result.all_covenants_passed()
    ///     True
    fn all_covenants_passed(&self) -> bool {
        self.inner.all_covenants_passed()
    }

    #[pyo3(text_signature = "(self)")]
    /// List of covenant identifiers that failed (empty when all pass).
    ///
    /// Returns:
    ///     list[str]: Identifiers for covenants that evaluated to false.
    fn failed_covenants(&self) -> Vec<&str> {
        self.inner.failed_covenants()
    }

    #[pyo3(text_signature = "(self)")]
    /// Convert to a Python dictionary for JSON/Arrow serialization.
    ///
    /// Returns:
    ///     dict: Serializable dictionary containing the valuation payload.
    ///
    /// Examples:
    ///     >>> data = result.to_dict()
    ///     >>> sorted(data.keys())
    ///     ['as_of', 'covenants', 'instrument_id', 'measures', 'meta', 'value']
    fn to_dict<'py>(&self, py: Python<'py>) -> PyResult<PyObject> {
        let dict = PyDict::new(py);
        dict.set_item("instrument_id", &self.inner.instrument_id)?;
        dict.set_item("as_of", self.as_of(py)?)?;
        dict.set_item("value", PyMoney::new(self.inner.value))?;
        dict.set_item("measures", self.measures_dict(py)?)?;
        dict.set_item("meta", self.meta().to_dict(py)?)?;
        match self.covenants_dict(py)? {
            Some(obj) => dict.set_item("covenants", obj)?,
            None => dict.set_item("covenants", py.None())?,
        }
        Ok(dict.into())
    }

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!(
            "ValuationResult(id='{}', pv={}, measures={})",
            self.inner.instrument_id,
            self.inner.value,
            self.inner.measures.len()
        ))
    }
}

impl fmt::Display for PyValuationResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ValuationResult({}, PV={}, measures={})",
            self.inner.instrument_id,
            self.inner.value,
            self.inner.measures.len()
        )
    }
}

fn map_currency_scales<'py>(
    py: Python<'py>,
    ctx: &RoundingContext,
    ingest: bool,
) -> PyResult<Bound<'py, PyDict>> {
    let dict = PyDict::new(py);
    let source = if ingest {
        &ctx.ingest_scale_by_ccy
    } else {
        &ctx.output_scale_by_ccy
    };
    for (currency, scale) in source {
        dict.set_item(format!("{currency}"), *scale)?;
    }
    Ok(dict)
}

pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "results")?;
    module.setattr(
        "__doc__",
        "Valuation result envelopes, metadata, and covenant report bindings.",
    )?;
    module.add_class::<PyValuationResult>()?;
    module.add_class::<PyResultsMeta>()?;
    module.add_class::<PyCovenantReport>()?;
    let exports = ["ValuationResult", "ResultsMeta", "CovenantReport"];
    module.setattr("__all__", PyList::new(py, &exports)?)?;
    parent.add_submodule(&module)?;
    Ok(exports.to_vec())
}
