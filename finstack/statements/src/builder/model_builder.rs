//! Type-state builder pattern for financial models.

use crate::error::{Error, Result};
use crate::registry::dynamic::StoredMetric;
use crate::types::{AmountOrScalar, FinancialModelSpec, NodeSpec, NodeType};
use finstack_core::dates::{build_periods, Period, PeriodId};
use indexmap::{IndexMap, IndexSet};
use std::marker::PhantomData;

/// Validate that a node ID does not use reserved prefixes.
///
/// The `__cs__` prefix is reserved for internal capital structure references,
/// and `__` prefix is reserved for other internal use.
fn validate_node_id(node_id: &str) -> Result<()> {
    if node_id.contains("__cs__") {
        return Err(Error::build(format!(
            "Node ID '{}' contains reserved prefix '__cs__'. \
             This prefix is used internally for capital structure references.",
            node_id
        )));
    }
    if node_id.starts_with("__") {
        return Err(Error::build(format!(
            "Node ID '{}' cannot start with '__' (reserved for internal use)",
            node_id
        )));
    }
    Ok(())
}

/// Type-state marker: Periods not yet defined
#[derive(Debug)]
pub struct NeedPeriods;

/// Type-state marker: Ready to add nodes
#[derive(Debug)]
pub struct Ready;

/// Builder for financial models with compile-time type-state enforcement.
///
/// The builder uses a type-state pattern to ensure correct usage:
/// 1. Start with `ModelBuilder::new()` → `ModelBuilder<NeedPeriods>`
/// 2. Call `.periods()` → `ModelBuilder<Ready>`
/// 3. Add nodes, forecasts, etc.
/// 4. Call `.build()` → `FinancialModelSpec`
///
/// # Example
///
/// ```rust
/// use finstack_statements::builder::ModelBuilder;
/// use finstack_statements::types::{AmountOrScalar, NodeType};
/// use finstack_core::dates::PeriodId;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let model = ModelBuilder::new("test_model")
///     .periods("2025Q1..Q4", None)?
///     .value("revenue", &[
///         (PeriodId::quarter(2025, 1), AmountOrScalar::scalar(100.0)),
///     ])
///     .build()?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct ModelBuilder<State> {
    id: String,
    pub(crate) periods: Vec<Period>,
    pub(crate) nodes: IndexMap<String, NodeSpec>,
    meta: IndexMap<String, serde_json::Value>,
    pub(crate) capital_structure: Option<crate::types::CapitalStructureSpec>,
    alias_registry: Option<crate::registry::AliasRegistry>,
    _state: PhantomData<State>,
}

impl ModelBuilder<NeedPeriods> {
    /// Create a new model builder.
    ///
    /// # Arguments
    /// * `id` - Unique identifier for the model
    ///
    /// You must call `.periods()` before adding nodes.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            periods: Vec::new(),
            nodes: IndexMap::new(),
            meta: IndexMap::new(),
            capital_structure: None,
            alias_registry: None,
            _state: PhantomData,
        }
    }

    /// Define periods using a range expression.
    ///
    /// # Arguments
    ///
    /// * `range` - Period range (e.g., "2025Q1..Q4", "2025Q1..2026Q2")
    /// * `actuals_until` - Optional cutoff for actuals (e.g., Some("2025Q2"))
    pub fn periods(self, range: &str, actuals_until: Option<&str>) -> Result<ModelBuilder<Ready>> {
        let period_plan = build_periods(range, actuals_until)?;

        if period_plan.periods.is_empty() {
            return Err(Error::period(
                "Period range must contain at least one period",
            ));
        }

        Ok(ModelBuilder {
            id: self.id,
            periods: period_plan.periods,
            nodes: self.nodes,
            meta: self.meta,
            capital_structure: self.capital_structure,
            alias_registry: self.alias_registry,
            _state: PhantomData,
        })
    }

    /// Define periods explicitly (for advanced use cases).
    ///
    /// # Arguments
    /// * `periods` - Vector of [`Period`](finstack_core::dates::Period) instances, typically
    ///   produced by `finstack_core::dates::build_periods`
    pub fn periods_explicit(self, periods: Vec<Period>) -> Result<ModelBuilder<Ready>> {
        if periods.is_empty() {
            return Err(Error::period(
                "Period list must contain at least one period",
            ));
        }

        Ok(ModelBuilder {
            id: self.id,
            periods,
            nodes: self.nodes,
            meta: self.meta,
            capital_structure: self.capital_structure,
            alias_registry: self.alias_registry,
            _state: PhantomData,
        })
    }
}

