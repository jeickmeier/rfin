//! Type-state builder pattern for financial models.

use crate::error::{Error, Result};
use crate::types::{AmountOrScalar, FinancialModelSpec, NodeSpec, NodeType};
use finstack_core::dates::{build_periods, Period, PeriodId};
use indexmap::IndexMap;
use std::marker::PhantomData;

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
    periods: Vec<Period>,
    nodes: IndexMap<String, NodeSpec>,
    meta: IndexMap<String, serde_json::Value>,
    pub(crate) capital_structure: Option<crate::types::CapitalStructureSpec>,
    _state: PhantomData<State>,
}

impl ModelBuilder<NeedPeriods> {
    /// Create a new model builder.
    ///
    /// You must call `.periods()` before adding nodes.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            periods: Vec::new(),
            nodes: IndexMap::new(),
            meta: IndexMap::new(),
            capital_structure: None,
            _state: PhantomData,
        }
    }

    /// Define periods using a range expression.
    ///
    /// # Arguments
    ///
    /// * `range` - Period range (e.g., "2025Q1..Q4", "2025Q1..2026Q2")
    /// * `actuals_until` - Optional cutoff for actuals (e.g., Some("2025Q2"))
    ///
    /// # Example
    ///
    /// ```rust
    /// # use finstack_statements::builder::ModelBuilder;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let builder = ModelBuilder::new("test")
    ///     .periods("2025Q1..Q4", Some("2025Q2"))?;
    /// // Q1-Q2 are actuals, Q3-Q4 are forecast
    /// # Ok(())
    /// # }
    /// ```
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
            _state: PhantomData,
        })
    }

    /// Define periods explicitly (for advanced use cases).
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
            _state: PhantomData,
        })
    }
}

impl ModelBuilder<Ready> {
    /// Add a value node with explicit period values.
    ///
    /// Value nodes contain only explicit data (actuals or assumptions).
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

    /// Add a calculated node with a formula.
    ///
    /// Calculated nodes derive their values from formulas only.
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

        // Basic validation: formula should not be empty
        if formula.trim().is_empty() {
            return Err(Error::formula_parse("Formula cannot be empty"));
        }

        let node = NodeSpec::new(node_id.clone(), NodeType::Calculated).with_formula(formula);

