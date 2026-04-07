//! Builder API for financial models.

use crate::core::dates::periods::PyPeriod;
use crate::statements::error::stmt_to_py;
use crate::statements::types::forecast::PyForecastSpec;
use crate::statements::types::model::PyFinancialModelSpec;
use crate::statements::types::value::PyAmountOrScalar;
use crate::statements::types::waterfall::PyWaterfallSpec;
use finstack_core::dates::PeriodId;
use finstack_statements::builder::{ModelBuilder, NeedPeriods, Ready};
use finstack_statements::types::AmountOrScalar;
use finstack_statements_analytics::templates::{
    real_estate, RealEstateExtension, TemplatesExtension, VintageExtension,
};
use pyo3::exceptions::PyRuntimeError;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAnyMethods, PyDict, PyList, PyModule, PyType};
use pyo3::Bound;
use std::str::FromStr;

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
    NeedPeriods(Option<ModelBuilder<NeedPeriods>>),
    Ready(Option<ModelBuilder<Ready>>),
    Consumed,
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
            state: BuilderState::NeedPeriods(Some(ModelBuilder::new(id))),
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
    /// None
    ///     Mutates the builder in place
    fn periods(&mut self, range: &str, actuals_until: Option<&str>) -> PyResult<()> {
        let builder = match &mut self.state {
            BuilderState::NeedPeriods(b) => b
                .take()
                .ok_or_else(|| PyRuntimeError::new_err("ModelBuilder internal state error"))?,
            BuilderState::Ready(_) => {
                return Err(PyValueError::new_err("periods() already called"));
            }
            BuilderState::Consumed => {
                return Err(PyValueError::new_err(
                    "ModelBuilder has been consumed (build() already called)",
                ));
            }
        };

        let ready = builder.periods(range, actuals_until).map_err(stmt_to_py)?;
        self.state = BuilderState::Ready(Some(ready));
        Ok(())
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
    /// None
    ///     Mutates the builder in place
    fn periods_explicit(&mut self, periods: Vec<PyPeriod>) -> PyResult<()> {
        let builder = match &mut self.state {
            BuilderState::NeedPeriods(b) => b
                .take()
                .ok_or_else(|| PyRuntimeError::new_err("ModelBuilder internal state error"))?,
            BuilderState::Ready(_) => {
                return Err(PyValueError::new_err("periods() already called"));
            }
            BuilderState::Consumed => {
                return Err(PyValueError::new_err(
                    "ModelBuilder has been consumed (build() already called)",
                ));
            }
        };

        let periods = periods.into_iter().map(|p| p.inner).collect();
        let ready = builder.periods_explicit(periods).map_err(stmt_to_py)?;
        self.state = BuilderState::Ready(Some(ready));
        Ok(())
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
    /// None
    ///     Mutates the builder in place
    fn value(&mut self, node_id: String, values: &Bound<'_, PyAny>) -> PyResult<()> {
        let values_vec = parse_period_values(values)?;

        let builder = self.take_ready_builder()?;
        let builder = builder.value(node_id, &values_vec);
        self.state = BuilderState::Ready(Some(builder));
        Ok(())
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

        if let Ok(dict) = values.cast::<PyDict>() {
            // Dict format
            for (key, value) in dict.iter() {
                let period_id: PyPeriodId = key.extract()?;
                let money: PyMoney = value.extract()?;
                values_vec.push((period_id.inner, money.inner));
            }
        } else if let Ok(list) = values.cast::<PyList>() {
            // List of tuples format
            for (idx, item) in list.iter().enumerate() {
                let (period, money) = item.extract::<(PyPeriodId, PyMoney)>().map_err(|err| {
                    PyValueError::new_err(format!(
                        "Invalid values[{idx}] (expected (PeriodId, Money)): {err}"
                    ))
                })?;
                values_vec.push((period.inner, money.inner));
            }
        } else {
            return Err(PyValueError::new_err(
                "values must be a dict or list of tuples",
            ));
        }

        let builder = self.take_ready_builder()?;
        let builder = builder.value_money(node_id, &values_vec);
        self.state = BuilderState::Ready(Some(builder));
        Ok(())
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

        if let Ok(dict) = values.cast::<PyDict>() {
            // Dict format
            for (key, value) in dict.iter() {
                let period_id: PyPeriodId = key.extract()?;
                let scalar: f64 = value.extract()?;
                values_vec.push((period_id.inner, scalar));
            }
        } else if let Ok(list) = values.cast::<PyList>() {
            // List of tuples format
            for (idx, item) in list.iter().enumerate() {
                let (period, scalar) = item.extract::<(PyPeriodId, f64)>().map_err(|err| {
                    PyValueError::new_err(format!(
                        "Invalid values[{idx}] (expected (PeriodId, float)): {err}"
                    ))
                })?;
                values_vec.push((period.inner, scalar));
            }
        } else {
            return Err(PyValueError::new_err(
                "values must be a dict or list of tuples",
            ));
        }

        let builder = self.take_ready_builder()?;
        let builder = builder.value_scalar(node_id, &values_vec);
        self.state = BuilderState::Ready(Some(builder));
        Ok(())
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
    /// None
    ///     Mutates the builder in place
    fn compute(&mut self, node_id: String, formula: String) -> PyResult<()> {
        let builder = self.take_ready_builder()?;
        let builder = builder.compute(node_id, formula).map_err(stmt_to_py)?;
        self.state = BuilderState::Ready(Some(builder));
        Ok(())
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
        let parent = self.take_ready_builder()?;
        // Mark this builder as temporarily consumed; the returned MixedNodeBuilder
        // will yield a new ModelBuilder when `build()` is called.
        self.state = BuilderState::Ready(None);

        Ok(PyMixedNodeBuilder {
            parent_builder: Some(parent),
            node_id,
            values: None,
            forecast: None,
            formula: None,
            name: None,
        })
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
    /// None
    ///     Mutates the builder in place
    fn forecast(&mut self, node_id: String, forecast_spec: &PyForecastSpec) -> PyResult<()> {
        let builder = self.take_ready_builder()?;
        let builder = builder.forecast(node_id, forecast_spec.inner.clone());
        self.state = BuilderState::Ready(Some(builder));
        Ok(())
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
    /// None
    ///     Mutates the builder in place
    fn with_meta(&mut self, key: String, value: &Bound<'_, PyAny>) -> PyResult<()> {
        let json_value = crate::statements::utils::py_to_json(value)?;

        let builder = self.take_ready_builder()?;
        let builder = builder.with_meta(key, json_value);
        self.state = BuilderState::Ready(Some(builder));
        Ok(())
    }

    #[pyo3(text_signature = "(self, waterfall_spec)")]
    /// Configure waterfall specification for dynamic cash flow allocation.
    ///
    /// Parameters
    /// ----------
    /// waterfall_spec : WaterfallSpec
    ///     Waterfall configuration with ECF sweep and PIK toggle settings
    ///
    /// Returns
    /// -------
    /// None
    ///     Mutates the builder in place
    fn waterfall(&mut self, waterfall_spec: PyWaterfallSpec) -> PyResult<()> {
        let builder = self.take_ready_builder()?;
        let builder = builder.waterfall(waterfall_spec.inner);
        self.state = BuilderState::Ready(Some(builder));
        Ok(())
    }

    #[pyo3(
        text_signature = "(self, id, notional, coupon_rate, issue_date, maturity_date, discount_curve_id)"
    )]
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
        use crate::core::dates::utils::py_to_date;

        let issue = py_to_date(issue_date)?;
        let maturity = py_to_date(maturity_date)?;

        let builder = self.take_ready_builder()?;
        let builder = builder
            .add_bond(
                id,
                notional.inner,
                coupon_rate,
                issue,
                maturity,
                discount_curve_id,
            )
            .map_err(stmt_to_py)?;
        self.state = BuilderState::Ready(Some(builder));
        Ok(())
    }

    #[pyo3(
        text_signature = "(self, id, notional, coupon_rate, issue_date, maturity_date, convention, discount_curve_id)"
    )]
    /// Add a bond with a named market convention to the capital structure.
    ///
    /// Uses pre-configured regional conventions that set day count, coupon
    /// frequency, settlement days, and business-day rules automatically.
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
    /// convention : str
    ///     Market convention name. Supported values:
    ///     ``"us_treasury"`` / ``"UST"``, ``"us_agency"``,
    ///     ``"german_bund"``, ``"uk_gilt"``, ``"french_oat"``,
    ///     ``"jgb"``, ``"corporate"``
    /// discount_curve_id : str
    ///     Discount curve ID for pricing
    ///
    /// Returns
    /// -------
    /// None
    #[allow(clippy::too_many_arguments)]
    fn add_bond_with_convention(
        &mut self,
        id: String,
        notional: &crate::core::money::PyMoney,
        coupon_rate: f64,
        issue_date: &Bound<'_, PyAny>,
        maturity_date: &Bound<'_, PyAny>,
        convention: &str,
        discount_curve_id: String,
    ) -> PyResult<()> {
        use crate::core::dates::utils::py_to_date;

        let issue = py_to_date(issue_date)?;
        let maturity = py_to_date(maturity_date)?;

        let bond_conv = finstack_valuations::instruments::BondConvention::from_str(convention)
            .map_err(|_| {
                PyValueError::new_err(format!(
                    "Unknown bond convention '{}'. Supported: us_treasury, UST, us_agency, \
                     german_bund, uk_gilt, french_oat, jgb, corporate",
                    convention
                ))
            })?;

        let rate = finstack_core::types::Rate::from_decimal(coupon_rate);

        let builder = self.take_ready_builder()?;
        let builder = builder
            .add_bond_with_convention(
                id,
                notional.inner,
                rate,
                issue,
                maturity,
                bond_conv,
                discount_curve_id,
            )
            .map_err(stmt_to_py)?;
        self.state = BuilderState::Ready(Some(builder));
        Ok(())
    }

    #[pyo3(
        text_signature = "(self, id, notional, fixed_rate, start_date, maturity_date, discount_curve_id, forward_curve_id)"
    )]
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
    #[allow(clippy::too_many_arguments)]
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
        use crate::core::dates::utils::py_to_date;

        let start = py_to_date(start_date)?;
        let maturity = py_to_date(maturity_date)?;

        let builder = self.take_ready_builder()?;
        let builder = builder
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
        self.state = BuilderState::Ready(Some(builder));
        Ok(())
    }

    #[pyo3(
        text_signature = "(self, id, notional, fixed_rate, start_date, maturity_date, discount_curve_id, forward_curve_id, fixed_freq, fixed_dc, float_freq, float_dc, bdc)"
    )]
    /// Add an interest rate swap with explicit leg conventions.
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
    /// fixed_freq : Tenor
    ///     Fixed leg payment frequency (e.g., ``Tenor.semi_annual()``)
    /// fixed_dc : DayCount
    ///     Fixed leg day count (e.g., ``DayCount.THIRTY_360``)
    /// float_freq : Tenor
    ///     Float leg payment frequency (e.g., ``Tenor.quarterly()``)
    /// float_dc : DayCount
    ///     Float leg day count (e.g., ``DayCount.ACT_360``)
    /// bdc : BusinessDayConvention
    ///     Business day convention (e.g., ``BusinessDayConvention.MODIFIED_FOLLOWING``)
    ///
    /// Returns
    /// -------
    /// None
    #[allow(clippy::too_many_arguments)]
    fn add_swap_with_conventions(
        &mut self,
        id: String,
        notional: &crate::core::money::PyMoney,
        fixed_rate: f64,
        start_date: &Bound<'_, PyAny>,
        maturity_date: &Bound<'_, PyAny>,
        discount_curve_id: String,
        forward_curve_id: String,
        fixed_freq: &crate::core::dates::tenor::PyTenor,
        fixed_dc: &crate::core::dates::daycount::PyDayCount,
        float_freq: &crate::core::dates::tenor::PyTenor,
        float_dc: &crate::core::dates::daycount::PyDayCount,
        bdc: &crate::core::dates::calendar::PyBusinessDayConvention,
    ) -> PyResult<()> {
        use crate::core::dates::utils::py_to_date;

        let start = py_to_date(start_date)?;
        let maturity = py_to_date(maturity_date)?;

        let builder = self.take_ready_builder()?;
        let builder = builder
            .add_swap_with_conventions(
                id,
                notional.inner,
                fixed_rate,
                start,
                maturity,
                discount_curve_id,
                forward_curve_id,
                fixed_freq.inner,
                fixed_dc.inner,
                float_freq.inner,
                float_dc.inner,
                bdc.inner,
            )
            .map_err(stmt_to_py)?;
        self.state = BuilderState::Ready(Some(builder));
        Ok(())
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

        let builder = self.take_ready_builder()?;
        let builder = builder.add_custom_debt(id, json_value);
        self.state = BuilderState::Ready(Some(builder));
        Ok(())
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
        let builder = self.take_ready_builder()?;
        let builder = builder.with_builtin_metrics().map_err(stmt_to_py)?;
        self.state = BuilderState::Ready(Some(builder));
        Ok(())
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
        let builder = self.take_ready_builder()?;
        let builder = builder.with_metrics(path).map_err(stmt_to_py)?;
        self.state = BuilderState::Ready(Some(builder));
        Ok(())
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
        let builder = self.take_ready_builder()?;
        let builder = builder.add_metric(qualified_id).map_err(stmt_to_py)?;
        self.state = BuilderState::Ready(Some(builder));
        Ok(())
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
        let registry_ref: PyRef<'_, crate::statements::registry::PyRegistry> =
            registry.extract()?;

        let builder = self.take_ready_builder()?;
        let builder = builder
            .add_metric_from_registry(qualified_id, registry_ref.inner())
            .map_err(stmt_to_py)?;
        self.state = BuilderState::Ready(Some(builder));
        Ok(())
    }

    #[pyo3(text_signature = "(self, qualified_ids, registry)")]
    /// Add multiple metrics from a registry at once.
    ///
    /// This is a convenience method for batch-adding metrics from a registry
    /// instead of calling add_metric_from_registry multiple times.
    ///
    /// Parameters
    /// ----------
    /// qualified_ids : list[str]
    ///     List of fully qualified metric identifiers to add
    /// registry : Registry
    ///     Registry loaded by the caller (allows reuse across builders)
    ///
    /// Returns
    /// -------
    /// None
    ///
    /// Examples
    /// --------
    /// >>> registry = Registry.new()
    /// >>> registry.load_builtins()
    /// >>> builder.add_registry_metrics(
    /// ...     ["fin.gross_margin", "fin.ebitda", "fin.net_income"],
    /// ...     registry
    /// ... )
    #[pyo3(signature = (qualified_ids, registry))]
    fn add_registry_metrics(
        &mut self,
        qualified_ids: Vec<String>,
        registry: Bound<'_, PyAny>,
    ) -> PyResult<()> {
        // Extract the PyRegistry directly
        let registry_ref: PyRef<'_, crate::statements::registry::PyRegistry> =
            registry.extract()?;

        let mut builder = self.take_ready_builder()?;

        // Add each metric in sequence
        for qualified_id in qualified_ids {
            builder = builder
                .add_metric_from_registry(&qualified_id, registry_ref.inner())
                .map_err(stmt_to_py)?;
        }

        self.state = BuilderState::Ready(Some(builder));
        Ok(())
    }

    #[pyo3(text_signature = "(self)")]
    /// Build the final model specification.
    ///
    /// Returns
    /// -------
    /// FinancialModelSpec
    ///     Complete model specification
    fn build(&mut self) -> PyResult<PyFinancialModelSpec> {
        let builder = self.take_ready_builder()?;
        let spec = builder.build().map_err(stmt_to_py)?;
        self.state = BuilderState::Consumed;
        Ok(PyFinancialModelSpec::new(spec))
    }

    #[pyo3(text_signature = "(self, name, increases, decreases)")]
    /// Add a roll-forward structure to the model.
    ///
    /// This creates:
    /// - `{name}_beg`: Beginning balance (linked to previous period's ending balance)
    /// - `{name}_end`: Ending balance (Begin + Increases - Decreases)
    ///
    /// Parameters
    /// ----------
    /// name : str
    ///     Base name for the roll-forward (e.g., "arr")
    /// increases : list[str]
    ///     List of node IDs that increase the balance
    /// decreases : list[str]
    ///     List of node IDs that decrease the balance
    ///
    /// Returns
    /// -------
    /// None
    ///     Mutates the builder in place
    fn add_roll_forward(
        &mut self,
        name: String,
        increases: Vec<String>,
        decreases: Vec<String>,
    ) -> PyResult<()> {
        let builder = self.take_ready_builder()?;

        // Convert Vec<String> to Vec<&str>
        let inc_refs: Vec<&str> = increases.iter().map(|s| s.as_str()).collect();
        let dec_refs: Vec<&str> = decreases.iter().map(|s| s.as_str()).collect();

        let builder = builder
            .add_roll_forward(&name, &inc_refs, &dec_refs)
            .map_err(stmt_to_py)?;
        self.state = BuilderState::Ready(Some(builder));
        Ok(())
    }

    #[pyo3(text_signature = "(self, name, new_volume_node, decay_curve)")]
    /// Add a vintage buildup (cohort analysis) structure.
    ///
    /// This models a "stack" of layers (cohorts) where each layer is generated
    /// by a "new volume" node and then decays/evolves according to a curve.
    ///
    /// The total value is the sum of all active cohorts:
    /// `Total[t] = Sum(NewVolume[t-k] * Curve[k])` for k = 0..curve_len
    ///
    /// Parameters
    /// ----------
    /// name : str
    ///     Name of the resulting total node (e.g., "revenue")
    /// new_volume_node : str
    ///     Node ID for the new volume per period (e.g., "new_sales")
    /// decay_curve : list[float]
    ///     Multipliers for the vintage curve (index 0 = inception, 1 = next period, etc.)
    ///
    /// Returns
    /// -------
    /// None
    ///     Mutates the builder in place
    fn add_vintage_buildup(
        &mut self,
        name: String,
        new_volume_node: String,
        decay_curve: Vec<f64>,
    ) -> PyResult<()> {
        let builder = self.take_ready_builder()?;
        let builder = builder
            .add_vintage_buildup(&name, &new_volume_node, &decay_curve)
            .map_err(stmt_to_py)?;
        self.state = BuilderState::Ready(Some(builder));
        Ok(())
    }

    // ── Real-estate template helpers ────────────────────────────

    /// Add a NOI (Net Operating Income) buildup template.
    ///
    /// Parameters
    /// ----------
    /// total_revenue_node : str
    ///     Node that aggregates all revenue line items.
    /// revenue_nodes : list[str]
    ///     Revenue line-item node IDs.
    /// total_expenses_node : str
    ///     Node that aggregates all expense line items.
    /// expense_nodes : list[str]
    ///     Expense line-item node IDs.
    /// noi_node : str
    ///     Target NOI node (``total_revenue - total_expenses``).
    fn add_noi_buildup(
        &mut self,
        total_revenue_node: String,
        revenue_nodes: Vec<String>,
        total_expenses_node: String,
        expense_nodes: Vec<String>,
        noi_node: String,
    ) -> PyResult<()> {
        let builder = self.take_ready_builder()?;
        let rev_refs: Vec<&str> = revenue_nodes.iter().map(|s| s.as_str()).collect();
        let exp_refs: Vec<&str> = expense_nodes.iter().map(|s| s.as_str()).collect();
        let builder = builder
            .add_noi_buildup(
                &total_revenue_node,
                &rev_refs,
                &total_expenses_node,
                &exp_refs,
                &noi_node,
            )
            .map_err(stmt_to_py)?;
        self.state = BuilderState::Ready(Some(builder));
        Ok(())
    }

    /// Add a NCF (Net Cash Flow) buildup template.
    ///
    /// Parameters
    /// ----------
    /// noi_node : str
    ///     NOI source node.
    /// capex_nodes : list[str]
    ///     Capital expenditure node IDs.
    /// ncf_node : str
    ///     Target NCF node (``NOI - capex``).
    fn add_ncf_buildup(
        &mut self,
        noi_node: String,
        capex_nodes: Vec<String>,
        ncf_node: String,
    ) -> PyResult<()> {
        let builder = self.take_ready_builder()?;
        let capex_refs: Vec<&str> = capex_nodes.iter().map(|s| s.as_str()).collect();
        let builder = builder
            .add_ncf_buildup(&noi_node, &capex_refs, &ncf_node)
            .map_err(stmt_to_py)?;
        self.state = BuilderState::Ready(Some(builder));
        Ok(())
    }

    /// Add a rent-roll rental revenue projection (v1, simple leases).
    ///
    /// Parameters
    /// ----------
    /// leases : list[SimpleLeaseSpec]
    ///     Lease specifications.
    /// total_rent_node : str
    ///     Target node for aggregated rental revenue.
    fn add_rent_roll_rental_revenue(
        &mut self,
        leases: Vec<crate::statements::templates::PySimpleLeaseSpec>,
        total_rent_node: String,
    ) -> PyResult<()> {
        let builder = self.take_ready_builder()?;
        let specs: Vec<_> = leases.into_iter().map(|l| l.inner).collect();
        let builder = real_estate::add_rent_roll_rental_revenue(builder, &specs, &total_rent_node)
            .map_err(stmt_to_py)?;
        self.state = BuilderState::Ready(Some(builder));
        Ok(())
    }

    /// Add a rent-roll rental revenue projection (enhanced leases).
    ///
    /// Parameters
    /// ----------
    /// leases : list[LeaseSpec]
    ///     Enhanced lease specifications with steps, windows, and renewal.
    /// nodes : RentRollOutputNodes
    ///     Output node names for the rent decomposition.
    fn add_rent_roll(
        &mut self,
        leases: Vec<crate::statements::templates::PyLeaseSpec>,
        nodes: crate::statements::templates::PyRentRollOutputNodes,
    ) -> PyResult<()> {
        let builder = self.take_ready_builder()?;
        let specs: Vec<_> = leases.into_iter().map(|l| l.inner).collect();
        let builder =
            real_estate::add_rent_roll(builder, &specs, &nodes.inner).map_err(stmt_to_py)?;
        self.state = BuilderState::Ready(Some(builder));
        Ok(())
    }

    /// Add a full property operating statement template.
    ///
    /// Combines rent roll, other income, expenses, management fees,
    /// and capex into a complete NOI-to-NCF waterfall.
    ///
    /// Parameters
    /// ----------
    /// leases : list[LeaseSpec]
    ///     Enhanced lease specifications.
    /// other_income_nodes : list[str]
    ///     Other income node IDs.
    /// opex_nodes : list[str]
    ///     Operating expense node IDs.
    /// capex_nodes : list[str]
    ///     Capital expenditure node IDs.
    /// management_fee : ManagementFeeSpec or None
    ///     Optional management fee specification.
    /// nodes : PropertyTemplateNodes
    ///     Node names for the property template output.
    #[pyo3(signature = (leases, other_income_nodes, opex_nodes, capex_nodes, nodes, management_fee=None))]
    fn add_property_operating_statement(
        &mut self,
        leases: Vec<crate::statements::templates::PyLeaseSpec>,
        other_income_nodes: Vec<String>,
        opex_nodes: Vec<String>,
        capex_nodes: Vec<String>,
        nodes: crate::statements::templates::PyPropertyTemplateNodes,
        management_fee: Option<crate::statements::templates::PyManagementFeeSpec>,
    ) -> PyResult<()> {
        let builder = self.take_ready_builder()?;
        let specs: Vec<_> = leases.into_iter().map(|l| l.inner).collect();
        let oi_refs: Vec<&str> = other_income_nodes.iter().map(|s| s.as_str()).collect();
        let opex_refs: Vec<&str> = opex_nodes.iter().map(|s| s.as_str()).collect();
        let capex_refs: Vec<&str> = capex_nodes.iter().map(|s| s.as_str()).collect();
        let builder = builder
            .add_property_operating_statement(
                &specs,
                &oi_refs,
                &opex_refs,
                &capex_refs,
                management_fee.map(|m| m.inner),
                &nodes.inner,
            )
            .map_err(stmt_to_py)?;
        self.state = BuilderState::Ready(Some(builder));
        Ok(())
    }
}

