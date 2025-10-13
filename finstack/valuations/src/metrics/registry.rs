//! Metric registry and computation engine.
//!
//! Manages metric calculators with dependency resolution, caching, and batch
//! computation. The registry handles which metrics apply to which instrument
//! types and ensures efficient computation ordering.

use super::ids::MetricId;
use super::traits::{MetricCalculator, MetricContext};

use hashbrown::HashMap;
use std::sync::Arc;

/// Registry for metric calculators.
///
/// Manages metric calculators with dependency resolution, caching, and batch
/// computation. Also handles which metrics apply to which instrument types.
///
/// # Key Features
///
/// - **Calculator management**: Register and retrieve metric calculators
/// - **Dependency resolution**: Automatic computation ordering based on dependencies
/// - **Instrument applicability**: Metrics can be restricted to specific instrument types
/// - **Batch computation**: Compute multiple metrics efficiently
///
/// # Example
/// ```rust
/// use finstack_valuations::metrics::registry::MetricRegistry;
/// use finstack_valuations::metrics::ids::MetricId;
/// use finstack_valuations::metrics::traits::MetricCalculator;
/// use std::sync::Arc;
///
/// struct MyCalculator;
/// impl MetricCalculator for MyCalculator {
///     fn calculate(&self, _context: &mut finstack_valuations::metrics::traits::MetricContext) -> finstack_core::Result<f64> {
///         Ok(42.0)
///     }
/// }
///
/// let mut registry = MetricRegistry::new();
/// registry.register_metric(
///     MetricId::Ytm,
///     Arc::new(MyCalculator),
///     &["Bond"]
/// );
///
/// assert!(registry.has_metric(MetricId::Ytm));
/// ```
#[derive(Clone)]
pub struct MetricRegistry {
    entries: HashMap<MetricId, MetricEntry>,
}

#[derive(Clone, Default)]
struct MetricEntry {
    default: Option<Arc<dyn MetricCalculator>>,
    per_instrument: HashMap<&'static str, Arc<dyn MetricCalculator>>,
}

impl MetricEntry {
    fn get_for(&self, instrument_type: &str) -> Option<&Arc<dyn MetricCalculator>> {
        self.per_instrument
            .get(instrument_type)
            .or(self.default.as_ref())
    }

    fn applies_to(&self, instrument_type: &str) -> bool {
        self.per_instrument.contains_key(instrument_type) || self.default.is_some()
    }
}

impl MetricRegistry {
    /// Creates a new empty registry.
    ///
    /// See unit tests and `examples/` for usage.
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }
}

