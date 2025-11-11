//! Bucketed CS01 for term loans using discount-curve key-rate bumps.

use crate::instruments::common::traits::Instrument;
use crate::instruments::TermLoan;
use crate::metrics::{bucketed_dv01::standard_ir_dv01_buckets, MetricCalculator, MetricContext};

#[derive(Debug, Default, Clone, Copy)]
pub struct BucketedCs01Calculator;

impl MetricCalculator for BucketedCs01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let loan: &TermLoan = context.instrument_as()?;
        let as_of = context.as_of;
        let inst_clone = loan.clone();

        let buckets = standard_ir_dv01_buckets();
        let discount_curve_id = inst_clone.discount_curve_id.clone();

        // Revalue via full MarketContext per bucket
        let reval = move |temp_ctx: &finstack_core::market_data::MarketContext| {
            inst_clone.value(temp_ctx, as_of)
        };

        crate::metrics::bucketed_dv01::compute_key_rate_series_with_context_for_id(
            context,
            crate::metrics::MetricId::BucketedCs01,
            &discount_curve_id,
            buckets,
            1.0,
            reval,
        )
    }
}
