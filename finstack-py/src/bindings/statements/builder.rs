//! Python wrapper for the type-state ModelBuilder.
//!
//! Since Python cannot model Rust type-state at the type level, we collapse
//! the two states into a single class and track readiness at runtime.

use super::types::PyFinancialModelSpec;
use crate::errors::display_to_py;
use finstack_core::dates::PeriodId;
use finstack_statements::builder::ModelBuilder;
use finstack_statements::types::AmountOrScalar;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

/// Builder for financial models (type-state collapsed for Python).
///
/// Usage::
///
///     builder = ModelBuilder("Acme Corp")
///     builder.periods("2025Q1..Q4", "2025Q2")
///     builder.value("revenue", [("2025Q1", 10_000_000.0), ("2025Q2", 11_000_000.0)])
///     builder.compute("cogs", "revenue * 0.6")
///     model = builder.build()
#[pyclass(name = "ModelBuilder", module = "finstack.statements")]
pub struct PyModelBuilder {
    inner: Option<BuilderState>,
}

enum BuilderState {
    NeedPeriods(ModelBuilder<finstack_statements::builder::NeedPeriods>),
    Ready(ModelBuilder<finstack_statements::builder::Ready>),
}

#[pymethods]
impl PyModelBuilder {
    /// Create a new model builder.
    #[new]
    fn new(id: &str) -> Self {
        Self {
            inner: Some(BuilderState::NeedPeriods(ModelBuilder::new(id))),
        }
    }

    /// Define periods using a range expression (e.g. ``"2025Q1..Q4"``).
    ///
    /// Parameters
    /// ----------
    /// range : str
    ///     Period range expression.
    /// actuals_until : str | None
    ///     Optional cutoff for actual values.
    #[pyo3(signature = (range, actuals_until=None))]
    fn periods(&mut self, range: &str, actuals_until: Option<&str>) -> PyResult<()> {
        let state = self
            .inner
            .take()
            .ok_or_else(|| PyValueError::new_err("Builder has already been consumed by build()"))?;
        match state {
            BuilderState::NeedPeriods(b) => {
                let ready = b.periods(range, actuals_until).map_err(display_to_py)?;
                self.inner = Some(BuilderState::Ready(ready));
                Ok(())
            }
            BuilderState::Ready(_) => Err(PyValueError::new_err("Periods already set")),
        }
    }

    /// Add a value node with explicit period values.
    ///
    /// Parameters
    /// ----------
    /// node_id : str
    ///     Node identifier.
    /// values : list[tuple[str, float]]
    ///     List of (period_string, value) tuples.
    fn value(&mut self, node_id: &str, values: Vec<(String, f64)>) -> PyResult<()> {
        let state = self.take_ready()?;
        let parsed: Vec<(PeriodId, AmountOrScalar)> = values
            .into_iter()
            .map(|(p, v)| {
                let pid: PeriodId = p
                    .parse()
                    .map_err(|e: finstack_core::Error| PyValueError::new_err(e.to_string()))?;
                Ok((pid, AmountOrScalar::scalar(v)))
            })
            .collect::<PyResult<Vec<_>>>()?;

        let ready = state.value(node_id, &parsed);
        self.inner = Some(BuilderState::Ready(ready));
        Ok(())
    }

    /// Add a computed node with a formula.
    ///
    /// Parameters
    /// ----------
    /// node_id : str
    ///     Node identifier.
    /// formula : str
    ///     DSL formula expression (e.g. ``"revenue - cogs"``).
    fn compute(&mut self, node_id: &str, formula: &str) -> PyResult<()> {
        let state = self.take_ready()?;
        let ready = state.compute(node_id, formula).map_err(display_to_py)?;
        self.inner = Some(BuilderState::Ready(ready));
        Ok(())
    }

    /// Build the model specification.
    ///
    /// Returns
    /// -------
    /// FinancialModelSpec
    ///     The completed model specification.
    fn build(&mut self) -> PyResult<PyFinancialModelSpec> {
        let state = self.take_ready()?;
        let spec = state.build().map_err(display_to_py)?;
        Ok(PyFinancialModelSpec { inner: spec })
    }
}

impl PyModelBuilder {
    fn take_ready(&mut self) -> PyResult<ModelBuilder<finstack_statements::builder::Ready>> {
        let state = self
            .inner
            .take()
            .ok_or_else(|| PyValueError::new_err("Builder has already been consumed by build()"))?;
        match state {
            BuilderState::Ready(b) => Ok(b),
            BuilderState::NeedPeriods(b) => {
                self.inner = Some(BuilderState::NeedPeriods(b));
                Err(PyValueError::new_err(
                    "Must call periods() before adding nodes",
                ))
            }
        }
    }
}

/// Register builder classes.
pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyModelBuilder>()?;
    Ok(())
}