impl Default for MetricRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl MetricRegistry {
    /// Registers a metric calculator with explicit ID and applicability.
    ///
    /// If a calculator with the same ID already exists, it will be replaced.
    /// The `applicable_to` parameter specifies which instrument types this metric
    /// applies to. An empty slice means it applies to all instruments.
    ///
    /// # Arguments
    /// * `id` - Unique identifier for the metric
    /// * `calculator` - Implementation of the metric calculation
    /// * `applicable_to` - Instrument types this metric applies to (empty = all)
    ///
    /// # Returns
    /// Mutable reference to self for method chaining
    ///
    /// See unit tests and `examples/` for usage.
    pub fn register_metric(
        &mut self,
        id: MetricId,
        calculator: Arc<dyn MetricCalculator>,
        applicable_to: &[&'static str],
    ) -> &mut Self {
        let entry = self.entries.entry(id).or_default();
        if applicable_to.is_empty() {
            entry.default = Some(calculator);
        } else {
            for instrument_type in applicable_to {
                entry
                    .per_instrument
                    .insert(*instrument_type, Arc::clone(&calculator));
            }
        }
        self
    }

    /// Checks if a metric is registered.
    ///
    /// # Arguments
    /// * `id` - Metric ID to check
    ///
    /// # Returns
    /// `true` if the metric is registered, `false` otherwise
    ///
    /// See unit tests and `examples/` for usage.
    pub fn has_metric(&self, id: MetricId) -> bool {
        self.entries.contains_key(&id)
    }

    /// Gets a list of all registered metric IDs.
    ///
    /// # Returns
    /// Vector of all registered metric IDs
    ///
    /// See unit tests and `examples/` for usage.
    pub fn available_metrics(&self) -> Vec<MetricId> {
        self.entries.keys().cloned().collect()
    }

    /// Gets metrics applicable to a specific instrument type.
    ///
    /// Returns all metrics that either apply to all instruments (empty applicability)
    /// or specifically apply to the given instrument type.
    ///
    /// # Arguments
    /// * `instrument_type` - Type of instrument (e.g., "Bond", "IRS")
    ///
    /// # Returns
    /// Vector of metric IDs applicable to the instrument type
    ///
    /// See unit tests and `examples/` for usage.
    pub fn metrics_for_instrument(&self, instrument_type: &str) -> Vec<MetricId> {
        self.entries
            .iter()
            .filter(|(_, entry)| entry.applies_to(instrument_type))
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Checks if a metric is applicable to a specific instrument type.
    ///
    /// A metric is applicable if it's registered and either applies to all
    /// instruments (empty applicability list) or specifically applies to
    /// the given instrument type.
    ///
    /// # Arguments
    /// * `metric_id` - Metric ID to check
    /// * `instrument_type` - Type of instrument (e.g., "Bond", "IRS")
    ///
    /// # Returns
    /// `true` if the metric is applicable, `false` otherwise
    ///
    /// See unit tests and `examples/` for usage.
    pub fn is_applicable(&self, metric_id: &MetricId, instrument_type: &str) -> bool {
        self.entries
            .get(metric_id)
            .map(|entry| entry.applies_to(instrument_type))
            .unwrap_or(false)
    }

    /// Computes specific metrics with dependency resolution.
    ///
    /// Handles dependency resolution, ordering, caching of intermediate results,
    /// and graceful error handling. Metrics are computed in the correct order based
    /// on their dependencies, and results are cached in the context.
    ///
    /// If a metric calculation fails or is not available for the instrument type,
    /// it will be assigned a value of 0.0 and computation will continue with other
    /// metrics. This ensures that one failing metric doesn't prevent calculation
    /// of other metrics.
    ///
    /// # Arguments
    /// * `metric_ids` - Vector of metric IDs to compute
    /// * `context` - Metric context containing instrument and market data
    ///
    /// # Returns
    /// HashMap mapping metric IDs to computed values. Failed metrics will have value 0.0.
    ///
    /// # Errors
    /// Returns an error only if dependency resolution fails due to circular dependencies.
    ///
    /// See unit tests and `examples/` for usage.
    pub fn compute(
        &self,
        metric_ids: &[MetricId],
        context: &mut MetricContext,
    ) -> finstack_core::Result<HashMap<MetricId, f64>> {
        // Build dependency graph and compute order for this instrument type
        let instrument_type = context.instrument.key().as_str();
        let order = self.resolve_dependencies(metric_ids, instrument_type)?;

        // Compute metrics in dependency order
        for metric_id in order {
            // Skip if already computed
            if context.computed.contains_key(&metric_id) {
                continue;
            }

            let Some(entry) = self.entries.get(&metric_id) else {
                // Metric not registered - insert 0.0 as fallback
                if metric_ids.contains(&metric_id) {
                    context.computed.insert(metric_id.clone(), 0.0);
                }
                continue;
            };

            let Some(calc) = entry.get_for(instrument_type) else {
                // Calculator not available for this instrument type - insert 0.0 as fallback
                if metric_ids.contains(&metric_id) {
                    context.computed.insert(metric_id.clone(), 0.0);
                }
                continue;
            };

            // Compute metric - if it fails, insert 0.0 as fallback and continue
            match calc.calculate(context) {
                Ok(value) => {
                    context.computed.insert(metric_id.clone(), value);
                }
                Err(_) => {
                    // Calculation failed - insert 0.0 as fallback
                    context.computed.insert(metric_id.clone(), 0.0);
                }
            }
        }

        // Return only the requested metrics
        let mut results = HashMap::new();
        for id in metric_ids {
            if let Some(&value) = context.computed.get(id) {
                results.insert(id.clone(), value);
            }
        }

        Ok(results)
    }

    /// Computes all registered metrics applicable to the instrument.
    ///
    /// This is a convenience method that finds all applicable metrics
    /// for the instrument type and computes them all at once. Useful
    /// for comprehensive analysis or when you want all available metrics.
    ///
    /// # Arguments
    /// * `context` - Metric context containing instrument and market data
    ///
    /// # Returns
    /// HashMap mapping all applicable metric IDs to computed values
    ///
    /// See unit tests and `examples/` for usage.
    pub fn compute_all(
        &self,
        context: &mut MetricContext,
    ) -> finstack_core::Result<HashMap<MetricId, f64>> {
        let instrument_type = context.instrument.key().as_str();
        let applicable = self.metrics_for_instrument(instrument_type);
        self.compute(&applicable, context)
    }

    /// Resolves dependencies and returns computation order.
    ///
    /// Uses topological sorting to ensure dependencies are computed first.
    /// This prevents circular dependencies and ensures efficient computation.
    ///
    /// Missing metrics or unavailable calculators are gracefully skipped without
    /// causing the entire resolution to fail.
    ///
    /// # Arguments
    /// * `metric_ids` - Vector of metric IDs to resolve dependencies for
    ///
    /// # Returns
    /// Vector of metric IDs in dependency order (dependencies first)
    ///
    /// # Errors
    /// Returns an error if circular dependencies are detected
    fn resolve_dependencies(
        &self,
        metric_ids: &[MetricId],
        instrument_type: &str,
    ) -> finstack_core::Result<Vec<MetricId>> {
        let mut visited = hashbrown::HashSet::new();
        let mut order = Vec::new();
        let mut temp_mark = hashbrown::HashSet::new();

        for id in metric_ids {
            // Ignore errors from missing metrics - they'll be handled in compute()
            let _ = self.visit_metric(
                id.clone(),
                instrument_type,
                &mut visited,
                &mut temp_mark,
                &mut order,
            );
        }

        Ok(order)
    }

    /// DFS visit for topological sort.
    fn visit_metric(
        &self,
        id: MetricId,
        instrument_type: &str,
        visited: &mut hashbrown::HashSet<MetricId>,
        temp_mark: &mut hashbrown::HashSet<MetricId>,
        order: &mut Vec<MetricId>,
    ) -> finstack_core::Result<()> {
        if visited.contains(&id) {
            return Ok(());
        }

        if temp_mark.contains(&id) {
            // Circular dependency detected - this is a real error
            return Err(finstack_core::Error::from(
                finstack_core::error::InputError::Invalid,
            ));
        }

        temp_mark.insert(id.clone());

        // Get calculator and process dependencies
        // If metric not found or not applicable, just skip it gracefully
        if let Some(entry) = self.entries.get(&id) {
            if let Some(calc) = entry.get_for(instrument_type) {
                let deps = calc.dependencies();
                for dep_id in deps {
                    // Ignore errors from missing dependencies
                    let _ = self.visit_metric(dep_id.clone(), instrument_type, visited, temp_mark, order);
                }
            }
        }

        temp_mark.remove(&id);
        visited.insert(id.clone());
        order.push(id);

        Ok(())
    }
}
