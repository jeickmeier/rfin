//! Type-state builder pattern for financial models.

use crate::error::{Error, Result};
use crate::registry::dynamic::StoredMetric;
use crate::types::{AmountOrScalar, FinancialModelSpec, NodeId, NodeSpec, NodeType};
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

fn replace_standalone_identifier(formula: &str, identifier: &str, replacement: &str) -> String {
    const MAX_REPLACE_ITERATIONS: usize = 1_000_000;
    let mut result = formula.to_string();
    let mut idx = 0;
    let mut iterations = 0usize;
    while let Some(pos) = result[idx..].find(identifier) {
        iterations += 1;
        if iterations > MAX_REPLACE_ITERATIONS {
            break;
        }
        let abs_pos = idx + pos;
        let end_pos = abs_pos + identifier.len();
        if crate::utils::formula::is_standalone_identifier(&result, abs_pos, end_pos, false) {
            result.replace_range(abs_pos..end_pos, replacement);
            idx = abs_pos + replacement.len();
        } else {
            idx = end_pos;
        }
    }
    result
}

fn normalize_formula_aliases(
    formula: &str,
    registry: &crate::registry::AliasRegistry,
    available_nodes: &IndexSet<String>,
) -> Result<String> {
    let identifiers = crate::utils::formula::extract_all_identifiers(formula)?;
    let mut normalized = formula.to_string();
    let mut ordered: Vec<_> = identifiers.into_iter().collect();
    ordered.sort_by_key(|id| std::cmp::Reverse(id.len()));

    for identifier in ordered {
        let replacement = registry
            .normalize(&identifier)
            .or_else(|| registry.normalize_fuzzy(&identifier, available_nodes));
        if let Some(replacement) = replacement {
            if replacement != identifier {
                normalized = replace_standalone_identifier(&normalized, &identifier, &replacement);
            }
        }
    }

    Ok(normalized)
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
/// 1. Start with [`FinancialModelSpec::builder`](crate::types::FinancialModelSpec::builder)
///    or `ModelBuilder::new()` → `ModelBuilder<NeedPeriods>`
/// 2. Call `.periods()` → `ModelBuilder<Ready>`
/// 3. Add nodes, forecasts, etc.
/// 4. Call `.build()` → `FinancialModelSpec`
///
/// # Example
///
/// ```rust
/// use finstack_statements::types::{AmountOrScalar, FinancialModelSpec, NodeType};
/// use finstack_core::dates::PeriodId;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let model = FinancialModelSpec::builder("test_model")
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
    pub(crate) nodes: IndexMap<NodeId, NodeSpec>,
    meta: IndexMap<String, serde_json::Value>,
    pub(crate) capital_structure: Option<crate::types::CapitalStructureSpec>,
    alias_registry: Option<crate::registry::AliasRegistry>,
    _state: PhantomData<State>,
}

impl<State> ModelBuilder<State> {
    /// Insert a pre-built node into the model.
    ///
    /// This is an advanced API for template builders that need to construct
    /// nodes programmatically. Prefer `.compute()` and `.value()` for
    /// standard model construction.
    pub fn insert_node(&mut self, id: NodeId, spec: NodeSpec) -> &mut Self {
        self.nodes.insert(id, spec);
        self
    }

    /// Return a read-only slice of the model's periods.
    ///
    /// This is an advanced API primarily for template builders in external crates
    /// that need to iterate over periods to generate per-period value nodes.
    pub fn periods_slice(&self) -> &[Period] {
        &self.periods
    }
}