impl ModelBuilder<Ready> {
    fn insert_metric_node(
        &mut self,
        qualified_id: &str,
        stored_metric: &StoredMetric,
        formula: String,
    ) {
        let key = qualified_id.to_string();
        let node = NodeSpec::new(key.clone(), NodeType::Calculated)
            .with_name(stored_metric.definition.name.clone())
            .with_formula(formula);
        self.nodes.insert(key, node);
    }

    /// Add a value node with explicit period values.
    ///
    /// Value nodes contain only explicit data (actuals or assumptions).
    ///
    /// # Arguments
    /// * `node_id` - Identifier for the node to create
    /// * `values` - Slice of `(PeriodId, AmountOrScalar)` tuples representing actual values
    ///
    /// # Example
    ///
    /// ```rust
    /// # use finstack_statements::builder::ModelBuilder;
    /// # use finstack_statements::types::AmountOrScalar;
    /// # use finstack_core::dates::PeriodId;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let model = ModelBuilder::new("test")
    ///     .periods("2025Q1..Q2", None)?
    ///     .value("revenue", &[
    ///         (PeriodId::quarter(2025, 1), AmountOrScalar::scalar(100_000.0)),
    ///         (PeriodId::quarter(2025, 2), AmountOrScalar::scalar(110_000.0)),
    ///     ])
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    #[must_use = "builder methods must be chained"]
    pub fn value(
        mut self,
        node_id: impl Into<String>,
        values: &[(PeriodId, AmountOrScalar)],
    ) -> Self {
        let node_id = node_id.into();
        let values_map: IndexMap<PeriodId, AmountOrScalar> = values.iter().cloned().collect();

        let node = NodeSpec::new(node_id.clone(), NodeType::Value).with_values(values_map);

        self.nodes.insert(node_id, node);
        self
    }

    /// Add a monetary value node with Money values.
    ///
    /// This is a type-safe way to create value nodes that explicitly represent
    /// monetary amounts with currency. The node will be tracked as a Monetary type.
    ///
    /// # Arguments
    /// * `node_id` - Identifier for the node to create
    /// * `values` - Slice of `(PeriodId, Money)` tuples representing monetary values
    ///
    /// # Example
    ///
    /// ```rust
    /// # use finstack_statements::builder::ModelBuilder;
    /// # use finstack_core::money::Money;
    /// # use finstack_core::currency::Currency;
    /// # use finstack_core::dates::PeriodId;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let model = ModelBuilder::new("test")
    ///     .periods("2025Q1..Q2", None)?
    ///     .value_money("revenue", &[
    ///         (PeriodId::quarter(2025, 1), Money::new(100_000.0, Currency::USD)),
    ///         (PeriodId::quarter(2025, 2), Money::new(110_000.0, Currency::USD)),
    ///     ])
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    #[must_use = "builder methods must be chained"]
    pub fn value_money(
        mut self,
        node_id: impl Into<String>,
        values: &[(PeriodId, finstack_core::money::Money)],
    ) -> Self {
        let node_id = node_id.into();
        let values_map: IndexMap<PeriodId, AmountOrScalar> = values
            .iter()
            .map(|(period_id, money)| (*period_id, AmountOrScalar::Amount(*money)))
            .collect();

        // Get currency from first value for type tracking
        let value_type = values
            .first()
            .map(|(_, money)| crate::types::NodeValueType::Monetary {
                currency: money.currency(),
            });

        let mut node = NodeSpec::new(node_id.clone(), NodeType::Value).with_values(values_map);
        node.value_type = value_type;

        self.nodes.insert(node_id, node);
        self
    }

