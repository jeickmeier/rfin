#![allow(missing_docs)]

use finstack_core::prelude::*;
use finstack_core::market_data::multicurve::CurveSet;
use finstack_core::market_data::traits::Discount;
use crate::pricing::discountable::Discountable;
use crate::metrics::MetricId;

/// Currency-preserving schedule as a list of dated `Money` amounts.
pub type DatedFlows = Vec<(Date, Money)>;

/// Build cashflow schedules and provide currency-safe aggregation hooks.
pub trait CashflowProvider: Send + Sync {
    /// Build complete dated cashflow schedule as `(date, amount)` pairs.
    fn build_schedule(&self, curves: &CurveSet, as_of: Date) -> finstack_core::Result<DatedFlows>;

    /// Convenience: present value the built schedule against a discount curve and day-count.
    #[inline]
    fn npv_with(
        &self,
        curves: &CurveSet,
        as_of: Date,
        disc: &dyn Discount,
        dc: DayCount,
    ) -> finstack_core::Result<Money> {
        let base = disc.base_date();
        let flows = self.build_schedule(curves, as_of)?;
        flows.npv(disc, base, dc)
    }
}

/// Priceable instruments produce a `ValuationResult` at `as_of` using curves.
/// 
/// The default implementation now uses the metrics framework to compute
/// measures, delegating to `value()` for base NPV calculation.
pub trait Priceable: Send + Sync {
    /// Compute full valuation with all standard metrics (backward compatible).
    fn price(&self, curves: &CurveSet, as_of: Date) -> finstack_core::Result<super::pricing::result::ValuationResult>;
    
    /// Compute only the base present value (fast, no metrics).
    fn value(&self, curves: &CurveSet, as_of: Date) -> finstack_core::Result<Money> {
        // Default implementation for backward compatibility
        self.price(curves, as_of).map(|r| r.value)
    }
    
    /// Compute value with specific metrics.
    fn price_with_metrics(
        &self, 
        curves: &CurveSet, 
        as_of: Date, 
        metrics: &[MetricId]
    ) -> finstack_core::Result<super::pricing::result::ValuationResult> {
        // Default implementation: just calls price() and filters metrics
        let result = self.price(curves, as_of)?;
        let mut filtered_result = result.clone();
        
        // Convert MetricIds to strings for filtering
        let metric_strs: Vec<String> = metrics.iter().map(|m| m.as_str().to_string()).collect();
        filtered_result.measures.retain(|k, _| metric_strs.contains(k));
        Ok(filtered_result)
    }
}