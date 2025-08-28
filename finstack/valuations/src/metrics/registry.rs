#![deny(missing_docs)]
//! Metric registry and computation engine.

use super::traits::{MetricCalculator, MetricContext};
use finstack_core::F;
use hashbrown::HashMap;
use std::sync::Arc;

/// Registry for metric calculators.
/// 
/// The registry manages a collection of metric calculators and handles
/// dependency resolution, caching, and batch computation of metrics.
#[derive(Clone)]
pub struct MetricRegistry {
    calculators: HashMap<String, Arc<dyn MetricCalculator>>,
}

impl MetricRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            calculators: HashMap::new(),
        }
    }
    
    /// Register a metric calculator.
    /// 
    /// If a calculator with the same ID already exists, it will be replaced.
    pub fn register(&mut self, calculator: Arc<dyn MetricCalculator>) -> &mut Self {
        self.calculators.insert(calculator.id().to_string(), calculator);
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
    pub fn has_metric(&self, id: &str) -> bool {
        self.calculators.contains_key(id)
    }
    
    /// Get a list of all registered metric IDs.
    pub fn available_metrics(&self) -> Vec<String> {
        self.calculators.keys().cloned().collect()
    }
    
    /// Get metrics applicable to a specific instrument type.
    pub fn metrics_for_instrument(&self, instrument_type: &str) -> Vec<String> {
        self.calculators
            .iter()
            .filter(|(_, calc)| calc.is_applicable(instrument_type))
            .map(|(id, _)| id.clone())
            .collect()
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
        metric_ids: &[&str],
        context: &mut MetricContext,
    ) -> finstack_core::Result<HashMap<String, F>> {
        // Build dependency graph and compute order
        let order = self.resolve_dependencies(metric_ids)?;
        
        // Compute metrics in dependency order
        for metric_id in order {
            // Skip if already computed
            if context.computed.contains_key(&metric_id) {
                continue;
            }
            
            // Get calculator
            let calc = self.calculators.get(&metric_id)
                .ok_or_else(|| finstack_core::Error::from(
                    finstack_core::error::InputError::NotFound
                ))?;
            
            // Check if applicable to this instrument
            if !calc.is_applicable(&context.instrument_type) {
                continue;
            }
            
            // Compute metric
            let value = calc.calculate(context)?;
            context.computed.insert(metric_id.clone(), value);
        }
        
        // Return only the requested metrics
        let mut results = HashMap::new();
        for &id in metric_ids {
            if let Some(&value) = context.computed.get(id) {
                results.insert(id.to_string(), value);
            }
        }
        
        Ok(results)
    }
    
    /// Compute all registered metrics applicable to the instrument.
    pub fn compute_all(
        &self,
        context: &mut MetricContext,
    ) -> finstack_core::Result<HashMap<String, F>> {
        let applicable: Vec<&str> = self.calculators
            .iter()
            .filter(|(_, calc)| calc.is_applicable(&context.instrument_type))
            .map(|(id, _)| id.as_str())
            .collect();
        
        self.compute(&applicable, context)
    }
    
    /// Resolve dependencies and return computation order.
    /// 
    /// Uses topological sorting to ensure dependencies are computed first.
    fn resolve_dependencies(&self, metric_ids: &[&str]) -> finstack_core::Result<Vec<String>> {
        let mut visited = hashbrown::HashSet::new();
        let mut order = Vec::new();
        let mut temp_mark = hashbrown::HashSet::new();
        
        for &id in metric_ids {
            self.visit_metric(id, &mut visited, &mut temp_mark, &mut order)?;
        }
        
        Ok(order)
    }
    
    /// DFS visit for topological sort.
    fn visit_metric(
        &self,
        id: &str,
        visited: &mut hashbrown::HashSet<String>,
        temp_mark: &mut hashbrown::HashSet<String>,
        order: &mut Vec<String>,
    ) -> finstack_core::Result<()> {
        if visited.contains(id) {
            return Ok(());
        }
        
        if temp_mark.contains(id) {
            return Err(finstack_core::Error::from(
                finstack_core::error::InputError::Invalid
            ));
        }
        
        temp_mark.insert(id.to_string());
        
        // Get calculator and process dependencies
        if let Some(calc) = self.calculators.get(id) {
            for dep_id in calc.dependencies() {
                self.visit_metric(dep_id, visited, temp_mark, order)?;
            }
        }
        
        temp_mark.remove(id);
        visited.insert(id.to_string());
        order.push(id.to_string());
        
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
        
        // Register standard bond metrics (to be implemented)
        // registry.register(Arc::new(YtmCalculator));
        // registry.register(Arc::new(DurationCalculator));
        // registry.register(Arc::new(ConvexityCalculator));
        // registry.register(Arc::new(AccruedInterestCalculator));
    }
    
    /// Create a registry with all standard IRS metrics.
    pub fn irs_registry() -> MetricRegistry {
        MetricRegistry::new()
        
        // Register standard IRS metrics (to be implemented)
        // registry.register(Arc::new(ParRateCalculator));
        // registry.register(Arc::new(AnnuityCalculator));
        // registry.register(Arc::new(Dv01Calculator));
    }
    
    /// Create a registry with generic risk metrics.
    pub fn risk_registry() -> MetricRegistry {
        MetricRegistry::new()
        
        // Register generic risk metrics (to be implemented)
        // registry.register(Arc::new(BucketedDv01Calculator));
        // registry.register(Arc::new(Cs01Calculator));
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