    /// Add a scalar value node.
    ///
    /// This is a convenience method for creating value nodes that represent
    /// non-monetary scalars (ratios, percentages, counts, etc.).
    ///
    /// # Arguments
    /// * `node_id` - Identifier for the node to create
    /// * `values` - Slice of `(PeriodId, f64)` tuples representing scalar values
    ///
    /// # Example
    ///
    /// ```rust
    /// # use finstack_statements::builder::ModelBuilder;
    /// # use finstack_core::dates::PeriodId;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let model = ModelBuilder::new("test")
    ///     .periods("2025Q1..Q2", None)?
    ///     .value_scalar("gross_margin_pct", &[
    ///         (PeriodId::quarter(2025, 1), 0.35),
    ///         (PeriodId::quarter(2025, 2), 0.37),
    ///     ])
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    #[must_use = "builder methods must be chained"]
    pub fn value_scalar(mut self, node_id: impl Into<String>, values: &[(PeriodId, f64)]) -> Self {
        let node_id = node_id.into();
        let values_map: IndexMap<PeriodId, AmountOrScalar> = values
            .iter()
            .map(|(period_id, value)| (*period_id, AmountOrScalar::Scalar(*value)))
            .collect();

        let mut node = NodeSpec::new(node_id.clone(), NodeType::Value).with_values(values_map);
        node.value_type = Some(crate::types::NodeValueType::Scalar);

        self.nodes.insert(node_id, node);
        self
    }

    /// Add a calculated node with a formula.
    ///
    /// Calculated nodes derive their values from formulas only.
    ///
    /// # Arguments
    /// * `node_id` - Identifier for the node to create
    /// * `formula` - Statements DSL expression (e.g., `"revenue - cogs"`)
    ///
    /// # Example
    ///
    /// ```rust
    /// # use finstack_statements::builder::ModelBuilder;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let model = ModelBuilder::new("test")
    ///     .periods("2025Q1..Q2", None)?
    ///     .compute("gross_profit", "revenue - cogs")?
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    #[must_use = "builder methods must be chained"]
    pub fn compute(
        mut self,
        node_id: impl Into<String>,
        formula: impl Into<String>,
    ) -> Result<Self> {
        let node_id = node_id.into();
        let formula = formula.into();

        // Validate node ID doesn't use reserved prefixes
        validate_node_id(&node_id)?;

        // Basic validation: formula should not be empty
        if formula.trim().is_empty() {
            return Err(Error::formula_parse("Formula cannot be empty"));
        }

        // Validate formula syntax by attempting to parse and compile
        // This catches syntax errors and invalid function arguments early
        crate::dsl::parse_and_compile(&formula)?;

        let node = NodeSpec::new(node_id.clone(), NodeType::Calculated).with_formula(formula);

        self.nodes.insert(node_id, node);
        Ok(self)
    }

    /// Create a mixed node with explicit configuration.
    ///
    /// Mixed nodes support Value, Forecast, and Formula with precedence: Value > Forecast > Formula.
    /// This method returns a fluent builder for configuring all aspects of a mixed node.
    ///
    /// # Arguments
    /// * `node_id` - Identifier for the mixed node being configured
    ///
    /// # Example
    ///
    /// ```rust
    /// # use finstack_statements::builder::ModelBuilder;
    /// # use finstack_statements::types::{AmountOrScalar, ForecastSpec, ForecastMethod};
    /// # use finstack_core::dates::PeriodId;
    /// # use indexmap::indexmap;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let model = ModelBuilder::new("test")
    ///     .periods("2025Q1..Q4", Some("2025Q2"))?
    ///     .mixed("revenue")
    ///         .values(&[
    ///             (PeriodId::quarter(2025, 1), AmountOrScalar::scalar(100_000.0)),
    ///             (PeriodId::quarter(2025, 2), AmountOrScalar::scalar(110_000.0)),
    ///         ])
    ///         .forecast(ForecastSpec {
    ///             method: ForecastMethod::GrowthPct,
    ///             params: indexmap! { "rate".into() => serde_json::json!(0.05) },
    ///         })
    ///         .formula("lag(revenue, 1) * 1.05")?
    ///         .finish()
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    #[must_use = "builder methods must be chained"]
    pub fn mixed(self, node_id: impl Into<String>) -> MixedNodeBuilder {
        MixedNodeBuilder {
            parent: self,
            node_id: node_id.into(),
            values: None,
            forecast: None,
            formula: None,
            name: None,
        }
    }

