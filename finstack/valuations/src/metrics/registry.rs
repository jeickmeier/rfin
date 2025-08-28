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
    applicability: HashMap<MetricId, Vec<&'static str>>,
}

impl MetricRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            calculators: HashMap::new(),
            applicability: HashMap::new(),
        }
    }
}

impl Default for MetricRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl MetricRegistry {
    /// Register a metric calculator with explicit ID and applicability.
    /// 
    /// If a calculator with the same ID already exists, it will be replaced.
    /// The `applicable_to` parameter explicitly specifies which instrument types
    /// this metric applies to. An empty slice means it applies to all instruments.
    pub fn register_metric(
        &mut self, 
        id: MetricId,
        calculator: Arc<dyn MetricCalculator>,
        applicable_to: &[&'static str]
    ) -> &mut Self {
        self.applicability.insert(id.clone(), applicable_to.to_vec());
        self.calculators.insert(id, calculator);
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
        self.applicability
            .iter()
            .filter(|(_, applicable)| {
                applicable.is_empty() || applicable.contains(&instrument_type)
            })
            .map(|(id, _)| id.clone())
            .collect()
    }
    
    /// Check if a metric is applicable to a specific instrument type.
    pub fn is_applicable(&self, metric_id: &MetricId, instrument_type: &str) -> bool {
        if let Some(applicable) = self.applicability.get(metric_id) {
            applicable.is_empty() || applicable.contains(&instrument_type)
        } else {
            false
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
        
        // Get instrument type once from the instrument
        let instrument_type = context.instrument.instrument_type();
        
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
            if !self.is_applicable(&metric_id, instrument_type) {
                continue;
            }
            
            // Compute metric
            let value = calc.calculate(context)?;
            context.computed.insert(metric_id, value);
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
    
    /// Compute all registered metrics applicable to the instrument.
    pub fn compute_all(
        &self,
        context: &mut MetricContext,
    ) -> finstack_core::Result<HashMap<MetricId, F>> {
        let instrument_type = context.instrument.instrument_type();
        let applicable = self.metrics_for_instrument(instrument_type);
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


