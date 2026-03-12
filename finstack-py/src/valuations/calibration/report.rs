use super::config::PySolverKind;
use finstack_valuations::calibration::{CalibrationDiagnostics, CalibrationReport, QuoteQuality};
use pyo3::prelude::*;
use pyo3::types::{IntoPyDict, PyDict, PyModule};
use pyo3::Bound;
use pythonize::pythonize;

#[pyclass(
    module = "finstack.valuations.calibration",
    name = "CalibrationReport",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyCalibrationReport {
    pub(crate) inner: CalibrationReport,
}

impl PyCalibrationReport {
    pub(crate) fn new(inner: CalibrationReport) -> Self {
        Self { inner }
    }

    fn map_to_dict<'py>(
        py: Python<'py>,
        map: &std::collections::BTreeMap<String, f64>,
    ) -> PyResult<Bound<'py, PyDict>> {
        let dict = PyDict::new(py);
        for (key, value) in map.iter() {
            dict.set_item(key, value)?;
        }
        Ok(dict)
    }

    fn string_map_to_dict<'py>(
        py: Python<'py>,
        map: &std::collections::BTreeMap<String, String>,
    ) -> PyResult<Bound<'py, PyDict>> {
        let dict = PyDict::new(py);
        for (key, value) in map.iter() {
            dict.set_item(key, value)?;
        }
        Ok(dict)
    }

    fn quote_quality_to_dict<'py>(
        py: Python<'py>,
        quote: &QuoteQuality,
    ) -> PyResult<Bound<'py, PyDict>> {
        let dict = PyDict::new(py);
        dict.set_item("quote_label", &quote.quote_label)?;
        dict.set_item("target_value", quote.target_value)?;
        dict.set_item("fitted_value", quote.fitted_value)?;
        dict.set_item("residual", quote.residual)?;
        dict.set_item("sensitivity", quote.sensitivity)?;
        Ok(dict)
    }

    fn diagnostics_to_dict<'py>(
        py: Python<'py>,
        diagnostics: &CalibrationDiagnostics,
    ) -> PyResult<Bound<'py, PyDict>> {
        let dict = PyDict::new(py);
        let per_quote: Vec<_> = diagnostics
            .per_quote
            .iter()
            .map(|quote| Self::quote_quality_to_dict(py, quote))
            .collect::<PyResult<_>>()?;
        dict.set_item("per_quote", per_quote)?;
        dict.set_item("condition_number", diagnostics.condition_number)?;
        dict.set_item("singular_values", diagnostics.singular_values.clone())?;
        dict.set_item("max_residual", diagnostics.max_residual)?;
        dict.set_item("rms_residual", diagnostics.rms_residual)?;
        dict.set_item("r_squared", diagnostics.r_squared)?;
        Ok(dict)
    }
}

#[pyclass(
    module = "finstack.valuations.calibration",
    name = "QuoteQuality",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyQuoteQuality {
    pub(crate) inner: QuoteQuality,
}

impl PyQuoteQuality {
    pub(crate) fn new(inner: QuoteQuality) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyQuoteQuality {
    #[new]
    fn py_new(
        quote_label: String,
        target_value: f64,
        fitted_value: f64,
        residual: f64,
        sensitivity: f64,
    ) -> Self {
        Self::new(QuoteQuality {
            quote_label,
            target_value,
            fitted_value,
            residual,
            sensitivity,
        })
    }

    #[getter]
    fn quote_label(&self) -> &str {
        &self.inner.quote_label
    }

    #[getter]
    fn target_value(&self) -> f64 {
        self.inner.target_value
    }

    #[getter]
    fn fitted_value(&self) -> f64 {
        self.inner.fitted_value
    }

    #[getter]
    fn residual(&self) -> f64 {
        self.inner.residual
    }

    #[getter]
    fn sensitivity(&self) -> f64 {
        self.inner.sensitivity
    }

    fn __repr__(&self) -> String {
        format!(
            "QuoteQuality(quote_label='{}', residual={})",
            self.inner.quote_label, self.inner.residual
        )
    }
}

#[pyclass(
    module = "finstack.valuations.calibration",
    name = "CalibrationDiagnostics",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyCalibrationDiagnostics {
    pub(crate) inner: CalibrationDiagnostics,
}

impl PyCalibrationDiagnostics {
    pub(crate) fn new(inner: CalibrationDiagnostics) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyCalibrationDiagnostics {
    #[new]
    #[pyo3(signature = (
        per_quote,
        *,
        condition_number = None,
        singular_values = None,
        max_residual,
        rms_residual,
        r_squared = None
    ))]
    fn py_new(
        per_quote: Vec<PyQuoteQuality>,
        condition_number: Option<f64>,
        singular_values: Option<Vec<f64>>,
        max_residual: f64,
        rms_residual: f64,
        r_squared: Option<f64>,
    ) -> Self {
        Self::new(CalibrationDiagnostics {
            per_quote: per_quote.into_iter().map(|quote| quote.inner).collect(),
            condition_number,
            singular_values,
            max_residual,
            rms_residual,
            r_squared,
        })
    }

    #[getter]
    fn per_quote(&self) -> Vec<PyQuoteQuality> {
        self.inner
            .per_quote
            .iter()
            .cloned()
            .map(PyQuoteQuality::new)
            .collect()
    }

    #[getter]
    fn condition_number(&self) -> Option<f64> {
        self.inner.condition_number
    }

    #[getter]
    fn singular_values(&self) -> Option<Vec<f64>> {
        self.inner.singular_values.clone()
    }

    #[getter]
    fn max_residual(&self) -> f64 {
        self.inner.max_residual
    }

    #[getter]
    fn rms_residual(&self) -> f64 {
        self.inner.rms_residual
    }

    #[getter]
    fn r_squared(&self) -> Option<f64> {
        self.inner.r_squared
    }

    fn __repr__(&self) -> String {
        format!(
            "CalibrationDiagnostics(quotes={}, max_residual={})",
            self.inner.per_quote.len(),
            self.inner.max_residual
        )
    }

    fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        Ok(PyCalibrationReport::diagnostics_to_dict(py, &self.inner)?.into())
    }
}