    /// Add a forecast specification to an existing node.
    ///
    /// This allows forecasting values into future periods using various methods.
    ///
    /// # Arguments
    /// * `node_id` - Identifier of the node to augment (created previously)
    /// * `forecast_spec` - Forecast configuration created with [`ForecastSpec`](crate::types::ForecastSpec)
    ///
    /// # Example
    ///
    /// ```rust
    /// # use finstack_statements::builder::ModelBuilder;
    /// # use finstack_statements::types::{AmountOrScalar, ForecastSpec, ForecastMethod};
    /// # use finstack_core::dates::PeriodId;
    /// # use indexmap::indexmap;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let model = ModelBuilder::new("test")
    ///     .periods("2025Q1..Q4", Some("2025Q2"))?
    ///     .value("revenue", &[
    ///         (PeriodId::quarter(2025, 1), AmountOrScalar::scalar(100_000.0)),
    ///         (PeriodId::quarter(2025, 2), AmountOrScalar::scalar(110_000.0)),
    ///     ])
    ///     .forecast("revenue", ForecastSpec {
    ///         method: ForecastMethod::GrowthPct,
    ///         params: indexmap! { "rate".into() => serde_json::json!(0.05) },
    ///     })
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    #[must_use = "builder methods must be chained"]
    pub fn forecast(
        mut self,
        node_id: impl Into<String>,
        forecast_spec: crate::types::ForecastSpec,
    ) -> Self {
        let node_id = node_id.into();

        // Get or create the node (converting to Mixed type if needed)
        if let Some(node) = self.nodes.get_mut(&node_id) {
            // Set forecast on existing node
            node.forecast = Some(forecast_spec);

            // Ensure node type is Mixed if it has a forecast
            if matches!(node.node_type, NodeType::Value) {
                node.node_type = NodeType::Mixed;
            }
        } else {
            // Create new Mixed node with just the forecast
            let node = NodeSpec::new(node_id.clone(), NodeType::Mixed).with_forecast(forecast_spec);
            self.nodes.insert(node_id, node);
        }

        self
    }

    /// Add metadata to the model.
    ///
    /// # Arguments
    /// * `key` - Metadata key
    /// * `value` - Arbitrary JSON payload
    ///
    /// # Example
    ///
    /// ```rust
    /// # use finstack_statements::builder::ModelBuilder;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let model = ModelBuilder::new("demo")
    ///     .periods("2025Q1..Q2", None)?
    ///     .with_meta("currency", serde_json::json!({ "code": "USD" }))
    ///     .build()?;
    /// assert_eq!(model.meta["currency"]["code"], "USD");
    /// # Ok(())
    /// # }
    /// ```
    #[must_use = "builder methods must be chained"]
    pub fn with_meta(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.meta.insert(key.into(), value);
        self
    }

    /// Add a where clause to the last added node.
    ///
    /// The where clause is a conditional expression that determines whether
    /// the node should be evaluated for a given period. If the where clause
    /// evaluates to false (0.0), the node value will be set to 0.0 for that period.
    ///
    /// # Arguments
    /// * `where_clause` - DSL expression evaluated as a predicate
    ///
    /// # Example
    ///
    /// ```rust
    /// # use finstack_statements::prelude::*;
    /// # use finstack_core::dates::PeriodId;
    /// # fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    /// let model = ModelBuilder::new("test")
    ///     .periods("2025Q1..Q4", Some("2025Q2"))?
    ///     .value("revenue", &[(PeriodId::quarter(2025, 1), AmountOrScalar::scalar(1500000.0))])
    ///     .compute("bonus", "revenue * 0.01")?
    ///     .where_clause("revenue > 1000000")  // Only compute bonus if revenue > 1M
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    #[must_use = "builder methods must be chained"]
    pub fn where_clause(mut self, where_clause: impl Into<String>) -> Self {
        if let Some((_, last_node)) = self.nodes.last_mut() {
            last_node.where_text = Some(where_clause.into());
        }
        self
    }

