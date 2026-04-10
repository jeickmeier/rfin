use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::core::money::PyMoney;
use finstack_margin::calculators::im::simm::SimmVersion;
use finstack_margin::{SimmCalculator, VmCalculator};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

use super::classification::PyMarginCall;
use super::csa::PyCsaSpec;
use super::helpers::parse_currency;
use super::results::{PySimmSensitivities, PyVmResult};

/// Variation margin calculator following ISDA CSA rules.
#[pyclass(
    name = "VmCalculator",
    module = "finstack.valuations.margin",
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyVmCalculator {
    pub(crate) inner: VmCalculator,
}

impl PyVmCalculator {
    pub(crate) fn new(inner: VmCalculator) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyVmCalculator {
    #[new]
    fn ctor(csa: PyCsaSpec) -> Self {
        Self::new(VmCalculator::new(csa.inner))
    }

    /// Calculate variation margin for a given exposure and posted collateral.
    fn calculate(
        &self,
        exposure: PyMoney,
        posted_collateral: PyMoney,
        as_of: &Bound<'_, PyAny>,
    ) -> PyResult<PyVmResult> {
        let date = py_to_date(as_of)?;
        let result = self
            .inner
            .calculate(exposure.inner, posted_collateral.inner, date)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        Ok(PyVmResult::new(result))
    }

    /// Generate a series of margin calls from an exposure time series.
    fn generate_margin_calls(
        &self,
        exposures: &Bound<'_, PyList>,
        initial_collateral: PyMoney,
    ) -> PyResult<Vec<PyMarginCall>> {
        let mut rust_exposures = Vec::with_capacity(exposures.len());
        for item in exposures.iter() {
            let tuple = item.extract::<(Bound<'_, PyAny>, PyMoney)>()?;
            let date = py_to_date(&tuple.0)?;
            rust_exposures.push((date, tuple.1.inner));
        }
        let calls = self
            .inner
            .generate_margin_calls(&rust_exposures, initial_collateral.inner)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        Ok(calls.into_iter().map(PyMarginCall::new).collect())
    }

    /// Generate margin call dates based on frequency.
    fn margin_call_dates(
        &self,
        py: Python<'_>,
        start: &Bound<'_, PyAny>,
        end: &Bound<'_, PyAny>,
    ) -> PyResult<Vec<Py<PyAny>>> {
        let start_date = py_to_date(start)?;
        let end_date = py_to_date(end)?;
        let dates = self.inner.margin_call_dates(start_date, end_date);
        dates.into_iter().map(|d| date_to_py(py, d)).collect()
    }

    fn __repr__(&self) -> String {
        "VmCalculator(...)".to_string()
    }
}

/// SIMM version identifier for calculator construction.
#[pyclass(
    name = "SimmVersion",
    module = "finstack.valuations.margin",
    frozen,
    from_py_object
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PySimmVersion {
    pub(crate) inner: SimmVersion,
}

impl PySimmVersion {
    pub(crate) const fn new(inner: SimmVersion) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PySimmVersion {
    #[classattr]
    const V2_5: Self = Self::new(SimmVersion::V2_5);
    #[classattr]
    const V2_6: Self = Self::new(SimmVersion::V2_6);

    #[getter]
    fn name(&self) -> String {
        self.inner.to_string()
    }

    fn __repr__(&self) -> String {
        format!("SimmVersion({})", self.inner)
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }
}

/// ISDA SIMM calculator for sensitivity-based IM.
#[pyclass(
    name = "SimmCalculator",
    module = "finstack.valuations.margin",
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PySimmCalculator {
    pub(crate) inner: SimmCalculator,
}

impl PySimmCalculator {
    pub(crate) fn new(inner: SimmCalculator) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PySimmCalculator {
    #[new]
    #[pyo3(signature = (version=None))]
    fn ctor(version: Option<PySimmVersion>) -> PyResult<Self> {
        let v = version.map_or(SimmVersion::default(), |pv| pv.inner);
        let calc = SimmCalculator::new(v)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        Ok(Self::new(calc))
    }

    /// Calculate IM from a complete set of SIMM sensitivities.
    ///
    /// Returns a tuple of (total_im_amount, breakdown_dict) where the
    /// breakdown maps risk class names to Money amounts.
    fn calculate_from_sensitivities<'py>(
        &self,
        py: Python<'py>,
        sensitivities: &PySimmSensitivities,
        currency: &Bound<'py, PyAny>,
    ) -> PyResult<(f64, Py<PyAny>)> {
        let ccy = parse_currency(currency)?;
        let (total, breakdown) = self
            .inner
            .calculate_from_sensitivities(&sensitivities.inner, ccy);
        let d = PyDict::new(py);
        for (key, value) in &breakdown {
            d.set_item(key, PyMoney::new(*value).into_pyobject(py)?)?;
        }
        Ok((total, d.into()))
    }

    fn __repr__(&self) -> String {
        "SimmCalculator(...)".to_string()
    }
}
