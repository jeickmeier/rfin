#![deny(missing_docs)]
//! Core traits for the metrics framework.

use crate::instruments::Instrument;
use crate::metrics::MetricId;
use finstack_core::prelude::*;
use finstack_core::F;
use std::sync::Arc;

/// Core trait for metric calculators.
/// 
/// Each metric calculator is responsible for computing a single metric value
/// based on the provided context. Calculators can declare dependencies on other
/// metrics to enable efficient computation ordering and caching.
pub trait MetricCalculator: Send + Sync {
    /// Unique identifier for this metric.
    fn id(&self) -> MetricId;
    
    /// Compute the metric value based on the provided context.
    /// 
    /// # Errors
    /// Returns an error if the metric cannot be computed due to missing data
    /// or invalid instrument configuration.
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F>;
    
    /// List of metric IDs this calculator depends on.
    /// 
    /// Dependencies will be computed first and made available via
    /// `context.computed`. Default implementation returns no dependencies.
    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Instrument-specific data for metric calculations.
/// 
/// Contains the instrument being valued and its type identifier.
pub struct InstrumentData {
    /// The instrument being valued.
    pub instrument: Arc<Instrument>,
    
    /// The instrument type identifier (e.g., "Bond", "IRS", "Deposit").
    pub instrument_type: String,
}

/// Market data for metric calculations.
/// 
/// Contains market curves and valuation date.
pub struct MarketData {
    /// Market curves for discounting and forwarding.
    pub curves: Arc<finstack_core::market_data::multicurve::CurveSet>,
    
    /// Valuation date.
    pub as_of: Date,
}

/// Computation cache and results storage.
/// 
/// Handles caching of intermediate results and previously computed metrics.
pub struct ComputationCache {
    /// Previously computed metrics (by ID).
    pub computed: hashbrown::HashMap<MetricId, F>,
    
    /// Cached intermediate results (generic storage).
    cache: hashbrown::HashMap<String, Arc<dyn std::any::Any + Send + Sync>>,
}

/// Context containing all data needed for metric calculations.
/// 
/// The context provides access to the instrument, market data, base valuation,
/// and any previously computed metrics. It also supports caching of intermediate
/// results like cashflows and discount factors.
pub struct MetricContext {
    /// Instrument-specific data.
    pub instrument_data: InstrumentData,
    
    /// Market data.
    pub market_data: MarketData,
    
    /// Base present value of the instrument.
    pub base_value: Money,
    
    /// Computation cache and results.
    pub cache: ComputationCache,
}

impl InstrumentData {
    /// Create new instrument data.
    pub fn new(instrument: Arc<Instrument>, instrument_type: String) -> Self {
        Self {
            instrument,
            instrument_type,
        }
    }
    
    /// Get the instrument, allowing pattern matching.
    pub fn instrument(&self) -> &Instrument {
        &self.instrument
    }
}

impl MarketData {
    /// Create new market data.
    pub fn new(
        curves: Arc<finstack_core::market_data::multicurve::CurveSet>,
        as_of: Date,
    ) -> Self {
        Self { curves, as_of }
    }
}

impl ComputationCache {
    /// Create a new empty cache.
    pub fn new() -> Self {
        Self {
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
}

impl Default for ComputationCache {
    fn default() -> Self {
        Self::new()
    }
}

impl MetricContext {
    /// Create a new metric context.
    pub fn new(
        instrument: Arc<Instrument>,
        instrument_type: String,
        curves: Arc<finstack_core::market_data::multicurve::CurveSet>,
        as_of: Date,
        base_value: Money,
    ) -> Self {
        Self {
            instrument_data: InstrumentData::new(instrument, instrument_type),
            market_data: MarketData::new(curves, as_of),
            base_value,
            cache: ComputationCache::new(),
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
        self.cache.get_or_compute(key, compute)
    }
    
    /// Get a previously cached value if it exists.
    pub fn get_cached<T: Send + Sync + 'static>(&self, key: &str) -> Option<Arc<T>> {
        self.cache.get_cached(key)
    }
    
    /// Store a value in the cache.
    pub fn cache_value<T: Send + Sync + 'static>(&mut self, key: &str, value: T) {
        self.cache.cache_value(key, value);
    }
    
    /// Get the instrument, allowing pattern matching.
    pub fn instrument(&self) -> &Instrument {
        self.instrument_data.instrument()
    }
    
    // Convenience accessors for backward compatibility
    /// Get the instrument type.
    pub fn instrument_type(&self) -> &str {
        &self.instrument_data.instrument_type
    }
    
    /// Get the curves.
    pub fn curves(&self) -> &Arc<finstack_core::market_data::multicurve::CurveSet> {
        &self.market_data.curves
    }
    
    /// Get the valuation date.
    pub fn as_of(&self) -> Date {
        self.market_data.as_of
    }
    
    /// Get previously computed metrics.
    pub fn computed(&self) -> &hashbrown::HashMap<MetricId, F> {
        &self.cache.computed
    }
    
    /// Get mutable access to computed metrics.
    pub fn computed_mut(&mut self) -> &mut hashbrown::HashMap<MetricId, F> {
        &mut self.cache.computed
    }
}