    /// Load built-in metrics (fin.* namespace) and add them to the model.
    ///
    /// This is a convenience method that loads standard financial metrics
    /// and adds all of them to the model.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use finstack_statements::builder::ModelBuilder;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let model = ModelBuilder::new("test")
    ///     .periods("2025Q1..Q2", None)?
    ///     .value("revenue", &[])
    ///     .value("cogs", &[])
    ///     .with_builtin_metrics()?
    ///     .build()?;
    ///
    /// // Now you can use metrics like fin.gross_profit
    /// assert!(model.has_node("fin.gross_profit"));
    /// # Ok(())
    /// # }
    /// ```
    #[must_use = "builder methods must be chained"]
    pub fn with_builtin_metrics(mut self) -> Result<Self> {
        let mut registry = crate::registry::Registry::new();
        registry.load_builtins()?;

        // Add all metrics from the registry as calculated nodes
        for (qualified_id, stored_metric) in registry.all_metrics() {
            self.insert_metric_node(
                qualified_id,
                stored_metric,
                stored_metric.definition.formula.clone(),
            );
        }

        Ok(self)
    }

    /// Load metrics from a JSON file and add them to the model.
    /// # Arguments
    /// * `path` - Path to a metrics JSON definition file
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use finstack_statements::builder::ModelBuilder;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let model = ModelBuilder::new("test")
    ///     .periods("2025Q1..Q2", None)?
    ///     .with_metrics("metrics/custom.json")?
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    #[must_use = "builder methods must be chained"]
    pub fn with_metrics(mut self, path: &str) -> Result<Self> {
        let mut registry = crate::registry::Registry::new();
        registry.load_from_json(path)?;

        // Add all metrics from the registry as calculated nodes
        for (qualified_id, stored_metric) in registry.all_metrics() {
            self.insert_metric_node(
                qualified_id,
                stored_metric,
                stored_metric.definition.formula.clone(),
            );
        }

        Ok(self)
    }

    /// Add a specific metric from the built-in registry.
    ///
    /// This is a convenience method that loads the built-in metrics registry
    /// and adds a specific metric to the model.
    ///
    /// # Arguments
    /// * `qualified_id` - Fully qualified metric identifier (e.g., `"fin.gross_margin"`)
    ///
    /// # Example
    ///
    /// ```rust
    /// # use finstack_statements::builder::ModelBuilder;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let model = ModelBuilder::new("test")
    ///     .periods("2025Q1..Q2", None)?
    ///     .value("revenue", &[])
    ///     .value("cogs", &[])
    ///     .add_metric("fin.gross_profit")?
    ///     .add_metric("fin.gross_margin")?
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn add_metric(self, qualified_id: &str) -> Result<Self> {
        let mut registry = crate::registry::Registry::new();
        registry.load_builtins()?;
        self.add_metric_from_registry(qualified_id, &registry)
    }

    /// Add a specific metric from a registry.
    ///
    /// This allows selectively adding metrics from a registry instead of
    /// adding all of them.
    ///
    /// # Arguments
    /// * `qualified_id` - Fully qualified metric identifier to add
    /// * `registry` - Registry loaded by the caller (allows reuse across builders)
    ///
    /// # Example
    ///
    /// ```rust
    /// # use finstack_statements::builder::ModelBuilder;
    /// # use finstack_statements::registry::Registry;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut registry = Registry::new();
    /// registry.load_builtins()?;
    ///
    /// let model = ModelBuilder::new("test")
    ///     .periods("2025Q1..Q2", None)?
    ///     .value("revenue", &[])
    ///     .value("cogs", &[])
    ///     .add_metric_from_registry("fin.gross_profit", &registry)?
    ///     .add_metric_from_registry("fin.gross_margin", &registry)?
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn add_metric_from_registry(
        mut self,
        qualified_id: &str,
        registry: &crate::registry::Registry,
    ) -> Result<Self> {
        // Get dependencies (in dependency order)
        let dependencies = registry.get_metric_dependencies(qualified_id)?;

        // Extract namespace from qualified_id
        let namespace = qualified_id
            .split('.')
            .next()
            .ok_or_else(|| Error::registry(format!(
                "Invalid qualified ID '{}'. Expected format: 'namespace.metric_id' (e.g., 'fin.gross_margin')",
                qualified_id
            )))?;

        // Add all dependencies first (if not already added)
        for dep_id in dependencies {
            if !self.nodes.contains_key(&dep_id) {
                let dep_metric = registry.get(&dep_id)?;

                // Update formula to use qualified references for metrics in the same namespace
                let formula = self.qualify_metric_references(
                    &dep_metric.definition.formula,
                    namespace,
                    registry,
                )?;

                self.insert_metric_node(&dep_id, dep_metric, formula);
            }
        }

        // Add the requested metric (if not already added)
        if !self.nodes.contains_key(qualified_id) {
            let stored_metric = registry.get(qualified_id)?;

            // Update formula to use qualified references for metrics in the same namespace
            let formula = self.qualify_metric_references(
                &stored_metric.definition.formula,
                namespace,
                registry,
            )?;

            self.insert_metric_node(qualified_id, stored_metric, formula);
        }

        Ok(self)
    }

