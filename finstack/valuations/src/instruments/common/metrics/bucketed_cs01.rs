//! Generic BucketedCs01 calculator to eliminate duplication across credit instruments.
//!
//! This module provides a generic implementation that can be used by any instrument
//! that has a credit curve and can be valued using standard revaluation patterns.

use std::marker::PhantomData;

use crate::instruments::common::traits::Instrument;
use crate::metrics::{MetricCalculator, MetricContext};

/// Trait for instruments that have a primary credit curve.
///
/// Used by generic bucketed CS01 calculators to identify which credit curve
/// to bump for credit spread sensitivity calculations.
pub trait HasCreditCurve {
    /// Returns the ID of the primary credit curve used for credit spread sensitivity.
    fn credit_curve_id(&self) -> &finstack_core::types::CurveId;
}

/// Generic BucketedCs01 calculator that works for any instrument implementing
/// the required traits.
pub struct GenericBucketedCs01<I> {
    _phantom: PhantomData<I>,
}

impl<I> Default for GenericBucketedCs01<I> {
    fn default() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<I> MetricCalculator for GenericBucketedCs01<I>
where
    I: Instrument + HasCreditCurve + Clone + 'static,
{
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let instrument: &I = context.instrument_as()?;
        let hazard_id = instrument.credit_curve_id().clone();

        // Standard credit bucket times
        let buckets = crate::metrics::bucketed_cs01::standard_credit_cs01_buckets();

        // Generic revaluation using full MarketContext (for complex pricers)
        let inst_clone = instrument.clone();
        let as_of = context.as_of;

        let reval = move |temp_ctx: &finstack_core::market_data::MarketContext| {
            inst_clone.value(temp_ctx, as_of)
        };

        let total = crate::metrics::bucketed_cs01::compute_key_rate_cs01_series_with_context(
            context, &hazard_id, buckets, 1.0, reval,
        )?;

        Ok(total)
    }
}