impl ModelBuilder<NeedPeriods> {
    /// Create a new model builder.
    ///
    /// # Arguments
    /// * `id` - Unique identifier for the model
    ///
    /// You must call `.periods()` before adding nodes.
    #[must_use]
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
        let key = NodeId::new(qualified_id);
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
        node_id: impl Into<NodeId>,
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
        node_id: impl Into<NodeId>,
        values: &[(PeriodId, finstack_core::money::Money)],
    ) -> Self {
        let node_id = node_id.into();
        let values_map: IndexMap<PeriodId, AmountOrScalar> = values
            .iter()
            .map(|(period_id, money)| (*period_id, AmountOrScalar::Amount(*money)))
            .collect();

        // Validate all values share the same currency
        if values.len() > 1 {
            let first_currency = values[0].1.currency();
            for (period_id, money) in values.iter().skip(1) {
                if money.currency() != first_currency {
                    tracing::warn!(
                        "value_money('{}') has mixed currencies: {:?} at {:?} vs {:?}",
                        node_id,
                        money.currency(),
                        period_id,
                        first_currency
                    );
                }
            }
        }

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
    pub fn value_scalar(mut self, node_id: impl Into<NodeId>, values: &[(PeriodId, f64)]) -> Self {
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
        node_id: impl Into<NodeId>,
        formula: impl Into<String>,
    ) -> Result<Self> {
        let node_id = node_id.into();
        let formula = formula.into();

        // Validate node ID doesn't use reserved prefixes
        validate_node_id(node_id.as_str())?;

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
    ///         .build()
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    #[must_use = "builder methods must be chained"]
    pub fn mixed(self, node_id: impl Into<NodeId>) -> MixedNodeBuilder {
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
        node_id: impl Into<NodeId>,
        forecast_spec: crate::types::ForecastSpec,
    ) -> Self {
        let node_id = node_id.into();

        // Get or create the node (converting to Mixed type if needed)
        if let Some(node) = self.nodes.get_mut(node_id.as_str()) {
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

    /// Add all metrics from a loaded registry to the model.
    ///
    /// Common internal implementation used by [`with_builtin_metrics`] and [`with_metrics`].
    ///
    /// [`with_builtin_metrics`]: ModelBuilder::with_builtin_metrics
    /// [`with_metrics`]: ModelBuilder::with_metrics
    fn add_all_metrics_from_registry_internal(
        mut self,
        registry: &crate::registry::Registry,
    ) -> Result<Self> {
        for (qualified_id, stored_metric) in registry.all_metrics() {
            let namespace = qualified_id.split('.').next().unwrap_or("");
            let formula = if namespace.is_empty() {
                stored_metric.definition.formula.clone()
            } else {
                self.qualify_metric_references(
                    &stored_metric.definition.formula,
                    namespace,
                    registry,
                )?
            };
            self.insert_metric_node(qualified_id, stored_metric, formula);
        }
        Ok(self)
    }

    /// Load built-in metrics (fin.* namespace) and add them to the model.
    ///
    /// This is a convenience method that loads standard financial metrics
    /// and adds all of them to the model.
    ///
    /// For selective loading, prefer [`add_metric`] or [`add_metric_from_registry`].
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use finstack_statements::builder::ModelBuilder;
    /// # fn main() -> finstack_statements::Result<()> {
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
    ///
    /// [`add_metric`]: ModelBuilder::add_metric
    /// [`add_metric_from_registry`]: ModelBuilder::add_metric_from_registry
    #[must_use = "builder methods must be chained"]
    pub fn with_builtin_metrics(self) -> Result<Self> {
        let mut registry = crate::registry::Registry::new();
        registry.load_builtins()?;
        self.add_all_metrics_from_registry_internal(&registry)
    }

    /// Load metrics from a JSON file and add them to the model.
    ///
    /// For selective loading, prefer [`add_metric_from_registry`] after loading the file
    /// yourself via [`Registry::load_from_json`].
    ///
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
    ///
    /// [`add_metric_from_registry`]: ModelBuilder::add_metric_from_registry
    /// [`Registry::load_from_json`]: crate::registry::Registry::load_from_json
    #[must_use = "builder methods must be chained"]
    pub fn with_metrics(self, path: &str) -> Result<Self> {
        let mut registry = crate::registry::Registry::new();
        registry.load_from_json(path)?;
        self.add_all_metrics_from_registry_internal(&registry)
    }

    /// Add a specific metric from the built-in registry.
    ///
    /// This is a convenience method that loads the built-in metrics registry
    /// and adds a specific metric to the model. For adding multiple metrics,
    /// prefer loading the registry once and calling [`add_metric_from_registry`]
    /// for each metric to avoid repeated I/O.
    ///
    /// [`add_metric_from_registry`]: ModelBuilder::add_metric_from_registry
    ///
    /// # Arguments
    /// * `qualified_id` - Fully qualified metric identifier (e.g., `"fin.gross_margin"`)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use finstack_statements::builder::ModelBuilder;
    /// # use finstack_statements::registry::Registry;
    /// # fn main() -> finstack_statements::Result<()> {
    /// // Preferred: load registry once for multiple metrics
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
    /// ```rust,no_run
    /// # use finstack_statements::builder::ModelBuilder;
    /// # use finstack_statements::registry::Registry;
    /// # fn main() -> finstack_statements::Result<()> {
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
            if !self.nodes.contains_key(dep_id.as_str()) {
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
    pub fn build(mut self) -> Result<FinancialModelSpec> {
        // Validate that we have at least one period
        if self.periods.is_empty() {
            return Err(Error::build("Model must have at least one period"));
        }

        // Validate all node IDs don't use reserved prefixes
        for node_id in self.nodes.keys() {
            validate_node_id(node_id.as_str())?;
        }

        // Validate node type / field consistency
        for (node_id, node) in &self.nodes {
            match node.node_type {
                NodeType::Value => {
                    if node.formula_text.is_some() {
                        return Err(Error::build(format!(
                            "Value node '{}' cannot have a formula — use Mixed or Calculated type",
                            node_id
                        )));
                    }
                }
                NodeType::Calculated => {
                    if node.values.is_some() {
                        return Err(Error::build(format!(
                            "Calculated node '{}' cannot have explicit values — use Mixed or Value type",
                            node_id
                        )));
                    }
                }
                NodeType::Mixed => {}
            }
        }

        // Validate where clauses at build time (catches syntax errors early)
        for (node_id, node) in &self.nodes {
            if let Some(where_text) = &node.where_text {
                crate::dsl::parse_and_compile(where_text).map_err(|e| {
                    Error::build(format!("Invalid where clause on node '{}': {}", node_id, e))
                })?;
            }
        }

        for node in self.nodes.values_mut() {
            if let Some(values) = &node.values {
                let inferred = crate::types::infer_series_value_type(values.values())?;
                if node.value_type.is_none() {
                    node.value_type = inferred;
                }
            }
        }

        if let Some(alias_registry) = &self.alias_registry {
            let available_nodes: IndexSet<String> = self
                .nodes
                .keys()
                .map(|id| id.as_str().to_string())
                .collect();
            for node in self.nodes.values_mut() {
                if let Some(formula) = node.formula_text.as_mut() {
                    *formula =
                        normalize_formula_aliases(formula, alias_registry, &available_nodes)?;
                }
                if let Some(where_text) = node.where_text.as_mut() {
                    *where_text =
                        normalize_formula_aliases(where_text, alias_registry, &available_nodes)?;
                }
            }
        }

        // Create the model spec
        let mut spec = FinancialModelSpec::new(self.id, self.periods);
        spec.nodes = self.nodes;
        spec.meta = self.meta;
        spec.capital_structure = self.capital_structure;

        // Detect circular dependencies at build time.
        // Graph construction may fail if formula references are not
        // yet defined (partial models) — that is allowed, but if it
        // succeeds we *must* verify no cycles exist.
        match crate::evaluator::DependencyGraph::from_model(&spec) {
            Ok(graph) => graph.detect_cycles()?,
            Err(e) => {
                tracing::debug!(
                    model_id = %spec.id,
                    error = %e,
                    "Skipping cycle detection: dependency graph could not be built"
                );
            }
        }

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
    node_id: NodeId,
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
    ///     .build();
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
    ///     .build();
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
    ///     .build();
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
    ///     .build();
    /// # Ok(())
    /// # }
    /// ```
    #[must_use = "builder methods must be chained"]
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Build the mixed node and return to the parent model builder.
    ///
    /// Note: Node ID validation happens in `ModelBuilder::build()`. If you need
    /// early validation, use `try_build()` instead.
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
    ///         .build()
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    #[must_use = "builder methods must be chained"]
    pub fn build(mut self) -> ModelBuilder<Ready> {
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

    /// Build the mixed node with validation.
    ///
    /// This is like `build()` but validates the node ID eagerly, returning a `Result`.
    ///
    /// # Errors
    ///
    /// Returns an error if the node ID uses a reserved prefix (`__cs__` or `__`).
    pub fn try_build(self) -> Result<ModelBuilder<Ready>> {
        validate_node_id(self.node_id.as_str())?;
        Ok(self.build())
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
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::evaluator::Evaluator;

    #[test]
    fn test_name_normalization_rewrites_formula_aliases() {
        let period = PeriodId::quarter(2025, 1);
        let model = ModelBuilder::new("alias-test")
            .periods("2025Q1..Q1", None)
            .expect("valid periods")
            .with_name_normalization()
            .value("revenue", &[(period, AmountOrScalar::scalar(100_000.0))])
            .value("cogs", &[(period, AmountOrScalar::scalar(40_000.0))])
            .compute("gross_profit", "rev - cogs")
            .expect("valid formula")
            .build()
            .expect("valid model");

        let mut evaluator = Evaluator::new();
        let results = evaluator
            .evaluate(&model)
            .expect("evaluation should succeed");
        assert_eq!(results.get("gross_profit", &period), Some(60_000.0));
    }
}