    /// Replace unqualified metric references with qualified ones in a formula.
    fn qualify_metric_references(
        &self,
        formula: &str,
        namespace: &str,
        registry: &crate::registry::Registry,
    ) -> Result<String> {
        // Get all metrics in this namespace
        let metrics_in_namespace: IndexSet<String> = registry
            .namespace(namespace)
            .map(|(id, _)| {
                // Extract unqualified ID
                id.strip_prefix(&format!("{}.", namespace))
                    .unwrap_or(id)
                    .to_string()
            })
            .collect();

        // Use shared utility to qualify identifiers
        Ok(crate::utils::formula::qualify_identifiers(
            formula,
            &metrics_in_namespace,
            namespace,
        ))
    }

    /// Build the final model specification.
    ///
    /// This validates the model and returns a `FinancialModelSpec`.
    pub fn build(self) -> Result<FinancialModelSpec> {
        // Validate that we have at least one period
        if self.periods.is_empty() {
            return Err(Error::build("Model must have at least one period"));
        }

        // Validate all node IDs don't use reserved prefixes
        for node_id in self.nodes.keys() {
            validate_node_id(node_id)?;
        }

        // Create the model spec
        let mut spec = FinancialModelSpec::new(self.id, self.periods);
        spec.nodes = self.nodes;
        spec.meta = self.meta;
        spec.capital_structure = self.capital_structure;

        Ok(spec)
    }
}

/// Fluent builder for mixed nodes.
///
/// This builder allows configuring all aspects of a mixed node (values, forecast, formula)
/// in a fluent manner before adding it to the model.
#[derive(Debug)]
pub struct MixedNodeBuilder {
    parent: ModelBuilder<Ready>,
    node_id: String,
    values: Option<IndexMap<PeriodId, AmountOrScalar>>,
    forecast: Option<crate::types::ForecastSpec>,
    formula: Option<String>,
    name: Option<String>,
}

impl MixedNodeBuilder {
    /// Set explicit values for the mixed node.
    ///
    /// # Arguments
    /// * `values` - Slice of `(PeriodId, AmountOrScalar)` tuples to seed actual periods
    ///
    /// # Example
    ///
    /// ```rust
    /// # use finstack_statements::builder::ModelBuilder;
    /// # use finstack_core::dates::PeriodId;
    /// # use finstack_statements::types::AmountOrScalar;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let builder = ModelBuilder::new("demo")
    ///     .periods("2025Q1..Q2", None)?
    ///     .mixed("revenue")
    ///     .values(&[(PeriodId::quarter(2025, 1), AmountOrScalar::scalar(100.0))])
    ///     .finish();
    /// # Ok(())
    /// # }
    /// ```
    #[must_use = "builder methods must be chained"]
    pub fn values(mut self, values: &[(PeriodId, AmountOrScalar)]) -> Self {
        self.values = Some(values.iter().cloned().collect());
        self
    }

    /// Set the forecast specification.
    ///
    /// # Arguments
    /// * `forecast_spec` - Forecast configuration created with [`ForecastSpec`](crate::types::ForecastSpec)
    ///
    /// # Example
    ///
    /// ```rust
    /// # use finstack_statements::builder::ModelBuilder;
    /// # use finstack_statements::types::ForecastSpec;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let builder = ModelBuilder::new("demo")
    ///     .periods("2025Q1..Q2", None)?
    ///     .mixed("revenue")
    ///     .forecast(ForecastSpec::forward_fill())
    ///     .finish();
    /// # Ok(())
    /// # }
    /// ```
    #[must_use = "builder methods must be chained"]
    pub fn forecast(mut self, forecast_spec: crate::types::ForecastSpec) -> Self {
        self.forecast = Some(forecast_spec);
        self
    }

