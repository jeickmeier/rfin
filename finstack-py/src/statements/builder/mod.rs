//! Builder API for financial models.

use crate::core::dates::periods::PyPeriod;
use crate::statements::error::stmt_to_py;
use crate::statements::types::forecast::PyForecastSpec;
use crate::statements::types::model::PyFinancialModelSpec;
use crate::statements::types::value::PyAmountOrScalar;
use finstack_core::dates::PeriodId;
use finstack_statements::builder::{ModelBuilder, NeedPeriods, Ready};
use finstack_statements::types::AmountOrScalar;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAnyMethods, PyDict, PyList, PyModule, PyType};
use pyo3::Bound;

/// Builder for financial models.
///
/// Provides a fluent API for building financial statement models with
/// type-safe construction.
///
/// Examples
/// --------
/// >>> builder = ModelBuilder.new("Acme Corp")
/// >>> builder = builder.periods("2025Q1..Q4", "2025Q2")
/// >>> builder = builder.value("revenue", [(PeriodId.quarter(2025, 1), AmountOrScalar.scalar(100))])
/// >>> builder = builder.compute("gross_profit", "revenue * 0.4")
/// >>> model = builder.build()
#[pyclass(module = "finstack.statements.builder", name = "ModelBuilder", unsendable)]
pub struct PyModelBuilder {
    state: BuilderState,
}

enum BuilderState {
    NeedPeriods(ModelBuilder<NeedPeriods>),
    Ready(ModelBuilder<Ready>),
}

#[pymethods]
impl PyModelBuilder {
    #[classmethod]
    #[pyo3(text_signature = "(cls, id)")]
    /// Create a new model builder.
    ///
    /// You must call `periods()` before adding nodes.
    ///
    /// Parameters
    /// ----------
    /// id : str
    ///     Unique model identifier
    ///
    /// Returns
    /// -------
    /// ModelBuilder
    ///     Model builder instance
    fn new(_cls: &Bound<'_, PyType>, id: String) -> Self {
        Self {
            state: BuilderState::NeedPeriods(ModelBuilder::new(id)),
        }
    }

    #[pyo3(text_signature = "(self, range, actuals_until=None)")]
    /// Define periods using a range expression.
    ///
    /// Parameters
    /// ----------
    /// range : str
    ///     Period range (e.g., "2025Q1..Q4", "2025Q1..2026Q2")
    /// actuals_until : str, optional
    ///     Optional cutoff for actuals (e.g., "2025Q2")
    ///
    /// Returns
    /// -------
    /// ModelBuilder
    ///     Builder instance ready for node definitions
    fn periods(
        &mut self,
        range: &str,
        actuals_until: Option<&str>,
    ) -> PyResult<()> {
        match std::mem::replace(&mut self.state, BuilderState::Ready(ModelBuilder::new("dummy").periods("2025Q1..Q2", None).unwrap())) {
            BuilderState::NeedPeriods(builder) => {
                let ready = builder
                    .periods(range, actuals_until)
                    .map_err(stmt_to_py)?;
                self.state = BuilderState::Ready(ready);
                Ok(())
            }
            BuilderState::Ready(_) => {
                Err(PyValueError::new_err("periods() already called"))
            }
        }
    }

    #[pyo3(text_signature = "(self, periods)")]
    /// Define periods explicitly.
    ///
    /// Parameters
    /// ----------
    /// periods : list[Period]
    ///     Explicit list of periods
    ///
    /// Returns
    /// -------
    /// ModelBuilder
    ///     Builder instance ready for node definitions
    fn periods_explicit(
        &mut self,
        periods: Vec<PyPeriod>,
    ) -> PyResult<()> {
        match std::mem::replace(&mut self.state, BuilderState::Ready(ModelBuilder::new("dummy").periods("2025Q1..Q2", None).unwrap())) {
            BuilderState::NeedPeriods(builder) => {
                let periods = periods.into_iter().map(|p| p.inner).collect();
                let ready = builder
                    .periods_explicit(periods)
                    .map_err(stmt_to_py)?;
                self.state = BuilderState::Ready(ready);
                Ok(())
            }
            BuilderState::Ready(_) => {
                Err(PyValueError::new_err("periods() already called"))
            }
        }
    }

    #[pyo3(text_signature = "(self, node_id, values)")]
    /// Add a value node with explicit period values.
    ///
    /// Value nodes contain only explicit data (actuals or assumptions).
    ///
    /// Parameters
    /// ----------
    /// node_id : str
    ///     Node identifier
    /// values : list[tuple[PeriodId, AmountOrScalar]] or dict[PeriodId, AmountOrScalar]
    ///     Period values
    ///
    /// Returns
    /// -------
    /// ModelBuilder
    ///     Builder instance for chaining
    fn value(
        &mut self,
        node_id: String,
        values: &Bound<'_, PyAny>,
    ) -> PyResult<()> {
        let values_vec = parse_period_values(values)?;
        
        if let BuilderState::Ready(builder) = &mut self.state {
            let new_builder = std::mem::replace(builder, ModelBuilder::new("dummy").periods("2025Q1..Q2", None).unwrap());
            *builder = new_builder.value(node_id, &values_vec);
            Ok(())
        } else {
            Err(PyValueError::new_err("Must call periods() first"))
        }
    }

