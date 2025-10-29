use finstack_valuations::calibration::CalibrationReport;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyModule};
use pyo3::Bound;
use pythonize::pythonize;

#[pyclass(
    module = "finstack.valuations.calibration",
    name = "CalibrationReport",
    frozen
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
    fn results_meta(&self, py: Python<'_>) -> PyResult<PyObject> {
        let bound = pythonize(py, &self.inner.results_meta)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        Ok(bound.unbind())
    }

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

    fn to_dict(&self, py: Python<'_>) -> PyResult<PyObject> {
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
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyCalibrationReport>()?;
    Ok(vec!["CalibrationReport"])
}