    /// Set the fallback formula.
    ///
    /// # Arguments
    /// * `formula` - DSL expression evaluated when explicit values or forecasts are absent
    ///
    /// # Example
    ///
    /// ```rust
    /// # use finstack_statements::builder::ModelBuilder;
    /// # use finstack_core::dates::PeriodId;
    /// # use finstack_statements::types::AmountOrScalar;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let builder = ModelBuilder::new("demo")
    ///     .periods("2025Q1..Q2", None)?
    ///     .mixed("revenue")
    ///     .values(&[(PeriodId::quarter(2025, 1), AmountOrScalar::scalar(100.0))])
    ///     .formula("lag(revenue, 1)")?
    ///     .finish();
    /// # Ok(())
    /// # }
    /// ```
    #[must_use = "builder methods must be chained"]
    pub fn formula(mut self, formula: impl Into<String>) -> Result<Self> {
        let formula = formula.into();

        // Validate formula syntax
        if formula.trim().is_empty() {
            return Err(Error::formula_parse("Formula cannot be empty"));
        }
        crate::dsl::parse_and_compile(&formula)?;

        self.formula = Some(formula);
        Ok(self)
    }

    /// Set the human-readable name.
    ///
    /// # Arguments
    /// * `name` - Display label used in reports or exports
    ///
    /// # Example
    ///
    /// ```rust
    /// # use finstack_statements::builder::ModelBuilder;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let builder = ModelBuilder::new("demo")
    ///     .periods("2025Q1..Q2", None)?
    ///     .mixed("revenue")
    ///     .name("Revenue (actual + forecast)")
    ///     .finish();
    /// # Ok(())
    /// # }
    /// ```
    #[must_use = "builder methods must be chained"]
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Finish building the mixed node and return to the parent builder.
    ///
    /// Note: Node ID validation happens in `ModelBuilder::build()`. If you need
    /// early validation, use `try_finish()` instead.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use finstack_statements::builder::ModelBuilder;
    /// # use finstack_statements::types::ForecastSpec;
    /// # use finstack_core::dates::PeriodId;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let model = ModelBuilder::new("demo")
    ///     .periods("2025Q1..Q2", None)?
    ///     .mixed("revenue")
    ///         .values(&[(PeriodId::quarter(2025, 1), 100.0.into())])
    ///         .forecast(ForecastSpec::forward_fill())
    ///         .formula("lag(revenue, 1)")?
    ///         .finish()
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    #[must_use = "builder methods must be chained"]
    pub fn finish(mut self) -> ModelBuilder<Ready> {
        let mut node = NodeSpec::new(self.node_id.clone(), NodeType::Mixed);

        if let Some(name) = self.name {
            node.name = Some(name);
        }
        if let Some(values) = self.values {
            node.values = Some(values);
        }
        if let Some(forecast) = self.forecast {
            node.forecast = Some(forecast);
        }
        if let Some(formula) = self.formula {
            node.formula_text = Some(formula);
        }

        self.parent.nodes.insert(self.node_id, node);
        self.parent
    }

    /// Finish building the mixed node with validation.
    ///
    /// This is like `finish()` but validates the node ID and returns a Result.
    ///
    /// # Errors
    ///
    /// Returns an error if the node ID uses a reserved prefix (`__cs__` or `__`).
    pub fn try_finish(self) -> Result<ModelBuilder<Ready>> {
        validate_node_id(&self.node_id)?;
        Ok(self.finish())
    }
}

impl ModelBuilder<Ready> {
    /// Enable name normalization with standard aliases.
    ///
    /// Loads standard accounting term aliases (e.g., "rev" → "revenue", "sales" → "revenue").
    ///
    /// # Example
    ///
    /// ```rust
    /// # use finstack_statements::builder::ModelBuilder;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let model = ModelBuilder::new("demo")
    ///     .periods("2025Q1..Q2", None)?
    ///     .with_name_normalization()
    ///     .compute("revenue", "100000")?
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    #[must_use = "builder methods take self by value and return the modified value"]
    pub fn with_name_normalization(mut self) -> Self {
        let mut registry = crate::registry::AliasRegistry::new();
        registry.load_standard_aliases();
        self.alias_registry = Some(registry);
        self
    }