    #[pyo3(text_signature = "(self, node_id, formula)")]
    /// Add a calculated node with a formula.
    ///
    /// Calculated nodes derive their values from formulas only.
    ///
    /// Parameters
    /// ----------
    /// node_id : str
    ///     Node identifier
    /// formula : str
    ///     Formula text in statement DSL
    ///
    /// Returns
    /// -------
    /// ModelBuilder
    ///     Builder instance for chaining
    fn compute(
        &mut self,
        node_id: String,
        formula: String,
    ) -> PyResult<()> {
        if let BuilderState::Ready(builder) = &mut self.state {
            let new_builder = std::mem::replace(builder, ModelBuilder::new("dummy").periods("2025Q1..Q2", None).unwrap());
            *builder = new_builder.compute(node_id, formula).map_err(stmt_to_py)?;
            Ok(())
        } else {
            Err(PyValueError::new_err("Must call periods() first"))
        }
    }

    #[pyo3(text_signature = "(self, node_id, forecast_spec)")]
    /// Add a forecast specification to an existing node.
    ///
    /// This allows forecasting values into future periods using various methods.
    ///
    /// Parameters
    /// ----------
    /// node_id : str
    ///     Node identifier
    /// forecast_spec : ForecastSpec
    ///     Forecast specification
    ///
    /// Returns
    /// -------
    /// ModelBuilder
    ///     Builder instance for chaining
    fn forecast(
        &mut self,
        node_id: String,
        forecast_spec: &PyForecastSpec,
    ) -> PyResult<()> {
        if let BuilderState::Ready(builder) = &mut self.state {
            let new_builder = std::mem::replace(builder, ModelBuilder::new("dummy").periods("2025Q1..Q2", None).unwrap());
            *builder = new_builder.forecast(node_id, forecast_spec.inner.clone());
            Ok(())
        } else {
            Err(PyValueError::new_err("Must call periods() first"))
        }
    }

    // Note: mixed() and with_where() methods are not exposed for now
    // They can be added later if needed when the Rust API stabilizes

    #[pyo3(text_signature = "(self, key, value)")]
    /// Add metadata to the model.
    ///
    /// Parameters
    /// ----------
    /// key : str
    ///     Metadata key
    /// value : Any
    ///     Metadata value (must be JSON-serializable)
    ///
    /// Returns
    /// -------
    /// ModelBuilder
    ///     Builder instance for chaining
    fn with_meta(
        &mut self,
        key: String,
        value: &Bound<'_, PyAny>,
    ) -> PyResult<()> {
        let json_value = crate::statements::utils::py_to_json(value)?;
        
        if let BuilderState::Ready(builder) = &mut self.state {
            let new_builder = std::mem::replace(builder, ModelBuilder::new("dummy").periods("2025Q1..Q2", None).unwrap());
            *builder = new_builder.with_meta(key, json_value);
            Ok(())
        } else {
            Err(PyValueError::new_err("Must call periods() first"))
        }
    }

    // Note: capital_structure() method not yet exposed
    // Will be added when the Rust API method is available

    #[pyo3(text_signature = "(self)")]
    /// Build the final model specification.
    ///
    /// Returns
    /// -------
    /// FinancialModelSpec
    ///     Complete model specification
    fn build(&mut self) -> PyResult<PyFinancialModelSpec> {
        if let BuilderState::Ready(builder) = &mut self.state {
            let new_builder = std::mem::replace(builder, ModelBuilder::new("dummy").periods("2025Q1..Q2", None).unwrap());
            let spec = new_builder.build().map_err(stmt_to_py)?;
            Ok(PyFinancialModelSpec::new(spec))
        } else {
            Err(PyValueError::new_err("Must call periods() first"))
        }
    }
}

/// Helper to parse period values from dict or list of tuples.
fn parse_period_values(
    values: &Bound<'_, PyAny>,
) -> PyResult<Vec<(PeriodId, AmountOrScalar)>> {
    let mut vec = Vec::new();

    if let Ok(dict) = values.downcast::<PyDict>() {
        // Dict format
        for (key, value) in dict.iter() {
            let period_id: crate::core::dates::periods::PyPeriodId = key.extract()?;
            let amount: PyAmountOrScalar = value.extract()?;
            vec.push((period_id.inner, amount.inner.clone()));
        }
    } else if let Ok(list) = values.downcast::<PyList>() {
        // List of tuples format
        for item in list.iter() {
            if let Ok((period, amount)) = item.extract::<(crate::core::dates::periods::PyPeriodId, PyAmountOrScalar)>() {
                vec.push((period.inner, amount.inner.clone()));
            }
        }
    } else {
        return Err(PyValueError::new_err(
            "values must be a dict or list of tuples",
        ));
    }

    Ok(vec)
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(_py, "builder")?;
    module.setattr("__doc__", "Builder API for financial models.")?;

    module.add_class::<PyModelBuilder>()?;

    parent.add_submodule(&module)?;
    parent.setattr("builder", &module)?;

    Ok(vec!["ModelBuilder"])
}

