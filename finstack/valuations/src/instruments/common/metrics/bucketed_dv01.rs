//! Generic BucketedDv01 calculator to eliminate duplication across instruments.
//!
//! This module provides a generic implementation that can be used by any instrument
//! that has a discount curve and can be valued using standard revaluation patterns.

use std::marker::PhantomData;

use crate::cashflow::traits::CashflowProvider;
use crate::instruments::common::traits::Instrument;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::types::CurveId;


/// Trait for instruments that have a primary discount curve for valuation.
pub trait HasDiscountCurve {
    /// Get the instrument's primary discount curve ID.
    fn discount_curve_id(&self) -> &CurveId;
}

/// Generic BucketedDv01 calculator that works for any instrument implementing
/// the required traits.
pub struct GenericBucketedDv01<I> {
    _phantom: PhantomData<I>,
}

impl<I> Default for GenericBucketedDv01<I> {
    fn default() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<I> MetricCalculator for GenericBucketedDv01<I>
where
    I: Instrument + HasDiscountCurve + CashflowProvider + Clone + 'static,
{
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let instrument: &I = context.instrument_as()?;
        let disc_id = instrument.discount_curve_id().clone();

        // Standard bucket times (years) - shared across all instruments
        let buckets = crate::metrics::standard_ir_dv01_buckets();

        // Generic revaluation using cashflow building and discounting
        let inst_clone = instrument.clone();
        let curves = context.curves.clone();
        let as_of = context.as_of;

        let reval = move |
            bumped_disc: &finstack_core::market_data::term_structures::discount_curve::DiscountCurve|
         {
            // Build flows using original curves (preserves forward projections)
            let flows = inst_clone.build_schedule(&curves, as_of)?;
            let base = bumped_disc.base_date();
            let dc = bumped_disc.day_count();

            // Discount using bumped curve
            crate::instruments::common::discountable::npv_static(
                bumped_disc,
                base,
                dc,
                &flows,
            )
        };

        let total =
            crate::metrics::compute_key_rate_dv01_series(context, &disc_id, buckets, 1.0, reval)?;

        Ok(total)
    }
}

/// Alternative generic calculator for instruments that need full MarketContext revaluation.
///
/// Use this for instruments whose pricing requires access to multiple curves or
/// complex pricing models that can't be reduced to simple cashflow discounting.
pub struct GenericBucketedDv01WithContext<I> {
    _phantom: PhantomData<I>,
}

impl<I> Default for GenericBucketedDv01WithContext<I> {
    fn default() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<I> MetricCalculator for GenericBucketedDv01WithContext<I>
where
    I: Instrument + HasDiscountCurve + Clone + 'static,
{
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let instrument: &I = context.instrument_as()?;
        let disc_id = instrument.discount_curve_id().clone();

        // Standard bucket times
        let buckets = crate::metrics::standard_ir_dv01_buckets();

        // Revaluation using full MarketContext (for complex pricers)
        let inst_clone = instrument.clone();
        let as_of = context.as_of;

        let reval = move |temp_ctx: &finstack_core::market_data::MarketContext| {
            inst_clone.value(temp_ctx, as_of)
        };

        let total = crate::metrics::compute_key_rate_dv01_series_with_context(
            context, &disc_id, buckets, 1.0, reval,
        )?;

        Ok(total)
    }
}
