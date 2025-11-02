//! Unified pricer for all structured credit instruments.
//!
//! The pricing logic is identical across ABS, CLO, CMBS, and RMBS since they all
//! use the shared waterfall implementation via the `StructuredCreditInstrument` trait.

use super::StructuredCredit;
use crate::cashflow::traits::CashflowProvider;
use crate::instruments::common::discountable::Discountable;
use crate::metrics::MetricId;
use crate::results::ValuationResult;
use finstack_core::dates::{Date, DayCount};
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;

impl StructuredCredit {
    /// Value the structured credit instrument using discounted cashflow analysis.
    ///
    /// This method generates cashflows through the waterfall engine and discounts
    /// them back to present value using the instrument's discount curve.
    pub fn price(&self, context: &MarketContext, as_of: Date) -> finstack_core::Result<Money> {
        let disc = context.get_discount_ref(self.discount_curve_id.as_str())?;
        let flows = self.build_schedule(context, as_of)?;

        flows.npv(disc, as_of, DayCount::Act360)
    }

    /// Price with additional risk metrics.
    ///
    /// Computes the base NPV plus any requested metrics such as duration, spread, etc.
    pub fn price_with_metrics_standalone(
        &self,
        context: &MarketContext,
        as_of: Date,
        metrics: &[MetricId],
    ) -> finstack_core::Result<ValuationResult> {
        let base_value = self.price(context, as_of)?;

        if metrics.is_empty() {
            return Ok(ValuationResult::stamped(
                self.id.as_str(),
                as_of,
                base_value,
            ));
        }

        let flows = self.build_schedule(context, as_of)?;
        let mut metric_context = crate::metrics::MetricContext::new(
            std::sync::Arc::new(self.clone())
                as std::sync::Arc<dyn crate::instruments::common::traits::Instrument>,
            std::sync::Arc::new(context.clone()),
            as_of,
            base_value,
        );
        metric_context.cashflows = Some(flows);
        metric_context.discount_curve_id = Some(self.discount_curve_id.to_owned());

        let registry = crate::metrics::standard_registry();
        let computed_metrics = registry.compute(metrics, &mut metric_context)?;

        let mut result = ValuationResult::stamped(self.id.as_str(), as_of, base_value);
        for (metric_id, value) in computed_metrics {
            result.measures.insert(metric_id.to_string(), value);
        }

        Ok(result)
    }
}

// Using generic pricer implementation to eliminate boilerplate
pub use crate::instruments::common::GenericDiscountingPricer;

/// Structured Credit discounting pricer using the generic implementation.
///
/// This pricer handles all structured credit deal types (ABS, CLO, CMBS, RMBS)
/// using the unified waterfall implementation.
pub type StructuredCreditDiscountingPricer = GenericDiscountingPricer<StructuredCredit>;

impl Default for StructuredCreditDiscountingPricer {
    fn default() -> Self {
        Self::new(crate::pricer::InstrumentType::StructuredCredit)
    }
}