#[pymethods]
impl PyCalibrationReport {
    #[getter]
    fn success(&self) -> bool {
        self.inner.success
    }

    #[getter]
    fn residuals<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        Self::map_to_dict(py, &self.inner.residuals)
    }

    #[getter]
    fn iterations(&self) -> usize {
        self.inner.iterations
    }

    #[getter]
    fn objective_value(&self) -> f64 {
        self.inner.objective_value
    }

    #[getter]
    fn max_residual(&self) -> f64 {
        self.inner.max_residual
    }

    #[getter]
    fn rmse(&self) -> f64 {
        self.inner.rmse
    }

    #[getter]
    fn convergence_reason(&self) -> &str {
        &self.inner.convergence_reason
    }

    #[getter]
    fn metadata<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        Self::string_map_to_dict(py, &self.inner.metadata)
    }

    #[getter]
    fn results_meta(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let bound = pythonize(py, &self.inner.results_meta)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        Ok(bound.unbind())
    }

    #[getter]
    fn explanation(&self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
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

    #[getter]
    fn validation_passed(&self) -> bool {
        self.inner.validation_passed
    }

    #[getter]
    fn validation_error(&self) -> Option<String> {
        self.inner.validation_error.clone()
    }

    #[getter]
    fn solver_config(&self) -> PySolverKind {
        PySolverKind::new(self.inner.solver_config.clone())
    }

    #[getter]
    fn model_version(&self) -> Option<String> {
        self.inner.model_version.clone()
    }

    #[getter]
    fn diagnostics(&self) -> Option<PyCalibrationDiagnostics> {
        self.inner
            .diagnostics
            .clone()
            .map(PyCalibrationDiagnostics::new)
    }

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

    fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let dict = PyDict::new(py);
        dict.set_item("success", self.inner.success)?;
        dict.set_item("iterations", self.inner.iterations)?;
        dict.set_item("objective_value", self.inner.objective_value)?;
        dict.set_item("max_residual", self.inner.max_residual)?;
        dict.set_item("rmse", self.inner.rmse)?;
        dict.set_item("convergence_reason", &self.inner.convergence_reason)?;
        dict.set_item("residuals", self.residuals(py)?)?;
        dict.set_item("metadata", self.metadata(py)?)?;
        dict.set_item("results_meta", self.results_meta(py)?)?;
        dict.set_item("validation_passed", self.inner.validation_passed)?;
        dict.set_item("validation_error", &self.inner.validation_error)?;
        let solver_name = match self.inner.solver_config {
            finstack_valuations::calibration::SolverConfig::Newton { .. } => "newton",
            finstack_valuations::calibration::SolverConfig::Brent { .. } => "brent",
        };
        dict.set_item("solver_config", solver_name)?;
        dict.set_item("model_version", &self.inner.model_version)?;
        if let Some(diagnostics) = &self.inner.diagnostics {
            dict.set_item("diagnostics", Self::diagnostics_to_dict(py, diagnostics)?)?;
        }
        if let Some(explanation) = self.explanation(py)? {
            dict.set_item("explanation", explanation)?;
        }
        Ok(dict.into())
    }

    fn __repr__(&self) -> String {
        format!(
            "CalibrationReport(success={}, iterations={}, max_residual={:.6})",
            self.inner.success, self.inner.iterations, self.inner.max_residual
        )
    }

    /// Extract Jacobian sensitivity matrix as a Pandas DataFrame.
    ///
    /// Returns a DataFrame with:
    /// - Rows: Instrument IDs
    /// - Columns: Curve point times
    /// - Values: Sensitivities (∂curve_point/∂instrument_quote)
    ///
    /// Returns None if explainability was not enabled during calibration.
    fn explain(&self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        // Check if explanation exists
        let trace = match &self.inner.explanation {
            Some(t) => t,
            None => return Ok(None),
        };

        // Find the Jacobian entry
        let jacobian = trace.entries.iter().find_map(|entry| match entry {
            finstack_core::explain::TraceEntry::Jacobian {
                row_labels,
                col_labels,
                sensitivity_matrix,
            } => Some((row_labels, col_labels, sensitivity_matrix)),
            _ => None,
        });

        let (row_labels, col_labels, matrix) = match jacobian {
            Some(j) => j,
            None => return Ok(None),
        };

        // Import pandas
        let pd = py.import("pandas")?;

        // Convert sensitivity_matrix to Python list of lists
        let py_matrix: Vec<Vec<f64>> = matrix.clone();

        // Create DataFrame: pd.DataFrame(data, index=row_labels, columns=col_labels)
        let df = pd.call_method(
            "DataFrame",
            (py_matrix,),
            Some(&[("index", row_labels), ("columns", col_labels)].into_py_dict(py)?),
        )?;

        Ok(Some(df.into()))
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyQuoteQuality>()?;
    module.add_class::<PyCalibrationDiagnostics>()?;
    module.add_class::<PyCalibrationReport>()?;
    Ok(vec![
        "QuoteQuality",
        "CalibrationDiagnostics",
        "CalibrationReport",
    ])
}
