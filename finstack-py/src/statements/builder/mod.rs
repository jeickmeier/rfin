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
#[pyclass(
    module = "finstack.statements.builder",
    name = "ModelBuilder",
    unsendable
)]
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
    fn periods(&mut self, range: &str, actuals_until: Option<&str>) -> PyResult<()> {
        match std::mem::replace(
            &mut self.state,
            BuilderState::Ready(
                ModelBuilder::new("dummy")
                    .periods("2025Q1..Q2", None)
                    .unwrap(),
            ),
        ) {
            BuilderState::NeedPeriods(builder) => {
                let ready = builder.periods(range, actuals_until).map_err(stmt_to_py)?;
                self.state = BuilderState::Ready(ready);
                Ok(())
            }
            BuilderState::Ready(_) => Err(PyValueError::new_err("periods() already called")),
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
    fn periods_explicit(&mut self, periods: Vec<PyPeriod>) -> PyResult<()> {
        match std::mem::replace(
            &mut self.state,
            BuilderState::Ready(
                ModelBuilder::new("dummy")
                    .periods("2025Q1..Q2", None)
                    .unwrap(),
            ),
        ) {
            BuilderState::NeedPeriods(builder) => {
                let periods = periods.into_iter().map(|p| p.inner).collect();
                let ready = builder.periods_explicit(periods).map_err(stmt_to_py)?;
                self.state = BuilderState::Ready(ready);
                Ok(())
            }
            BuilderState::Ready(_) => Err(PyValueError::new_err("periods() already called")),
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
    fn value(&mut self, node_id: String, values: &Bound<'_, PyAny>) -> PyResult<()> {
        let values_vec = parse_period_values(values)?;

        if let BuilderState::Ready(builder) = &mut self.state {
            let new_builder = std::mem::replace(
                builder,
                ModelBuilder::new("dummy")
                    .periods("2025Q1..Q2", None)
                    .unwrap(),
            );
            *builder = new_builder.value(node_id, &values_vec);
            Ok(())
        } else {
            Err(PyValueError::new_err("Must call periods() first"))
        }
    }

    #[pyo3(text_signature = "(self, node_id, values)")]
    /// Add a monetary value node.
    ///
    /// This is a convenience method for creating value nodes that represent
    /// monetary amounts (Money type).
    ///
    /// Parameters
    /// ----------
    /// node_id : str
    ///     Node identifier
    /// values : list[tuple[PeriodId, Money]] or dict[PeriodId, Money]
    ///     Period values as Money objects
    ///
    /// Returns
    /// -------
    /// None
    fn value_money(&mut self, node_id: String, values: &Bound<'_, PyAny>) -> PyResult<()> {
        use crate::core::dates::periods::PyPeriodId;
        use crate::core::money::PyMoney;

        let mut values_vec = Vec::new();

        if let Ok(dict) = values.downcast::<PyDict>() {
            // Dict format
            for (key, value) in dict.iter() {
                let period_id: PyPeriodId = key.extract()?;
                let money: PyMoney = value.extract()?;
                values_vec.push((period_id.inner, money.inner));
            }
        } else if let Ok(list) = values.downcast::<PyList>() {
            // List of tuples format
            for item in list.iter() {
                if let Ok((period, money)) = item.extract::<(PyPeriodId, PyMoney)>() {
                    values_vec.push((period.inner, money.inner));
                }
            }
        } else {
            return Err(PyValueError::new_err(
                "values must be a dict or list of tuples",
            ));
        }

        if let BuilderState::Ready(builder) = &mut self.state {
            let new_builder = std::mem::replace(
                builder,
                ModelBuilder::new("dummy")
                    .periods("2025Q1..Q2", None)
                    .unwrap(),
            );
            *builder = new_builder.value_money(node_id, &values_vec);
            Ok(())
        } else {
            Err(PyValueError::new_err("Must call periods() first"))
        }
    }

    #[pyo3(text_signature = "(self, node_id, values)")]
    /// Add a scalar value node.
    ///
    /// This is a convenience method for creating value nodes that represent
    /// non-monetary scalars (ratios, percentages, counts, etc.).
    ///
    /// Parameters
    /// ----------
    /// node_id : str
    ///     Node identifier
    /// values : list[tuple[PeriodId, float]] or dict[PeriodId, float]
    ///     Period values as floats
    ///
    /// Returns
    /// -------
    /// None
    fn value_scalar(&mut self, node_id: String, values: &Bound<'_, PyAny>) -> PyResult<()> {
        use crate::core::dates::periods::PyPeriodId;

        let mut values_vec = Vec::new();

        if let Ok(dict) = values.downcast::<PyDict>() {
            // Dict format
            for (key, value) in dict.iter() {
                let period_id: PyPeriodId = key.extract()?;
                let scalar: f64 = value.extract()?;
                values_vec.push((period_id.inner, scalar));
            }
        } else if let Ok(list) = values.downcast::<PyList>() {
            // List of tuples format
            for item in list.iter() {
                if let Ok((period, scalar)) = item.extract::<(PyPeriodId, f64)>() {
                    values_vec.push((period.inner, scalar));
                }
            }
        } else {
            return Err(PyValueError::new_err(
                "values must be a dict or list of tuples",
            ));
        }

        if let BuilderState::Ready(builder) = &mut self.state {
            let new_builder = std::mem::replace(
                builder,
                ModelBuilder::new("dummy")
                    .periods("2025Q1..Q2", None)
                    .unwrap(),
            );
            *builder = new_builder.value_scalar(node_id, &values_vec);
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
    fn compute(&mut self, node_id: String, formula: String) -> PyResult<()> {
        if let BuilderState::Ready(builder) = &mut self.state {
            let new_builder = std::mem::replace(
                builder,
                ModelBuilder::new("dummy")
                    .periods("2025Q1..Q2", None)
                    .unwrap(),
            );
            *builder = new_builder.compute(node_id, formula).map_err(stmt_to_py)?;
            Ok(())
        } else {
            Err(PyValueError::new_err("Must call periods() first"))
        }
    }

    #[pyo3(text_signature = "(self, node_id)")]
    /// Create a mixed node with values, forecasts, and formulas.
    ///
    /// Returns a MixedNodeBuilder for chaining method calls.
    ///
    /// Parameters
    /// ----------
    /// node_id : str
    ///     Node identifier
    ///
    /// Returns
    /// -------
    /// MixedNodeBuilder
    ///     Mixed node builder instance
    fn mixed(&mut self, node_id: String) -> PyResult<PyMixedNodeBuilder> {
        if let BuilderState::Ready(builder) = &mut self.state {
            let parent = std::mem::replace(
                builder,
                ModelBuilder::new("dummy")
                    .periods("2025Q1..Q2", None)
                    .unwrap(),
            );
            
            Ok(PyMixedNodeBuilder {
                parent_builder: Some(parent),
                node_id,
                values: None,
                forecast: None,
                formula: None,
                name: None,
            })
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
    fn forecast(&mut self, node_id: String, forecast_spec: &PyForecastSpec) -> PyResult<()> {
        if let BuilderState::Ready(builder) = &mut self.state {
            let new_builder = std::mem::replace(
                builder,
                ModelBuilder::new("dummy")
                    .periods("2025Q1..Q2", None)
                    .unwrap(),
            );
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
    fn with_meta(&mut self, key: String, value: &Bound<'_, PyAny>) -> PyResult<()> {
        let json_value = crate::statements::utils::py_to_json(value)?;

        if let BuilderState::Ready(builder) = &mut self.state {
            let new_builder = std::mem::replace(
                builder,
                ModelBuilder::new("dummy")
                    .periods("2025Q1..Q2", None)
                    .unwrap(),
            );
            *builder = new_builder.with_meta(key, json_value);
            Ok(())
        } else {
            Err(PyValueError::new_err("Must call periods() first"))
        }
    }

    #[pyo3(text_signature = "(self, id, notional, coupon_rate, issue_date, maturity_date, discount_curve_id)")]
    /// Add a bond instrument to the capital structure.
    ///
    /// Parameters
    /// ----------
    /// id : str
    ///     Unique instrument identifier
    /// notional : Money
    ///     Principal amount
    /// coupon_rate : float
    ///     Annual coupon rate (e.g., 0.05 for 5%)
    /// issue_date : date
    ///     Bond issue date
    /// maturity_date : date
    ///     Bond maturity date
    /// discount_curve_id : str
    ///     Discount curve ID for pricing
    ///
    /// Returns
    /// -------
    /// None
    fn add_bond(
        &mut self,
        id: String,
        notional: &crate::core::money::PyMoney,
        coupon_rate: f64,
        issue_date: &Bound<'_, PyAny>,
        maturity_date: &Bound<'_, PyAny>,
        discount_curve_id: String,
    ) -> PyResult<()> {
        use crate::core::utils::py_to_date;

        let issue = py_to_date(issue_date)?;
        let maturity = py_to_date(maturity_date)?;

        if let BuilderState::Ready(builder) = &mut self.state {
            let new_builder = std::mem::replace(
                builder,
                ModelBuilder::new("dummy")
                    .periods("2025Q1..Q2", None)
                    .unwrap(),
            );
            *builder = new_builder
                .add_bond(id, notional.inner, coupon_rate, issue, maturity, discount_curve_id)
                .map_err(stmt_to_py)?;
            Ok(())
        } else {
            Err(PyValueError::new_err("Must call periods() first"))
        }
    }

    #[pyo3(text_signature = "(self, id, notional, fixed_rate, start_date, maturity_date, discount_curve_id, forward_curve_id)")]
    /// Add an interest rate swap to the capital structure.
    ///
    /// Parameters
    /// ----------
    /// id : str
    ///     Unique instrument identifier
    /// notional : Money
    ///     Notional amount
    /// fixed_rate : float
    ///     Fixed rate (e.g., 0.04 for 4%)
    /// start_date : date
    ///     Swap start date
    /// maturity_date : date
    ///     Swap maturity date
    /// discount_curve_id : str
    ///     Discount curve ID
    /// forward_curve_id : str
    ///     Forward curve ID for floating leg
    ///
    /// Returns
    /// -------
    /// None
    fn add_swap(
        &mut self,
        id: String,
        notional: &crate::core::money::PyMoney,
        fixed_rate: f64,
        start_date: &Bound<'_, PyAny>,
        maturity_date: &Bound<'_, PyAny>,
        discount_curve_id: String,
        forward_curve_id: String,
    ) -> PyResult<()> {
        use crate::core::utils::py_to_date;

        let start = py_to_date(start_date)?;
        let maturity = py_to_date(maturity_date)?;

        if let BuilderState::Ready(builder) = &mut self.state {
            let new_builder = std::mem::replace(
                builder,
                ModelBuilder::new("dummy")
                    .periods("2025Q1..Q2", None)
                    .unwrap(),
            );
            *builder = new_builder
                .add_swap(
                    id,
                    notional.inner,
                    fixed_rate,
                    start,
                    maturity,
                    discount_curve_id,
                    forward_curve_id,
                )
                .map_err(stmt_to_py)?;
            Ok(())
        } else {
            Err(PyValueError::new_err("Must call periods() first"))
        }
    }

    #[pyo3(text_signature = "(self, id, spec)")]
    /// Add a generic debt instrument via JSON specification.
    ///
    /// This allows adding custom debt instruments not covered by the convenience
    /// methods (bonds, swaps).
    ///
    /// Parameters
    /// ----------
    /// id : str
    ///     Unique instrument identifier
    /// spec : dict
    ///     JSON specification for the debt instrument
    ///
    /// Returns
    /// -------
    /// None
    ///
    /// Examples
    /// --------
    /// >>> builder.add_custom_debt("TL-A", {
    /// ...     "type": "term_loan",
    /// ...     "notional": 10_000_000.0,
    /// ...     "currency": "USD",
    /// ... })
    fn add_custom_debt(&mut self, id: String, spec: &Bound<'_, PyAny>) -> PyResult<()> {
        let json_value = crate::statements::utils::py_to_json(spec)?;

        if let BuilderState::Ready(builder) = &mut self.state {
            let new_builder = std::mem::replace(
                builder,
                ModelBuilder::new("dummy")
                    .periods("2025Q1..Q2", None)
                    .unwrap(),
            );
            *builder = new_builder.add_custom_debt(id, json_value);
            Ok(())
        } else {
            Err(PyValueError::new_err("Must call periods() first"))
        }
    }

    #[pyo3(text_signature = "(self)")]
    /// Load built-in metrics (fin.* namespace) and add them to the model.
    ///
    /// This adds all standard financial metrics from the built-in registry.
    ///
    /// Returns
    /// -------
    /// None
    fn with_builtin_metrics(&mut self) -> PyResult<()> {
        if let BuilderState::Ready(builder) = &mut self.state {
            let new_builder = std::mem::replace(
                builder,
                ModelBuilder::new("dummy")
                    .periods("2025Q1..Q2", None)
                    .unwrap(),
            );
            *builder = new_builder.with_builtin_metrics().map_err(stmt_to_py)?;
            Ok(())
        } else {
            Err(PyValueError::new_err("Must call periods() first"))
        }
    }

    #[pyo3(text_signature = "(self, path)")]
    /// Load metrics from a JSON file and add them to the model.
    ///
    /// Parameters
    /// ----------
    /// path : str
    ///     Path to a metrics JSON definition file
    ///
    /// Returns
    /// -------
    /// None
    fn with_metrics(&mut self, path: &str) -> PyResult<()> {
        if let BuilderState::Ready(builder) = &mut self.state {
            let new_builder = std::mem::replace(
                builder,
                ModelBuilder::new("dummy")
                    .periods("2025Q1..Q2", None)
                    .unwrap(),
            );
            *builder = new_builder.with_metrics(path).map_err(stmt_to_py)?;
            Ok(())
        } else {
            Err(PyValueError::new_err("Must call periods() first"))
        }
    }

    #[pyo3(text_signature = "(self, qualified_id)")]
    /// Add a specific metric from the built-in registry.
    ///
    /// This is a convenience method that loads the built-in metrics registry
    /// and adds a specific metric to the model.
    ///
    /// Parameters
    /// ----------
    /// qualified_id : str
    ///     Fully qualified metric identifier (e.g., "fin.gross_margin")
    ///
    /// Returns
    /// -------
    /// None
    ///
    /// Examples
    /// --------
    /// >>> builder.add_metric("fin.gross_margin")
    fn add_metric(&mut self, qualified_id: &str) -> PyResult<()> {
        if let BuilderState::Ready(builder) = &mut self.state {
            let new_builder = std::mem::replace(
                builder,
                ModelBuilder::new("dummy")
                    .periods("2025Q1..Q2", None)
                    .unwrap(),
            );
            *builder = new_builder.add_metric(qualified_id).map_err(stmt_to_py)?;
            Ok(())
        } else {
            Err(PyValueError::new_err("Must call periods() first"))
        }
    }

    #[pyo3(text_signature = "(self, qualified_id, registry)")]
    /// Add a specific metric from a registry.
    ///
    /// This allows selectively adding metrics from a registry instead of
    /// adding all of them.
    ///
    /// Parameters
    /// ----------
    /// qualified_id : str
    ///     Fully qualified metric identifier to add
    /// registry : Registry
    ///     Registry loaded by the caller (allows reuse across builders)
    ///
    /// Returns
    /// -------
    /// None
    #[pyo3(signature = (qualified_id, registry))]
    fn add_metric_from_registry(
        &mut self,
        qualified_id: &str,
        registry: Bound<'_, PyAny>,
    ) -> PyResult<()> {
        // Extract the PyRegistry directly
        let registry_ref: PyRef<'_, crate::statements::registry::PyRegistry> = registry.extract()?;

        if let BuilderState::Ready(builder) = &mut self.state {
            let new_builder = std::mem::replace(
                builder,
                ModelBuilder::new("dummy")
                    .periods("2025Q1..Q2", None)
                    .unwrap(),
            );
            *builder = new_builder
                .add_metric_from_registry(qualified_id, registry_ref.inner())
                .map_err(stmt_to_py)?;
            Ok(())
        } else {
            Err(PyValueError::new_err("Must call periods() first"))
        }
    }

    #[pyo3(text_signature = "(self)")]
    /// Build the final model specification.
    ///
    /// Returns
    /// -------
    /// FinancialModelSpec
    ///     Complete model specification
    fn build(&mut self) -> PyResult<PyFinancialModelSpec> {
        if let BuilderState::Ready(builder) = &mut self.state {
            let new_builder = std::mem::replace(
                builder,
                ModelBuilder::new("dummy")
                    .periods("2025Q1..Q2", None)
                    .unwrap(),
            );
            let spec = new_builder.build().map_err(stmt_to_py)?;
            Ok(PyFinancialModelSpec::new(spec))
        } else {
            Err(PyValueError::new_err("Must call periods() first"))
        }
    }
}

/// Helper to parse period values from dict or list of tuples.
fn parse_period_values(values: &Bound<'_, PyAny>) -> PyResult<Vec<(PeriodId, AmountOrScalar)>> {
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
            if let Ok((period, amount)) =
                item.extract::<(crate::core::dates::periods::PyPeriodId, PyAmountOrScalar)>()
            {
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

/// Mixed node builder for creating nodes with values, forecasts, and formulas.
#[pyclass(
    module = "finstack.statements.builder",
    name = "MixedNodeBuilder",
    unsendable
)]
pub struct PyMixedNodeBuilder {
    parent_builder: Option<ModelBuilder<Ready>>,
    node_id: String,
    values: Option<Vec<(PeriodId, AmountOrScalar)>>,
    forecast: Option<finstack_statements::types::ForecastSpec>,
    formula: Option<String>,
    name: Option<String>,
}

impl Default for PyMixedNodeBuilder {
    fn default() -> Self {
        Self {
            parent_builder: None,
            node_id: String::new(),
            values: None,
            forecast: None,
            formula: None,
            name: None,
        }
    }
}

#[pymethods]
impl PyMixedNodeBuilder {
    #[pyo3(text_signature = "(self, values)")]
    /// Set explicit values for the mixed node.
    ///
    /// Parameters
    /// ----------
    /// values : list[tuple[PeriodId, AmountOrScalar]] or dict[PeriodId, AmountOrScalar]
    ///     Period values to seed actual periods
    ///
    /// Returns
    /// -------
    /// MixedNodeBuilder
    ///     Builder instance for chaining
    fn values(&mut self, values: &Bound<'_, PyAny>) -> PyResult<()> {
        let values_vec = parse_period_values(values)?;
        self.values = Some(values_vec);
        Ok(())
    }

    #[pyo3(text_signature = "(self, forecast_spec)")]
    /// Set the forecast specification.
    ///
    /// Parameters
    /// ----------
    /// forecast_spec : ForecastSpec
    ///     Forecast configuration
    ///
    /// Returns
    /// -------
    /// MixedNodeBuilder
    ///     Builder instance for chaining
    fn forecast(&mut self, forecast_spec: &PyForecastSpec) -> PyResult<()> {
        self.forecast = Some(forecast_spec.inner.clone());
        Ok(())
    }

    #[pyo3(text_signature = "(self, formula)")]
    /// Set the fallback formula.
    ///
    /// Parameters
    /// ----------
    /// formula : str
    ///     DSL expression evaluated when explicit values or forecasts are absent
    ///
    /// Returns
    /// -------
    /// MixedNodeBuilder
    ///     Builder instance for chaining
    fn formula(&mut self, formula: String) -> PyResult<()> {
        // Validate formula by calling Rust validation
        if formula.trim().is_empty() {
            return Err(PyValueError::new_err("Formula cannot be empty"));
        }
        finstack_statements::dsl::parse_and_compile(&formula).map_err(stmt_to_py)?;
        
        self.formula = Some(formula);
        Ok(())
    }

    #[pyo3(text_signature = "(self, name)")]
    /// Set the human-readable name.
    ///
    /// Parameters
    /// ----------
    /// name : str
    ///     Display label used in reports or exports
    ///
    /// Returns
    /// -------
    /// MixedNodeBuilder
    ///     Builder instance for chaining
    fn name(&mut self, name: String) -> PyResult<()> {
        self.name = Some(name);
        Ok(())
    }

    #[pyo3(text_signature = "(self)")]
    /// Finish building the mixed node and return to the parent builder.
    ///
    /// Returns
    /// -------
    /// None
    fn finish(mut self_: PyRefMut<'_, Self>) -> PyResult<PyModelBuilder> {
        let mut self_owned = std::mem::take(&mut *self_);
        let parent = self_owned.parent_builder.take()
            .ok_or_else(|| PyValueError::new_err("Builder already finished"))?;

        // Create mixed node using Rust builder API
        let mixed_builder = parent.mixed(&self_owned.node_id);
        
        let mut mixed_builder = if let Some(values) = self_owned.values.take() {
            mixed_builder.values(&values)
        } else {
            mixed_builder
        };

        mixed_builder = if let Some(forecast) = self_owned.forecast.take() {
            mixed_builder.forecast(forecast)
        } else {
            mixed_builder
        };

        mixed_builder = if let Some(formula) = self_owned.formula.take() {
            mixed_builder.formula(formula).map_err(stmt_to_py)?
        } else {
            mixed_builder
        };

        mixed_builder = if let Some(name) = self_owned.name.take() {
            mixed_builder.name(name)
        } else {
            mixed_builder
        };

        let parent = mixed_builder.finish();

        Ok(PyModelBuilder {
            state: BuilderState::Ready(parent),
        })
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(_py, "builder")?;
    module.setattr("__doc__", "Builder API for financial models.")?;

    module.add_class::<PyModelBuilder>()?;
    module.add_class::<PyMixedNodeBuilder>()?;

    parent.add_submodule(&module)?;
    parent.setattr("builder", &module)?;

    Ok(vec!["ModelBuilder", "MixedNodeBuilder"])
}
