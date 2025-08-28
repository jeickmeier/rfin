#![deny(missing_docs)]
//! Metric registry and computation engine.

use super::traits::{MetricCalculator, MetricContext};
use super::ids::MetricId;
use finstack_core::F;
use hashbrown::HashMap;
use std::sync::Arc;

/// Registry for metric calculators.
/// 
/// The registry manages a collection of metric calculators and handles
/// dependency resolution, caching, and batch computation of metrics.
/// It also manages which metrics are applicable to which instrument types.
#[derive(Clone)]
pub struct MetricRegistry {
    calculators: HashMap<MetricId, Arc<dyn MetricCalculator>>,
    /// Maps instrument types to applicable metric IDs
    applicability: HashMap<String, hashbrown::HashSet<MetricId>>,
}

impl MetricRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            calculators: HashMap::new(),
            applicability: HashMap::new(),
        }
    }
    
    /// Register a metric calculator for specific instrument types.
    /// 
    /// If a calculator with the same ID already exists, it will be replaced.
    /// The `applicable_types` parameter specifies which instrument types
    /// this metric applies to. If empty, the metric applies to all instruments.
    pub fn register(&mut self, calculator: Arc<dyn MetricCalculator>) -> &mut Self {
        let metric_id = calculator.id();
        self.calculators.insert(metric_id, calculator);
        self
    }
    
    /// Register a metric calculator with explicit applicability.
    pub fn register_for_types(
        &mut self,
        calculator: Arc<dyn MetricCalculator>,
        applicable_types: &[&str],
    ) -> &mut Self {
        let metric_id = calculator.id();
        self.calculators.insert(metric_id.clone(), calculator);
        
        // Add to applicability map
        for instrument_type in applicable_types {
            self.applicability
                .entry(instrument_type.to_string())
                .or_insert_with(hashbrown::HashSet::new)
                .insert(metric_id.clone());
        }
        self
    }
    
    /// Register multiple calculators at once.
    pub fn register_many(&mut self, calculators: Vec<Arc<dyn MetricCalculator>>) -> &mut Self {
        for calc in calculators {
            self.register(calc);
        }
        self
    }
    
    /// Check if a metric is registered.
    pub fn has_metric(&self, id: MetricId) -> bool {
        self.calculators.contains_key(&id)
    }
    
    /// Get a list of all registered metric IDs.
    pub fn available_metrics(&self) -> Vec<MetricId> {
        self.calculators.keys().cloned().collect()
    }
    
    /// Get metrics applicable to a specific instrument type.
    pub fn metrics_for_instrument(&self, instrument_type: &str) -> Vec<MetricId> {
        if let Some(metrics) = self.applicability.get(instrument_type) {
            metrics.iter().cloned().collect()
        } else {
            // If no explicit applicability, return all metrics (for backward compatibility)
            self.calculators.keys().cloned().collect()
        }
    }
    
    /// Check if a metric is applicable to a specific instrument type.
    pub fn is_applicable(&self, metric_id: &MetricId, instrument_type: &str) -> bool {
        if let Some(metrics) = self.applicability.get(instrument_type) {
            metrics.contains(metric_id)
        } else {
            // If no explicit applicability mapping, assume all metrics are applicable
            self.calculators.contains_key(metric_id)
        }
    }
    
    /// Compute specific metrics with dependency resolution.
    /// 
    /// This method handles:
    /// - Dependency resolution and ordering
    /// - Caching of intermediate results
    /// - Error propagation
    /// 
    /// # Errors
    /// Returns an error if:
    /// - A requested metric is not registered
    /// - A metric has unregistered dependencies
    /// - Any metric calculation fails
    pub fn compute(
        &self,
        metric_ids: &[MetricId],
        context: &mut MetricContext,
    ) -> finstack_core::Result<HashMap<MetricId, F>> {
        // Build dependency graph and compute order
        let order = self.resolve_dependencies(metric_ids)?;
        
        // Compute metrics in dependency order
        for metric_id in order {
            // Skip if already computed
            if context.cache.computed.contains_key(&metric_id) {
                continue;
            }
            
            // Get calculator
            let calc = self.calculators.get(&metric_id)
                .ok_or_else(|| finstack_core::Error::from(
                    finstack_core::error::InputError::NotFound
                ))?;
            
            // Check if applicable to this instrument
            if !self.is_applicable(&metric_id, &context.instrument_data.instrument_type) {
                continue;
            }
            
            // Compute metric
            let value = calc.calculate(context)?;
            context.cache.computed.insert(metric_id, value);
        }
        
        // Return only the requested metrics
        let mut results = HashMap::new();
        for id in metric_ids {
            if let Some(&value) = context.cache.computed.get(id) {
                results.insert(id.clone(), value);
            }
        }
        
        Ok(results)
    }
    
    /// Compute all registered metrics applicable to the instrument.
    pub fn compute_all(
        &self,
        context: &mut MetricContext,
    ) -> finstack_core::Result<HashMap<MetricId, F>> {
        let applicable = self.metrics_for_instrument(&context.instrument_data.instrument_type);
        self.compute(&applicable, context)
    }
    
    /// Resolve dependencies and return computation order.
    /// 
    /// Uses topological sorting to ensure dependencies are computed first.
    fn resolve_dependencies(&self, metric_ids: &[MetricId]) -> finstack_core::Result<Vec<MetricId>> {
        let mut visited = hashbrown::HashSet::new();
        let mut order = Vec::new();
        let mut temp_mark = hashbrown::HashSet::new();
        
        for id in metric_ids {
            self.visit_metric(id.clone(), &mut visited, &mut temp_mark, &mut order)?;
        }
        
        Ok(order)
    }
    
    /// DFS visit for topological sort.
    fn visit_metric(
        &self,
        id: MetricId,
        visited: &mut hashbrown::HashSet<MetricId>,
        temp_mark: &mut hashbrown::HashSet<MetricId>,
        order: &mut Vec<MetricId>,
    ) -> finstack_core::Result<()> {
        if visited.contains(&id) {
            return Ok(());
        }
        
        if temp_mark.contains(&id) {
            return Err(finstack_core::Error::from(
                finstack_core::error::InputError::Invalid
            ));
        }
        
        temp_mark.insert(id.clone());
        
        // Get calculator and process dependencies
        if let Some(calc) = self.calculators.get(&id) {
            for dep_id in calc.dependencies() {
                self.visit_metric(dep_id.clone(), visited, temp_mark, order)?;
            }
        }
        
        temp_mark.remove(&id);
        visited.insert(id.clone());
        order.push(id);
        
        Ok(())
    }
}

impl Default for MetricRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Global registry builder for standard metrics.
pub struct StandardMetrics;

impl StandardMetrics {
    /// Create a registry with all standard bond metrics.
    pub fn bond_registry() -> MetricRegistry {
        MetricRegistry::new()
    }
    
    /// Create a registry with all standard IRS metrics.
    pub fn irs_registry() -> MetricRegistry {
        MetricRegistry::new()
    }
    
    /// Create a registry with generic risk metrics.
    pub fn risk_registry() -> MetricRegistry {
        MetricRegistry::new()
    }
    
    /// Create a combined registry with all standard metrics.
    pub fn combined_registry() -> MetricRegistry {
        let mut registry = MetricRegistry::new();
        
        // Merge all standard registries
        let bond = Self::bond_registry();
        let irs = Self::irs_registry();
        let risk = Self::risk_registry();
        
        for (id, calc) in bond.calculators {
            registry.calculators.insert(id, calc);
        }
        for (id, calc) in irs.calculators {
            registry.calculators.insert(id, calc);
        }
        for (id, calc) in risk.calculators {
            registry.calculators.insert(id, calc);
        }
        
        registry
    }
}
