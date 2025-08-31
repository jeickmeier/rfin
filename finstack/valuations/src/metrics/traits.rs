//! Core traits for the metrics framework.
//!
//! Defines the fundamental interfaces for implementing and using financial
//! metrics. The `MetricCalculator` trait enables custom metric implementations,
//! while `MetricContext` provides the execution environment with caching.

use crate::instruments::traits::InstrumentLike;
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
/// See unit tests and `examples/` for usage.
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
    pub instrument: Arc<dyn InstrumentLike>,

    /// Market curves for discounting and forwarding.
    pub curves: Arc<finstack_core::market_data::MarketContext>,

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
    /// See unit tests and `examples/` for usage.
    pub fn new(
        instrument: Arc<dyn InstrumentLike>,
        curves: Arc<finstack_core::market_data::MarketContext>,
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

    /// Downcast the instrument to a specific concrete type.
    ///
    /// # Returns
    /// Reference to the concrete instrument type if the downcast succeeds
    ///
    /// # Errors
    /// Returns an error if the instrument is not of the expected type
    pub fn instrument_as<T: 'static>(&self) -> finstack_core::Result<&T> {
        self.instrument
            .as_any()
            .downcast_ref::<T>()
            .ok_or_else(|| finstack_core::error::InputError::Invalid.into())
    }
}
