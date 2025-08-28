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
/// 
/// Note: Metric ID and applicability are now provided during registration
/// via `MetricRegistry::register_metric()` rather than through trait methods.
pub trait MetricCalculator: Send + Sync {
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

/// Context containing all data needed for metric calculations.
/// 
/// The context provides access to the instrument, market data, base valuation,
/// and any previously computed metrics. It also supports caching of intermediate
/// results like cashflows and discount factors.
pub struct MetricContext {
    /// The instrument being valued.
    pub instrument: Arc<Instrument>,
    
    /// Market curves for discounting and forwarding.
    pub curves: Arc<finstack_core::market_data::multicurve::CurveSet>,
    
    /// Valuation date.
    pub as_of: Date,
    
    /// Base present value of the instrument.
    pub base_value: Money,
    
    /// Previously computed metrics (by ID).
    pub computed: hashbrown::HashMap<MetricId, F>,
    
    /// Cached cashflows for the instrument.
    pub cashflows: Option<Vec<(Date, Money)>>,
    
    /// Cached discount curve ID.
    pub discount_curve_id: Option<&'static str>,
    
    /// Cached day count convention.
    pub day_count: Option<DayCount>,
}





impl MetricContext {
    /// Create a new metric context.
    pub fn new(
        instrument: Arc<Instrument>,
        curves: Arc<finstack_core::market_data::multicurve::CurveSet>,
        as_of: Date,
        base_value: Money,
    ) -> Self {
        Self {
            instrument,
            curves,
            as_of,
            base_value,
            computed: hashbrown::HashMap::new(),
            cashflows: None,
            discount_curve_id: None,
            day_count: None,
        }
    }
    

}


