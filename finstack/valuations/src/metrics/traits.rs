//! Core traits for the metrics framework.
//! 
//! Defines the fundamental interfaces for implementing and using financial
//! metrics. The `MetricCalculator` trait enables custom metric implementations,
//! while `MetricContext` provides the execution environment with caching.

use crate::instruments::Instrument;
use crate::metrics::MetricId;
use finstack_core::prelude::*;
use finstack_core::F;
use std::sync::Arc;

/// Core trait for metric calculators.
/// 
/// Each calculator computes a single metric value based on the provided context.
/// Calculators can declare dependencies on other metrics for efficient computation
/// ordering and caching. Implement this trait to create custom financial metrics.
/// 
/// # Example
/// ```rust
/// use finstack_valuations::metrics::traits::{MetricCalculator, MetricContext};
/// use finstack_valuations::metrics::ids::MetricId;
/// use finstack_core::Result;
/// use std::sync::Arc;
/// 
/// struct CustomYieldCalculator;
/// 
/// impl MetricCalculator for CustomYieldCalculator {
///     fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
///         // Custom yield calculation logic
///         Ok(0.05) // 5% yield
///     }
///     
///     fn dependencies(&self) -> &[MetricId] {
///         &[MetricId::DirtyPrice] // Depends on dirty price
///     }
/// }
/// ```
pub trait MetricCalculator: Send + Sync {
    /// Computes the metric value based on the provided context.
    /// 
    /// This method should implement the core calculation logic for the metric.
    /// It can access cached results from `context.computed` for dependencies.
    /// 
    /// # Arguments
    /// * `context` - Metric context containing instrument, market data, and cached results
    /// 
    /// # Returns
    /// The computed metric value as a `Result<F>`
    /// 
    /// # Errors
    /// Returns an error if the metric cannot be computed due to missing data
    /// or invalid instrument configuration.
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F>;
    
    /// Lists metric IDs this calculator depends on.
    /// 
    /// Dependencies will be computed first and made available via
    /// `context.computed`. The registry uses this to determine computation order.
    /// 
    /// # Returns
    /// Slice of metric IDs that must be computed before this metric
    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Context containing all data needed for metric calculations.
/// 
/// Provides access to the instrument, market data, base valuation,
/// and any previously computed metrics. Supports caching of intermediate
/// results like cashflows and discount factors to improve performance.
/// 
/// # Key Features
/// 
/// - **Instrument data**: Access to the instrument being valued
/// - **Market curves**: Discount and forward curves for calculations
/// - **Cached results**: Previously computed metrics for dependency resolution
/// - **Cashflow caching**: Optional caching of instrument cashflows
/// - **Metadata**: Discount curve ID and day count convention
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
    /// Creates a new metric context.
    /// 
    /// # Arguments
    /// * `instrument` - The instrument to value
    /// * `curves` - Market curves for discounting and forwarding
    /// * `as_of` - Valuation date
    /// * `base_value` - Base present value of the instrument
    /// 
    /// # Example
    /// ```rust
    /// use finstack_valuations::metrics::traits::MetricContext;
    /// use finstack_valuations::instruments::Instrument;
    /// use finstack_core::currency::Currency;
    /// use finstack_core::money::Money;
    /// use finstack_core::dates::Date;
    /// use finstack_core::market_data::multicurve::CurveSet;
    /// use std::sync::Arc;
    /// use time::Month;
    /// 
    /// // Note: These would be created from actual data
    /// // let instrument: Arc<Instrument> = todo!();
    /// // let curves: Arc<CurveSet> = todo!();
    /// let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    /// let base_value = Money::new(1_000_000.0, Currency::USD);
    /// 
    /// // Note: MetricContext::new would be called here
    /// // let context = MetricContext::new(instrument, curves, as_of, base_value);
    /// ```
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


