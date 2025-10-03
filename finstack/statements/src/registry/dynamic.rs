//! Dynamic metric registry implementation.

use crate::dsl::{compile, parse_formula};
use crate::error::{Error, Result};
use crate::registry::schema::{MetricDefinition, MetricRegistry};
use crate::registry::validation::validate_metric_definition;
use finstack_core::expr::Expr;
use indexmap::IndexMap;
use std::collections::HashSet;

/// Dynamic registry for metric definitions.
///
/// The registry stores metrics organized by namespace and provides
/// lookup, validation, and compilation services.
#[derive(Debug, Clone, Default)]
pub struct Registry {
    /// Map of fully-qualified metric ID → metric definition
    metrics: IndexMap<String, StoredMetric>,

    /// Set of all namespaces
    namespaces: HashSet<String>,
}

/// Stored metric with compiled expression.
#[derive(Debug, Clone)]
pub struct StoredMetric {
    /// Namespace
    pub namespace: String,

    /// Metric definition
    pub definition: MetricDefinition,

    /// Compiled expression (cached)
    pub compiled: Expr,
}

impl Registry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            metrics: IndexMap::new(),
            namespaces: HashSet::new(),
        }
    }

    /// Load built-in metrics (fin.* namespace).
    ///
    /// This loads standard financial metrics from embedded JSON files.
    ///
    /// # Example
    ///
    /// ```rust
    /// use finstack_statements::registry::Registry;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut registry = Registry::new();
    /// registry.load_builtins()?;
    ///
    /// assert!(registry.has("fin.gross_profit"));
    /// assert!(registry.has("fin.gross_margin"));
    /// # Ok(())
    /// # }
    /// ```
    pub fn load_builtins(&mut self) -> Result<()> {
        // Load from embedded JSON files
        self.load_from_json_str(include_str!("../../data/metrics/fin_basic.json"))?;
        self.load_from_json_str(include_str!("../../data/metrics/fin_margins.json"))?;
        self.load_from_json_str(include_str!("../../data/metrics/fin_returns.json"))?;
        self.load_from_json_str(include_str!("../../data/metrics/fin_leverage.json"))?;
        Ok(())
    }

    /// Load metrics from a JSON file path.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use finstack_statements::registry::Registry;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut registry = Registry::new();
    /// registry.load_from_json("metrics/custom.json")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn load_from_json(&mut self, path: &str) -> Result<()> {
        let json = std::fs::read_to_string(path)?;
        self.load_from_json_str(&json)?;
        Ok(())
    }

    /// Load metrics from a JSON string.
    pub fn load_from_json_str(&mut self, json: &str) -> Result<MetricRegistry> {
        let registry: MetricRegistry = serde_json::from_str(json)?;
        self.load_registry(registry.clone())?;
        Ok(registry)
    }

    /// Load a metric registry.
    ///
    /// Validates all metrics and checks for collisions.
    pub fn load_registry(&mut self, registry: MetricRegistry) -> Result<()> {
        let namespace = registry.namespace.clone();

        // Validate namespace
        if namespace.is_empty() {
            return Err(Error::registry("Namespace cannot be empty"));
        }

        // Track namespace
        self.namespaces.insert(namespace.clone());

        // Load each metric
        for metric in registry.metrics {
            // Validate metric
            validate_metric_definition(&metric, &namespace)?;

            // Check for collisions
            let qualified_id = metric.qualified_id(&namespace);
            if self.metrics.contains_key(&qualified_id) {
                return Err(Error::registry(format!(
                    "Duplicate metric ID: '{}'",
                    qualified_id
                )));
            }

            // Parse and compile formula
            let ast = parse_formula(&metric.formula)?;
            let compiled = compile(&ast)?;

            // Store metric
            self.metrics.insert(
                qualified_id,
                StoredMetric {
                    namespace: namespace.clone(),
                    definition: metric,
                    compiled,
                },
            );
        }

        Ok(())
    }

    /// Get a metric by fully-qualified ID.
    ///
    /// # Example
    ///
    /// ```rust
    /// use finstack_statements::registry::Registry;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut registry = Registry::new();
    /// registry.load_builtins()?;
    ///
    /// let metric = registry.get("fin.gross_margin")?;
    /// assert_eq!(metric.definition.formula, "(revenue - cogs) / revenue");
    /// # Ok(())
    /// # }
    /// ```
    pub fn get(&self, qualified_id: &str) -> Result<&StoredMetric> {
        self.metrics
            .get(qualified_id)
            .ok_or_else(|| Error::registry(format!("Metric not found: '{}'", qualified_id)))
    }

    /// Check if a metric exists.
    pub fn has(&self, qualified_id: &str) -> bool {
        self.metrics.contains_key(qualified_id)
    }

    /// List all metrics in a namespace.
    ///
    /// Returns an iterator over (qualified_id, metric).
    ///
    /// # Example
    ///
    /// ```rust
    /// use finstack_statements::registry::Registry;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut registry = Registry::new();
    /// registry.load_builtins()?;
    ///
    /// let fin_metrics: Vec<_> = registry.namespace("fin").collect();
    /// assert!(fin_metrics.len() > 0);
    /// # Ok(())
    /// # }
    /// ```
    pub fn namespace<'a>(
        &'a self,
        namespace: &'a str,
    ) -> impl Iterator<Item = (&'a str, &'a StoredMetric)> + 'a {
        self.metrics
            .iter()
            .filter(move |(_id, m)| m.namespace == namespace)
            .map(|(id, m)| (id.as_str(), m))
    }

    /// List all namespaces.
    pub fn namespaces(&self) -> Vec<&str> {
        let mut namespaces: Vec<_> = self.namespaces.iter().map(|s| s.as_str()).collect();
        namespaces.sort();
        namespaces
    }

    /// List all metrics.
    pub fn all_metrics(&self) -> impl Iterator<Item = (&str, &StoredMetric)> {
        self.metrics.iter().map(|(id, m)| (id.as_str(), m))
    }

    /// Get the number of metrics.
    pub fn len(&self) -> usize {
        self.metrics.len()
    }

    /// Check if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.metrics.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_from_json_str() {
        let json = r#"{
            "namespace": "test",
            "schema_version": 1,
            "metrics": [
                {
                    "id": "gross_margin",
                    "name": "Gross Margin",
                    "formula": "gross_profit / revenue"
                }
            ]
        }"#;

        let mut registry = Registry::new();
        registry.load_from_json_str(json).unwrap();

        assert!(registry.has("test.gross_margin"));
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn test_get_metric() {
        let json = r#"{
            "namespace": "test",
            "metrics": [
                {
                    "id": "gross_margin",
                    "name": "Gross Margin",
                    "formula": "gross_profit / revenue"
                }
            ]
        }"#;

        let mut registry = Registry::new();
        registry.load_from_json_str(json).unwrap();

        let metric = registry.get("test.gross_margin").unwrap();
        assert_eq!(metric.definition.formula, "gross_profit / revenue");
    }

    #[test]
    fn test_namespace_listing() {
        let json = r#"{
            "namespace": "test",
            "metrics": [
                {
                    "id": "metric1",
                    "name": "Metric 1",
                    "formula": "a + b"
                },
                {
                    "id": "metric2",
                    "name": "Metric 2",
                    "formula": "c - d"
                }
            ]
        }"#;

        let mut registry = Registry::new();
        registry.load_from_json_str(json).unwrap();

        let test_metrics: Vec<_> = registry.namespace("test").collect();
        assert_eq!(test_metrics.len(), 2);
    }

    #[test]
    fn test_duplicate_metric_error() {
        let json = r#"{
            "namespace": "test",
            "metrics": [
                {
                    "id": "gross_margin",
                    "name": "Gross Margin",
                    "formula": "gross_profit / revenue"
                }
            ]
        }"#;

        let mut registry = Registry::new();
        registry.load_from_json_str(json).unwrap();

        // Try to load again
        let result = registry.load_from_json_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_namespaces() {
        let json1 = r#"{
            "namespace": "test1",
            "metrics": [
                {"id": "m1", "name": "M1", "formula": "a + b"}
            ]
        }"#;

        let json2 = r#"{
            "namespace": "test2",
            "metrics": [
                {"id": "m2", "name": "M2", "formula": "c - d"}
            ]
        }"#;

        let mut registry = Registry::new();
        registry.load_from_json_str(json1).unwrap();
        registry.load_from_json_str(json2).unwrap();

        let namespaces = registry.namespaces();
        assert_eq!(namespaces.len(), 2);
        assert!(namespaces.contains(&"test1"));
        assert!(namespaces.contains(&"test2"));
    }
}