impl PyModelBuilder {
    fn take_ready_builder(&mut self) -> PyResult<ModelBuilder<Ready>> {
        match &mut self.state {
            BuilderState::Ready(b) => b
                .take()
                .ok_or_else(|| PyRuntimeError::new_err("ModelBuilder internal state error")),
            BuilderState::NeedPeriods(_) => Err(PyValueError::new_err("Must call periods() first")),
            BuilderState::Consumed => Err(PyValueError::new_err(
                "ModelBuilder has been consumed (build() already called)",
            )),
        }
    }
}

/// Helper to parse period values from dict or list of tuples.
fn parse_period_values(values: &Bound<'_, PyAny>) -> PyResult<Vec<(PeriodId, AmountOrScalar)>> {
    let mut vec = Vec::new();

    if let Ok(dict) = values.cast::<PyDict>() {
        // Dict format
        for (key, value) in dict.iter() {
            let period_id: crate::core::dates::periods::PyPeriodId = key.extract()?;
            let amount: PyAmountOrScalar = value.extract()?;
            vec.push((period_id.inner, amount.inner));
        }
    } else if let Ok(list) = values.cast::<PyList>() {
        // List of tuples format
        for (idx, item) in list.iter().enumerate() {
            let (period, amount) = item
                .extract::<(crate::core::dates::periods::PyPeriodId, PyAmountOrScalar)>()
                .map_err(|err| {
                    PyValueError::new_err(format!(
                        "Invalid values[{idx}] (expected (PeriodId, AmountOrScalar)): {err}"
                    ))
                })?;
            vec.push((period.inner, amount.inner));
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
#[derive(Default)]
pub struct PyMixedNodeBuilder {
    parent_builder: Option<ModelBuilder<Ready>>,
    node_id: String,
    values: Option<Vec<(PeriodId, AmountOrScalar)>>,
    forecast: Option<finstack_statements::types::ForecastSpec>,
    formula: Option<String>,
    name: Option<String>,
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
    /// None
    ///     Mutates the mixed builder in place
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
    /// None
    ///     Mutates the mixed builder in place
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
    /// None
    ///     Mutates the mixed builder in place
    fn formula(&mut self, formula: String) -> PyResult<()> {
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
    /// None
    ///     Mutates the mixed builder in place
    fn name(&mut self, name: String) -> PyResult<()> {
        self.name = Some(name);
        Ok(())
    }

    #[pyo3(text_signature = "(self)")]
    /// Build the mixed node and return to the parent model builder.
    ///
    /// Returns
    /// -------
    /// ModelBuilder
    ///     Parent model builder instance
    fn build(mut self_: PyRefMut<'_, Self>) -> PyResult<PyModelBuilder> {
        let mut self_owned = std::mem::take(&mut *self_);
        let parent = self_owned
            .parent_builder
            .take()
            .ok_or_else(|| PyValueError::new_err("Builder already built"))?;

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

        let parent = mixed_builder.build();

        Ok(PyModelBuilder {
            state: BuilderState::Ready(Some(parent)),
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

    let exports = vec!["ModelBuilder", "MixedNodeBuilder"];
    module.setattr("__all__", PyList::new(_py, &exports)?)?;
    parent.add_submodule(&module)?;
    parent.setattr("builder", &module)?;

    Ok(exports)
}
