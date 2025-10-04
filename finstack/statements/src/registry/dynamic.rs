//! Dynamic metric registry implementation.

use crate::dsl::{compile, parse_formula};
use crate::error::{Error, Result};
use crate::registry::schema::{MetricDefinition, MetricRegistry};
use crate::registry::validation::validate_metric_definition;
use finstack_core::expr::Expr;
use indexmap::{IndexMap, IndexSet};
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
    /// Supports inter-metric dependencies - metrics can reference other metrics in the same registry.
    pub fn load_registry(&mut self, registry: MetricRegistry) -> Result<()> {
        let namespace = registry.namespace.clone();

        // Validate namespace
        if namespace.is_empty() {
            return Err(Error::registry(
                "Namespace cannot be empty. Provide a namespace identifier (e.g., 'fin', 'custom')."
            ));
        }

        // Track namespace
        self.namespaces.insert(namespace.clone());

        // Sort metrics by dependency order
        let sorted_metrics = self.sort_metrics_by_dependencies(&registry)?;

        // Load each metric in dependency order
        for metric in sorted_metrics {
            // Validate metric
            validate_metric_definition(&metric, &namespace)?;

            // Check for collisions
            let qualified_id = metric.qualified_id(&namespace);
            if self.metrics.contains_key(&qualified_id) {
                return Err(Error::registry(format!(
                    "Duplicate metric ID: '{}'. This metric is already registered in the registry.",
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
    /// assert_eq!(metric.definition.formula, "gross_profit / revenue");
    /// # Ok(())
    /// # }
    /// ```
    pub fn get(&self, qualified_id: &str) -> Result<&StoredMetric> {
        self.metrics.get(qualified_id).ok_or_else(|| {
            let available: Vec<_> = self.metrics.keys().take(5).map(|s| s.as_str()).collect();
            Error::registry(format!(
                "Metric not found: '{}'. Available metrics include: {}{}",
                qualified_id,
                available.join(", "),
                if self.metrics.len() > 5 { ", ..." } else { "" }
            ))
        })
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

    /// Sort metrics by dependency order using topological sort.
    ///
    /// This allows metrics to reference other metrics in the same registry.
    fn sort_metrics_by_dependencies(
        &self,
        registry: &MetricRegistry,
    ) -> Result<Vec<MetricDefinition>> {
        let namespace = &registry.namespace;

        // Build map of metric_id -> MetricDefinition for lookup
        let metric_map: IndexMap<String, MetricDefinition> = registry
            .metrics
            .iter()
            .map(|m| (m.id.clone(), m.clone()))
            .collect();

        // Build dependency graph: metric_id -> set of metrics it depends on
        let mut dependencies: IndexMap<String, IndexSet<String>> = IndexMap::new();
        let mut all_metric_ids: IndexSet<String> = metric_map.keys().cloned().collect();

        // Also include already-loaded metrics from the same namespace
        for (qualified_id, stored) in &self.metrics {
            if stored.namespace == *namespace {
                // Extract just the metric ID (without namespace prefix)
                if let Some(id) = qualified_id.strip_prefix(&format!("{}.", namespace)) {
                    all_metric_ids.insert(id.to_string());
                }
            }
        }

        for (metric_id, metric) in &metric_map {
            let deps = self.extract_metric_dependencies(&metric.formula, &all_metric_ids);
            dependencies.insert(metric_id.clone(), deps);
        }

        // Topological sort using Kahn's algorithm
        let mut sorted = Vec::new();
        let mut in_degree: IndexMap<String, usize> = IndexMap::new();

        // Calculate in-degrees
        for metric_id in metric_map.keys() {
            in_degree.insert(metric_id.clone(), 0);
        }
        for deps in dependencies.values() {
            for dep in deps {
                if metric_map.contains_key(dep) {
                    *in_degree.entry(dep.clone()).or_insert(0) += 1;
                }
            }
        }

        // Queue of metrics with no dependencies
        let mut queue: Vec<String> = in_degree
            .iter()
            .filter(|(_, &degree)| degree == 0)
            .map(|(id, _)| id.clone())
            .collect();

        while let Some(metric_id) = queue.pop() {
            sorted.push(metric_map[&metric_id].clone());

            // Reduce in-degree for dependents
            if let Some(deps) = dependencies.get(&metric_id) {
                for dep in deps {
                    if let Some(degree) = in_degree.get_mut(dep) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push(dep.clone());
                        }
                    }
                }
            }
        }

        // Check for circular dependencies
        if sorted.len() != metric_map.len() {
            // Find the cycle
            let remaining: Vec<String> = metric_map
                .keys()
                .filter(|id| !sorted.iter().any(|m| &m.id == *id))
                .cloned()
                .collect();

            return Err(Error::registry(format!(
                "Circular dependency detected among metrics: {}",
                remaining.join(" -> ")
            )));
        }

        Ok(sorted)
    }

    /// Extract dependencies from a metric formula.
    ///
    /// Returns the set of metric IDs (unqualified) that this formula references.
    fn extract_metric_dependencies(
        &self,
        formula: &str,
        all_metric_ids: &IndexSet<String>,
    ) -> IndexSet<String> {
        let mut deps = IndexSet::new();

        // Check if any metric ID appears as a standalone identifier in the formula
        for metric_id in all_metric_ids {
            if formula.contains(metric_id.as_str()) {
                // Verify it's a standalone identifier
                let is_standalone = formula.match_indices(metric_id.as_str()).any(|(idx, _)| {
                    let before = if idx > 0 {
                        formula.chars().nth(idx - 1)
                    } else {
                        None
                    };
                    let after = formula.chars().nth(idx + metric_id.len());

                    let before_ok =
                        before.map_or(true, |c| !c.is_alphanumeric() && c != '_' && c != '.');
                    let after_ok =
                        after.map_or(true, |c| !c.is_alphanumeric() && c != '_' && c != '.');

                    before_ok && after_ok
                });

                if is_standalone {
                    deps.insert(metric_id.clone());
                }
            }
        }

        deps
    }

    /// Get dependencies for a specific metric (including transitive dependencies).
    ///
    /// Returns the ordered list of metric IDs (qualified) that must be added before this metric.
    pub fn get_metric_dependencies(&self, qualified_id: &str) -> Result<Vec<String>> {
        // Recursively get transitive dependencies
        let mut all_deps = IndexSet::new();
        let mut visited = IndexSet::new();
        self.collect_transitive_dependencies(qualified_id, &mut all_deps, &mut visited)?;

        // Return in dependency order (dependencies before dependents)
        Ok(all_deps.into_iter().collect())
    }

    /// Recursively collect transitive dependencies.
    fn collect_transitive_dependencies(
        &self,
        qualified_id: &str,
        all_deps: &mut IndexSet<String>,
        visited: &mut IndexSet<String>,
    ) -> Result<()> {
        // Avoid infinite loops
        if visited.contains(qualified_id) {
            return Ok(());
        }
        visited.insert(qualified_id.to_string());

        let metric = self.get(qualified_id)?;
        let namespace = &metric.namespace;

        // Get all metric IDs in this namespace
        let all_metric_ids: IndexSet<String> = self
            .namespace(namespace)
            .map(|(id, _)| {
                id.strip_prefix(&format!("{}.", namespace))
                    .unwrap_or(id)
                    .to_string()
            })
            .collect();

        // Extract direct dependencies
        let deps = self.extract_metric_dependencies(&metric.definition.formula, &all_metric_ids);

        // Recursively process each dependency
        for dep_id in deps {
            let dep_qualified = format!("{}.{}", namespace, dep_id);
            if self.has(&dep_qualified) {
                self.collect_transitive_dependencies(&dep_qualified, all_deps, visited)?;
                all_deps.insert(dep_qualified);
            }
        }

        Ok(())
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

    #[test]
    fn test_inter_metric_dependencies() {
        // Test metrics that reference other metrics
        let json = r#"{
            "namespace": "test",
            "metrics": [
                {
                    "id": "gross_profit",
                    "name": "Gross Profit",
                    "formula": "revenue - cogs"
                },
                {
                    "id": "gross_margin",
                    "name": "Gross Margin",
                    "formula": "gross_profit / revenue"
                }
            ]
        }"#;

        let mut registry = Registry::new();
        registry.load_from_json_str(json).unwrap();

        // Both metrics should be loaded
        assert!(registry.has("test.gross_profit"));
        assert!(registry.has("test.gross_margin"));

        // gross_margin should reference gross_profit in its formula
        let margin = registry.get("test.gross_margin").unwrap();
        assert!(margin.definition.formula.contains("gross_profit"));
    }

    #[test]
    fn test_metric_dependency_order() {
        // Test that metrics are loaded in dependency order
        let json = r#"{
            "namespace": "test",
            "metrics": [
                {
                    "id": "net_margin",
                    "name": "Net Margin",
                    "formula": "net_income / revenue"
                },
                {
                    "id": "net_income",
                    "name": "Net Income",
                    "formula": "gross_profit - opex"
                },
                {
                    "id": "gross_profit",
                    "name": "Gross Profit",
                    "formula": "revenue - cogs"
                }
            ]
        }"#;

        let mut registry = Registry::new();
        let result = registry.load_from_json_str(json);

        // Should succeed even though metrics are in reverse dependency order
        assert!(result.is_ok());
        assert!(registry.has("test.gross_profit"));
        assert!(registry.has("test.net_income"));
        assert!(registry.has("test.net_margin"));
    }

    #[test]
    fn test_circular_dependency_detection() {
        // Test that circular dependencies are detected
        let json = r#"{
            "namespace": "test",
            "metrics": [
                {
                    "id": "metric_a",
                    "name": "Metric A",
                    "formula": "metric_b + 1"
                },
                {
                    "id": "metric_b",
                    "name": "Metric B",
                    "formula": "metric_a + 1"
                }
            ]
        }"#;

        let mut registry = Registry::new();
        let result = registry.load_from_json_str(json);

        // Should fail with circular dependency error
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Circular dependency"));
    }

    #[test]
    fn test_get_metric_dependencies() {
        let json = r#"{
            "namespace": "test",
            "metrics": [
                {
                    "id": "a",
                    "name": "A",
                    "formula": "x + y"
                },
                {
                    "id": "b",
                    "name": "B",
                    "formula": "a * 2"
                },
                {
                    "id": "c",
                    "name": "C",
                    "formula": "b + a"
                }
            ]
        }"#;

        let mut registry = Registry::new();
        registry.load_from_json_str(json).unwrap();

        // Get dependencies for "c" - should include both "a" and "b"
        let deps = registry.get_metric_dependencies("test.c").unwrap();
        assert_eq!(deps.len(), 2);
        assert!(deps.contains(&"test.a".to_string()));
        assert!(deps.contains(&"test.b".to_string()));

        // "a" should appear before "b" in the list (dependency order)
        let a_pos = deps.iter().position(|d| d == "test.a").unwrap();
        let b_pos = deps.iter().position(|d| d == "test.b").unwrap();
        assert!(a_pos < b_pos);
    }

    #[test]
    fn test_transitive_dependencies() {
        // Test that transitive dependencies are handled correctly
        let json = r#"{
            "namespace": "test",
            "metrics": [
                {
                    "id": "level1",
                    "name": "Level 1",
                    "formula": "base_value * 2"
                },
                {
                    "id": "level2",
                    "name": "Level 2",
                    "formula": "level1 + 10"
                },
                {
                    "id": "level3",
                    "name": "Level 3",
                    "formula": "level2 / 2"
                }
            ]
        }"#;

        let mut registry = Registry::new();
        registry.load_from_json_str(json).unwrap();

        // Get dependencies for level3 - should include level1 and level2
        let deps = registry.get_metric_dependencies("test.level3").unwrap();
        assert_eq!(deps.len(), 2);
        assert!(deps.contains(&"test.level1".to_string()));
        assert!(deps.contains(&"test.level2".to_string()));
    }

    #[test]
    fn test_mixed_dependencies() {
        // Test metrics that reference both base nodes and other metrics
        let json = r#"{
            "namespace": "test",
            "metrics": [
                {
                    "id": "gross_profit",
                    "name": "Gross Profit",
                    "formula": "revenue - cogs"
                },
                {
                    "id": "ebitda",
                    "name": "EBITDA",
                    "formula": "gross_profit - opex"
                },
                {
                    "id": "ebitda_margin",
                    "name": "EBITDA Margin",
                    "formula": "ebitda / revenue"
                }
            ]
        }"#;

        let mut registry = Registry::new();
        registry.load_from_json_str(json).unwrap();

        // All metrics should load successfully
        assert!(registry.has("test.gross_profit"));
        assert!(registry.has("test.ebitda"));
        assert!(registry.has("test.ebitda_margin"));

        // ebitda_margin depends on both ebitda (metric) and revenue (base node)
        let deps = registry
            .get_metric_dependencies("test.ebitda_margin")
            .unwrap();
        assert_eq!(deps.len(), 2); // gross_profit and ebitda
        assert!(deps.contains(&"test.gross_profit".to_string()));
        assert!(deps.contains(&"test.ebitda".to_string()));
    }
}
