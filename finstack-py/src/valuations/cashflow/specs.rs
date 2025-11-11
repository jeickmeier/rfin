//! Python bindings for cashflow builder specification types.

use finstack_valuations::cashflow::builder::AmortizationSpec;
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::PyType;

use crate::core::money::extract_money;
use crate::core::utils::py_to_date;

/// Amortization specification for principal over time.
#[pyclass(name = "AmortizationSpec", module = "finstack.valuations.cashflow.builder", frozen)]
#[derive(Clone, Debug)]
pub struct PyAmortizationSpec {
    pub(crate) inner: AmortizationSpec,
}

#[pymethods]
impl PyAmortizationSpec {
    #[classmethod]
    #[pyo3(text_signature = "(cls)")]
    /// No amortization: principal remains until redemption.
    fn none(_cls: &Bound<'_, PyType>) -> Self {
        Self {
            inner: AmortizationSpec::None,
        }
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, final_notional)")]
    /// Linear amortization towards a target final notional.
    fn linear_to(_cls: &Bound<'_, PyType>, final_notional: Bound<'_, PyAny>) -> PyResult<Self> {
        let m = extract_money(&final_notional)?;
        Ok(Self {
            inner: AmortizationSpec::LinearTo { final_notional: m },
        })
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, schedule)")]
    /// Step schedule of remaining principal after dates.
    /// ``schedule`` is a sequence of ``(date, Money)`` pairs ordered by date.
    fn step_remaining(
        _cls: &Bound<'_, PyType>,
        schedule: Vec<(Bound<'_, PyAny>, Bound<'_, PyAny>)>,
    ) -> PyResult<Self> {
        let mut items = Vec::with_capacity(schedule.len());
        for (d, m) in schedule {
            items.push((py_to_date(&d)?, extract_money(&m)?));
        }
        Ok(Self {
            inner: AmortizationSpec::StepRemaining { schedule: items },
        })
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, pct)")]
    /// Fixed percentage of original notional paid each period (e.g., 0.05 = 5%).
    fn percent_per_period(_cls: &Bound<'_, PyType>, pct: f64) -> Self {
        Self {
            inner: AmortizationSpec::PercentPerPeriod { pct },
        }
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, items)")]
    /// Custom principal exchanges as absolute cash amounts.
    /// ``items`` is a sequence of ``(date, Money)`` pairs.
    fn custom_principal(
        _cls: &Bound<'_, PyType>,
        items: Vec<(Bound<'_, PyAny>, Bound<'_, PyAny>)>,
    ) -> PyResult<Self> {
        let mut out = Vec::with_capacity(items.len());
        for (d, m) in items {
            out.push((py_to_date(&d)?, extract_money(&m)?));
        }
        Ok(Self {
            inner: AmortizationSpec::CustomPrincipal { items: out },
        })
    }

    fn __repr__(&self) -> String {
        match &self.inner {
            AmortizationSpec::None => "AmortizationSpec.none()".to_string(),
            AmortizationSpec::LinearTo { .. } => "AmortizationSpec.linear_to(...)".to_string(),
            AmortizationSpec::StepRemaining { .. } => {
                "AmortizationSpec.step_remaining(...)".to_string()
            }
            AmortizationSpec::PercentPerPeriod { pct } => {
                format!("AmortizationSpec.percent_per_period({pct})")
            }
            AmortizationSpec::CustomPrincipal { .. } => {
                "AmortizationSpec.custom_principal(...)".to_string()
            }
        }
    }
}

#[allow(dead_code)]
pub(crate) fn extract_amortization_spec(value: &Bound<'_, PyAny>) -> PyResult<AmortizationSpec> {
    if let Ok(spec) = value.extract::<PyRef<PyAmortizationSpec>>() {
        return Ok(spec.inner.clone());
    }
    Err(PyTypeError::new_err("Expected AmortizationSpec"))
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyAmortizationSpec>()?;
    Ok(vec!["AmortizationSpec"])
}