    /// Set custom alias registry.
    ///
    /// # Arguments
    ///
    /// * `registry` - Custom alias registry
    ///
    /// # Example
    ///
    /// ```rust
    /// # use finstack_statements::builder::ModelBuilder;
    /// # use finstack_statements::registry::AliasRegistry;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut aliases = AliasRegistry::new();
    /// aliases.add_alias("rev", "revenue");
    ///
    /// let model = ModelBuilder::new("demo")
    ///     .periods("2025Q1..Q2", None)?
    ///     .with_aliases(aliases)
    ///     .compute("revenue", "100000")?
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    #[must_use = "builder methods take self by value and return the modified value"]
    pub fn with_aliases(mut self, registry: crate::registry::AliasRegistry) -> Self {
        self.alias_registry = Some(registry);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::dates::PeriodId;

    #[test]
    fn test_builder_type_state() {
        // This should compile: correct order
        let result = ModelBuilder::new("test")
            .periods("2025Q1..Q2", None)
            .expect("valid period range")
            .build();

        assert!(result.is_ok());
    }

    #[test]
    fn test_periods_validation() {
        // Empty range should error
        let _result = ModelBuilder::new("test").periods("2025Q1..Q1", None);

        // This might succeed (Q1..Q1 could be valid for a single period)
        // but let's test that periods_explicit rejects empty
        let empty_result = ModelBuilder::new("test").periods_explicit(vec![]);
        assert!(empty_result.is_err());
    }

    #[test]
    fn test_value_node() {
        let model = ModelBuilder::new("test")
            .periods("2025Q1..Q2", None)
            .expect("valid period range")
            .value(
                "revenue",
                &[
                    (PeriodId::quarter(2025, 1), AmountOrScalar::scalar(100.0)),
                    (PeriodId::quarter(2025, 2), AmountOrScalar::scalar(110.0)),
                ],
            )
            .build()
            .expect("valid model");

        assert_eq!(model.nodes.len(), 1);
        assert!(model.has_node("revenue"));
        assert_eq!(
            model
                .get_node("revenue")
                .expect("revenue node should exist")
                .node_type,
            NodeType::Value
        );
    }

    #[test]
    fn test_computed_node() {
        let model = ModelBuilder::new("test")
            .periods("2025Q1..Q2", None)
            .expect("valid period range")
            .compute("gross_profit", "revenue - cogs")
            .expect("valid formula")
            .build()
            .expect("valid model");

        assert_eq!(model.nodes.len(), 1);
        let node = model
            .get_node("gross_profit")
            .expect("gross_profit node should exist");
        assert_eq!(node.node_type, NodeType::Calculated);
        assert_eq!(
            node.formula_text
                .as_ref()
                .expect("formula_text should exist"),
            "revenue - cogs"
        );
    }

    #[test]
    fn test_empty_formula_error() {
        let result = ModelBuilder::new("test")
            .periods("2025Q1..Q2", None)
            .expect("valid period range")
            .compute("invalid", "");

        assert!(result.is_err());
    }

    #[test]
    fn test_multiple_nodes() {
        let model = ModelBuilder::new("test")
            .periods("2025Q1..Q4", Some("2025Q2"))
            .expect("valid period range")
            .value(
                "revenue",
                &[
                    (PeriodId::quarter(2025, 1), AmountOrScalar::scalar(100.0)),
                    (PeriodId::quarter(2025, 2), AmountOrScalar::scalar(110.0)),
                ],
            )
            .compute("cogs", "revenue * 0.6")
            .expect("valid formula")
            .compute("gross_profit", "revenue - cogs")
            .expect("valid formula")
            .build()
            .expect("valid model");

        assert_eq!(model.nodes.len(), 3);
        assert!(model.has_node("revenue"));
        assert!(model.has_node("cogs"));
        assert!(model.has_node("gross_profit"));

        // Check period actuals flags
        assert_eq!(model.periods.len(), 4);
        assert!(model.periods[0].is_actual); // Q1
        assert!(model.periods[1].is_actual); // Q2
        assert!(!model.periods[2].is_actual); // Q3 (forecast)
        assert!(!model.periods[3].is_actual); // Q4 (forecast)
    }
}
