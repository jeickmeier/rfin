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
    
    /// Cached cashflows for the instrument.
    pub cashflows: Option<Vec<(Date, Money)>>,
    
    /// Cached discount curve ID.
    pub discount_curve_id: Option<&'static str>,
    
    /// Cached day count convention.
    pub day_count: Option<DayCount>,
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
    
    /// Get or compute cashflows.
    pub fn get_or_compute_cashflows<F>(&mut self, compute: F) -> &Vec<(Date, Money)>
    where
        F: FnOnce() -> Vec<(Date, Money)>,
    {
        self.cashflows.get_or_insert_with(compute)
    }
    
    /// Get or compute discount curve ID.
    pub fn get_or_compute_discount_curve<F>(&mut self, compute: F) -> &'static str
    where
        F: FnOnce() -> &'static str,
    {
        self.discount_curve_id.get_or_insert_with(compute)
    }
    
    /// Get or compute day count convention.
    pub fn get_or_compute_day_count<F>(&mut self, compute: F) -> DayCount
    where
        F: FnOnce() -> DayCount,
    {
        *self.day_count.get_or_insert_with(compute)
    }
    
    /// Get cached cashflows if they exist.
    pub fn get_cached_cashflows(&self) -> Option<&Vec<(Date, Money)>> {
        self.cashflows.as_ref()
    }
    
    /// Get cached discount curve ID if it exists.
    pub fn get_cached_discount_curve(&self) -> Option<&'static str> {
        self.discount_curve_id
    }
    
    /// Get cached day count convention if it exists.
    pub fn get_cached_day_count(&self) -> Option<DayCount> {
        self.day_count
    }
    
    /// Cache cashflows.
    pub fn cache_cashflows(&mut self, flows: Vec<(Date, Money)>) {
        self.cashflows = Some(flows);
    }
    
    /// Cache discount curve ID.
    pub fn cache_discount_curve(&mut self, curve_id: &'static str) {
        self.discount_curve_id = Some(curve_id);
    }
    
    /// Cache day count convention.
    pub fn cache_day_count(&mut self, dc: DayCount) {
        self.day_count = Some(dc);
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
            cache: ComputationCache {
                computed: hashbrown::HashMap::new(),
                cashflows: None,
                discount_curve_id: None,
                day_count: None,
            },
        }
    }
    
    /// Get or compute cashflows.
    pub fn get_or_compute_cashflows<F>(&mut self, compute: F) -> &Vec<(Date, Money)>
    where
        F: FnOnce() -> Vec<(Date, Money)>,
    {
        self.cache.get_or_compute_cashflows(compute)
    }
    
    /// Get or compute discount curve ID.
    pub fn get_or_compute_discount_curve<F>(&mut self, compute: F) -> &'static str
    where
        F: FnOnce() -> &'static str,
    {
        self.cache.get_or_compute_discount_curve(compute)
    }
    
    /// Get or compute day count convention.
    pub fn get_or_compute_day_count<F>(&mut self, compute: F) -> DayCount
    where
        F: FnOnce() -> DayCount,
    {
        self.cache.get_or_compute_day_count(compute)
    }
    
    /// Get cached cashflows if they exist.
    pub fn get_cached_cashflows(&self) -> Option<&Vec<(Date, Money)>> {
        self.cache.get_cached_cashflows()
    }
    
    /// Get cached discount curve ID if it exists.
    pub fn get_cached_discount_curve(&self) -> Option<&'static str> {
        self.cache.get_cached_discount_curve()
    }
    
    /// Get cached day count convention if it exists.
    pub fn get_cached_day_count(&self) -> Option<DayCount> {
        self.cache.get_cached_day_count()
    }
    
    /// Cache cashflows.
    pub fn cache_cashflows(&mut self, flows: Vec<(Date, Money)>) {
        self.cache.cache_cashflows(flows);
    }
    
    /// Cache discount curve ID.
    pub fn cache_discount_curve(&mut self, curve_id: &'static str) {
        self.cache.cache_discount_curve(curve_id);
    }
    
    /// Cache day count convention.
    pub fn cache_day_count(&mut self, dc: DayCount) {
        self.cache.cache_day_count(dc);
    }
    
    /// Get the instrument, allowing pattern matching.
    pub fn instrument(&self) -> &Instrument {
        self.instrument_data.instrument()
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