        self.nodes.insert(node_id, node);
        Ok(self)
    }

    /// Add a forecast specification to an existing node.
    ///
    /// This allows forecasting values into future periods using various methods.
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
            // Add forecast to existing node
            node.forecasts.push(forecast_spec);

            // Ensure node type is Mixed if it has forecasts
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
    #[must_use = "builder methods must be chained"]
    pub fn with_meta(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.meta.insert(key.into(), value);
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
    pub fn with_builtin_metrics(mut self) -> Result<Self> {
        let mut registry = crate::registry::Registry::new();
        registry.load_builtins()?;

        // Add all metrics from the registry as calculated nodes
        for (qualified_id, stored_metric) in registry.all_metrics() {
            let node = NodeSpec::new(qualified_id.to_string(), NodeType::Calculated)
                .with_name(stored_metric.definition.name.clone())
                .with_formula(stored_metric.definition.formula.clone());
            self.nodes.insert(qualified_id.to_string(), node);
        }

        Ok(self)
    }

    /// Load metrics from a JSON file and add them to the model.
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
    pub fn with_metrics(mut self, path: &str) -> Result<Self> {
        let mut registry = crate::registry::Registry::new();
        registry.load_from_json(path)?;

        // Add all metrics from the registry as calculated nodes
        for (qualified_id, stored_metric) in registry.all_metrics() {
            let node = NodeSpec::new(qualified_id.to_string(), NodeType::Calculated)
                .with_name(stored_metric.definition.name.clone())
                .with_formula(stored_metric.definition.formula.clone());
            self.nodes.insert(qualified_id.to_string(), node);
        }

        Ok(self)
    }

    /// Add a specific metric from a registry.
    ///
    /// This allows selectively adding metrics from a registry instead of
    /// adding all of them.
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
            .ok_or_else(|| Error::registry("Invalid qualified ID"))?;

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
                
                let dep_node = NodeSpec::new(dep_id.clone(), NodeType::Calculated)
                    .with_name(dep_metric.definition.name.clone())
                    .with_formula(formula);
                self.nodes.insert(dep_id, dep_node);
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
            
            let node = NodeSpec::new(qualified_id.to_string(), NodeType::Calculated)
                .with_name(stored_metric.definition.name.clone())
                .with_formula(formula);
            self.nodes.insert(qualified_id.to_string(), node);
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
        let mut result = formula.to_string();

        // Get all metrics in this namespace
        let metrics_in_namespace: Vec<String> = registry
            .namespace(namespace)
            .map(|(id, _)| {
                // Extract unqualified ID
                id.strip_prefix(&format!("{}.", namespace))
                    .unwrap_or(id)
                    .to_string()
            })
            .collect();

        // Sort by length descending to replace longer IDs first
        // This prevents "ebitda_margin" from being partially replaced as "ebitda"
        let mut sorted_metrics = metrics_in_namespace;
        sorted_metrics.sort_by_key(|b| std::cmp::Reverse(b.len()));

        // Replace each unqualified metric reference with qualified one
        for metric_id in sorted_metrics {
            let qualified = format!("{}.{}", namespace, metric_id);
            
            // Only replace if it's a standalone identifier
            let mut idx = 0;
            while let Some(pos) = result[idx..].find(&metric_id) {
                let abs_pos = idx + pos;
                
                // Check if it's a standalone identifier
                let before_ok = if abs_pos > 0 {
                    let before_char = result.chars().nth(abs_pos - 1);
                    before_char.map_or(true, |c| !c.is_alphanumeric() && c != '_' && c != '.')
                } else {
                    true
                };
                
                let after_pos = abs_pos + metric_id.len();
                let after_ok = if after_pos < result.len() {
                    let after_char = result.chars().nth(after_pos);
                    after_char.map_or(true, |c| !c.is_alphanumeric() && c != '_' && c != '.')
                } else {
                    true
                };
                
                if before_ok && after_ok {
                    // Replace this occurrence
                    result.replace_range(abs_pos..after_pos, &qualified);
                    idx = abs_pos + qualified.len();
                } else {
                    idx = after_pos;
                }
            }
        }

        Ok(result)
    }

    /// Build the final model specification.
    ///
    /// This validates the model and returns a `FinancialModelSpec`.
    pub fn build(self) -> Result<FinancialModelSpec> {
        // Validate that we have at least one period
        if self.periods.is_empty() {
            return Err(Error::build("Model must have at least one period"));
        }

        // Create the model spec
        let mut spec = FinancialModelSpec::new(self.id, self.periods);
        spec.nodes = self.nodes;
        spec.meta = self.meta;
        spec.capital_structure = self.capital_structure;

        Ok(spec)
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
            .unwrap()
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
            .unwrap()
            .value(
                "revenue",
                &[
                    (PeriodId::quarter(2025, 1), AmountOrScalar::scalar(100.0)),
                    (PeriodId::quarter(2025, 2), AmountOrScalar::scalar(110.0)),
                ],
            )
            .build()
            .unwrap();

        assert_eq!(model.nodes.len(), 1);
        assert!(model.has_node("revenue"));
        assert_eq!(
            model.get_node("revenue").unwrap().node_type,
            NodeType::Value
        );
    }

    #[test]
    fn test_computed_node() {
        let model = ModelBuilder::new("test")
            .periods("2025Q1..Q2", None)
            .unwrap()
            .compute("gross_profit", "revenue - cogs")
            .unwrap()
            .build()
            .unwrap();

        assert_eq!(model.nodes.len(), 1);
        let node = model.get_node("gross_profit").unwrap();
        assert_eq!(node.node_type, NodeType::Calculated);
        assert_eq!(node.formula_text.as_ref().unwrap(), "revenue - cogs");
    }

    #[test]
    fn test_empty_formula_error() {
        let result = ModelBuilder::new("test")
            .periods("2025Q1..Q2", None)
            .unwrap()
            .compute("invalid", "");

        assert!(result.is_err());
    }

    #[test]
    fn test_multiple_nodes() {
        let model = ModelBuilder::new("test")
            .periods("2025Q1..Q4", Some("2025Q2"))
            .unwrap()
            .value(
                "revenue",
                &[
                    (PeriodId::quarter(2025, 1), AmountOrScalar::scalar(100.0)),
                    (PeriodId::quarter(2025, 2), AmountOrScalar::scalar(110.0)),
                ],
            )
            .compute("cogs", "revenue * 0.6")
            .unwrap()
            .compute("gross_profit", "revenue - cogs")
            .unwrap()
            .build()
            .unwrap();

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
