//! Core traits for financial instruments.

use finstack_core::prelude::*;
use finstack_core::market_data::multicurve::CurveSet;
use finstack_core::market_data::traits::Discount;
use crate::pricing::discountable::Discountable;
use crate::metrics::MetricId;

/// Currency-preserving schedule as a list of dated `Money` amounts.
/// 
/// Used for cashflow aggregation and NPV calculations across different
/// instruments and time periods.
pub type DatedFlows = Vec<(Date, Money)>;

/// Build cashflow schedules and provide currency-safe aggregation hooks.
/// 
/// Instruments implement this to generate their cashflow schedules
/// given market curves and valuation date.
pub trait CashflowProvider: Send + Sync {
    /// Build complete dated cashflow schedule as `(date, amount)` pairs.
    /// 
    /// # Errors
    /// Returns an error if the schedule cannot be built due to invalid
    /// instrument parameters or missing market data.
    fn build_schedule(&self, curves: &CurveSet, as_of: Date) -> finstack_core::Result<DatedFlows>;

    /// Convenience: present value the built schedule against a discount curve and day-count.
    /// 
    /// # Example
    /// ```rust
    /// # use finstack_valuations::traits::CashflowProvider;
    /// # use finstack_core::market_data::multicurve::CurveSet;
    /// # use finstack_core::dates::Date;
    /// # use finstack_core::market_data::traits::Discount;
    /// # use finstack_core::dates::DayCount;
    /// # // Note: These would be created from actual data
    /// # // let provider: &dyn CashflowProvider = todo!();
    /// # // let curves: &CurveSet = todo!();
    /// # let as_of = Date::from_calendar_date(2025, time::Month::January, 1).unwrap();
    /// # // let disc: &dyn Discount = todo!();
    /// # let dc = DayCount::Act365F;
    /// # // let npv = provider.npv_with(curves, as_of, disc, dc)?;
    /// ```
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
/// The default implementation uses the metrics framework to compute
/// measures, delegating to `value()` for base NPV calculation.
pub trait Priceable: Send + Sync {
    /// Compute full valuation with all standard metrics (backward compatible).
    /// 
    /// Returns a complete `ValuationResult` with NPV and computed metrics
    /// appropriate for the instrument type.
    fn price(&self, curves: &CurveSet, as_of: Date) -> finstack_core::Result<super::pricing::result::ValuationResult>;
    
    /// Compute only the base present value (fast, no metrics).
    /// 
    /// Use this when you only need the NPV and don't require
    /// duration, convexity, or other risk measures.
    fn value(&self, curves: &CurveSet, as_of: Date) -> finstack_core::Result<Money> {
        // Default implementation for backward compatibility
        self.price(curves, as_of).map(|r| r.value)
    }
    
    /// Compute value with specific metrics.
    /// 
    /// # Example
    /// ```rust
    /// # use finstack_valuations::traits::Priceable;
    /// # use finstack_core::market_data::multicurve::CurveSet;
    /// # use finstack_core::dates::Date;
    /// # use finstack_valuations::metrics::MetricId;
    /// # // Note: These would be created from actual data
    /// # // let instrument: &dyn Priceable = todo!();
    /// # // let curves: &CurveSet = todo!();
    /// # let as_of = Date::from_calendar_date(2025, time::Month::January, 1).unwrap();
    /// # let metrics = [MetricId::Ytm, MetricId::DurationMac];
    /// # // let result = instrument.price_with_metrics(curves, as_of, &metrics)?;
    /// ```
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