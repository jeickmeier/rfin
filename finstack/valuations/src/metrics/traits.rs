#![deny(missing_docs)]
//! Core traits for the metrics framework.

use finstack_core::prelude::*;
use finstack_core::F;
use std::sync::Arc;

/// Core trait for metric calculators.
/// 
/// Each metric calculator is responsible for computing a single metric value
/// based on the provided context. Calculators can declare dependencies on other
/// metrics to enable efficient computation ordering and caching.
pub trait MetricCalculator: Send + Sync {
    /// Unique identifier for this metric (e.g., "ytm", "duration", "dv01").
    fn id(&self) -> &str;
    
    /// Compute the metric value based on the provided context.
    /// 
    /// # Errors
    /// Returns an error if the metric cannot be computed due to missing data
    /// or invalid instrument configuration.
    fn calculate(&self, context: &MetricContext) -> finstack_core::Result<F>;
    
    /// List of metric IDs this calculator depends on.
    /// 
    /// Dependencies will be computed first and made available via
    /// `context.computed`. Default implementation returns no dependencies.
    fn dependencies(&self) -> Vec<&str> {
        Vec::new()
    }
    
    /// Human-readable description of this metric.
    fn description(&self) -> &str {
        self.id()
    }
    
    /// Whether this metric is applicable to the given instrument type.
    /// 
    /// Default implementation returns true for all instruments.
    fn is_applicable(&self, _instrument_type: &str) -> bool {
        true
    }
}

/// Context containing all data needed for metric calculations.
/// 
/// The context provides access to the instrument, market data, base valuation,
/// and any previously computed metrics. It also supports caching of intermediate
/// results like cashflows and discount factors.
pub struct MetricContext {
    /// The instrument being valued.
    pub instrument: Arc<dyn std::any::Any + Send + Sync>,
    
    /// The instrument type identifier (e.g., "Bond", "IRS", "Deposit").
    pub instrument_type: String,
    
    /// Market curves for discounting and forwarding.
    pub curves: Arc<finstack_core::market_data::multicurve::CurveSet>,
    
    /// Valuation date.
    pub as_of: Date,
    
    /// Base present value of the instrument.
    pub base_value: Money,
    
    /// Previously computed metrics (by ID).
    pub computed: hashbrown::HashMap<String, F>,
    
    /// Cached intermediate results (generic storage).
    cache: hashbrown::HashMap<String, Arc<dyn std::any::Any + Send + Sync>>,
}

impl MetricContext {
    /// Create a new metric context.
    pub fn new(
        instrument: Arc<dyn std::any::Any + Send + Sync>,
        instrument_type: String,
        curves: Arc<finstack_core::market_data::multicurve::CurveSet>,
        as_of: Date,
        base_value: Money,
    ) -> Self {
        Self {
            instrument,
            instrument_type,
            curves,
            as_of,
            base_value,
            computed: hashbrown::HashMap::new(),
            cache: hashbrown::HashMap::new(),
        }
    }
    
    /// Get a cached value by key, computing it if not present.
    /// 
    /// This is useful for expensive intermediate calculations that multiple
    /// metrics might need (e.g., cashflow generation, discount factors).
    pub fn get_or_compute<T, F>(&mut self, key: &str, compute: F) -> Arc<T>
    where
        T: Send + Sync + 'static,
        F: FnOnce() -> T,
    {
        self.cache
            .entry(key.to_string())
            .or_insert_with(|| Arc::new(compute()) as Arc<dyn std::any::Any + Send + Sync>)
            .clone()
            .downcast::<T>()
            .expect("Cache type mismatch")
    }
    
    /// Get a previously cached value if it exists.
    pub fn get_cached<T: Send + Sync + 'static>(&self, key: &str) -> Option<Arc<T>> {
        self.cache.get(key)?.clone().downcast::<T>().ok()
    }
    
    /// Store a value in the cache.
    pub fn cache_value<T: Send + Sync + 'static>(&mut self, key: &str, value: T) {
        self.cache.insert(key.to_string(), Arc::new(value) as Arc<dyn std::any::Any + Send + Sync>);
    }
    
    /// Try to downcast the instrument to a specific type.
    pub fn instrument_as<T: 'static>(&self) -> Option<&T> {
        self.instrument.downcast_ref::<T>()
    }
}

/// Trait for instruments that support the metrics framework.
pub trait MetricsEnabled {
    /// Get the instrument type identifier.
    fn instrument_type(&self) -> &str;
    
    /// Get the list of standard metrics for this instrument type.
    fn standard_metrics(&self) -> Vec<&str>;
    
    /// Create a metric context for this instrument.
    fn create_context(
        &self,
        curves: Arc<finstack_core::market_data::multicurve::CurveSet>,
        as_of: Date,
        base_value: Money,
    ) -> MetricContext;
}
