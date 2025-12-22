//! Unified pricer for all structured credit instruments.
//!
//! The pricing logic is identical across ABS, CLO, CMBS, and RMBS since they all
//! use the shared waterfall implementation via the `StructuredCreditInstrument` trait.
//!
//! # Hedge Swap Integration
//!
//! When hedge swaps are attached to a deal, they are valued alongside the
//! collateral cashflows to provide a hedged NPV. This is important for:
//! - Basis risk management (e.g., SOFR vs Prime mismatches)
//! - Interest rate risk hedging via fixed-for-floating swaps
//! - Cap/floor protection embedded in structures

use super::StructuredCredit;
use crate::cashflow::traits::CashflowProvider;
use crate::instruments::common::discountable::Discountable;
use crate::metrics::MetricId;
use crate::results::ValuationResult;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;

impl StructuredCredit {
    /// Value the structured credit instrument using discounted cashflow analysis.
    ///
    /// This method generates cashflows through the waterfall engine and discounts
    /// them back to present value using the instrument's discount curve.
    ///
    /// **Note**: This returns only the unhedged deal NPV. Use `price_with_hedges()`
    /// for combined deal + hedge valuation.
    pub fn price(&self, context: &MarketContext, as_of: Date) -> finstack_core::Result<Money> {
        let disc = context.get_discount_ref(self.discount_curve_id.as_str())?;
        let flows = self.build_schedule(context, as_of)?;

        let day_count = disc.day_count();
        flows.npv(disc, as_of, day_count)
    }

    /// Value the total hedge swap portfolio.
    ///
    /// Returns the net present value of all attached hedge swaps.
    /// This represents the mark-to-market value of the hedging portfolio.
    pub fn hedge_npv(&self, context: &MarketContext, as_of: Date) -> finstack_core::Result<Money> {
        let base_ccy = self.pool.base_currency();
        let mut total_hedge_npv = Money::new(0.0, base_ccy);

        for swap in &self.hedge_swaps {
            // Use the IRS pricer to value each swap
            let swap_npv = crate::instruments::irs::pricer::npv(swap, context, as_of)?;

            // Convert to deal currency if needed (simplified - assumes same currency)
            total_hedge_npv = total_hedge_npv.checked_add(swap_npv)?;
        }

        Ok(total_hedge_npv)
    }

    /// Value the deal plus all hedge swaps (hedged NPV).
    ///
    /// This is the primary valuation method for hedged portfolios, computing:
    /// ```text
    /// Hedged NPV = Deal NPV + Hedge NPV
    /// ```
    ///
    /// # Returns
    /// - `Ok((deal_npv, hedge_npv, total_npv))` on success
    /// - Error if valuation fails
    ///
    /// # Example
    /// ```rust,no_run
    /// use finstack_core::market_data::context::MarketContext;
    /// use finstack_valuations::instruments::structured_credit::StructuredCredit;
    /// use time::macros::date;
    ///
    /// # fn main() -> finstack_core::Result<()> {
    /// let clo = StructuredCredit::example();
    /// let context = MarketContext::new();
    /// let as_of = date!(2025-01-01);
    ///
    /// let (deal, hedges, total) = clo.price_with_hedges(&context, as_of)?;
    /// println!(
    ///     "Deal NPV: {:.2}, Hedge NPV: {:.2}, Total: {:.2}",
    ///     deal.amount(),
    ///     hedges.amount(),
    ///     total.amount()
    /// );
    /// # Ok(())
    /// # }
    /// ```
    pub fn price_with_hedges(
        &self,
        context: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<(Money, Money, Money)> {
        let deal_npv = self.price(context, as_of)?;
        let hedge_npv = self.hedge_npv(context, as_of)?;
        let total_npv = deal_npv.checked_add(hedge_npv)?;

        Ok((deal_npv, hedge_npv, total_npv))
    }

    /// Check if this deal has any hedge swaps attached.
    pub fn has_hedges(&self) -> bool {
        !self.hedge_swaps.is_empty()
    }

    /// Get the number of hedge swaps attached to this deal.
    pub fn hedge_count(&self) -> usize {
        self.hedge_swaps.len()
    }

    /// Price with additional risk metrics.
    ///
    /// Computes the base NPV plus any requested metrics such as duration, spread, etc.
    /// If hedge swaps are attached, also includes hedge NPV in the results.
    pub fn price_with_metrics_standalone(
        &self,
        context: &MarketContext,
        as_of: Date,
        metrics: &[MetricId],
    ) -> finstack_core::Result<ValuationResult> {
        let base_value = self.price(context, as_of)?;

        if metrics.is_empty() && self.hedge_swaps.is_empty() {
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

        // Add hedge metrics if swaps are attached
        if !self.hedge_swaps.is_empty() {
            let hedge_npv = self.hedge_npv(context, as_of)?;
            let total_npv = base_value.checked_add(hedge_npv)?;

            result
                .measures
                .insert("HedgeNPV".to_string(), hedge_npv.amount());
            result
                .measures
                .insert("HedgedNPV".to_string(), total_npv.amount());
            result
                .measures
                .insert("HedgeCount".to_string(), self.hedge_swaps.len() as f64);
        }

        Ok(result)
    }

    /// Add a hedge swap to this instrument.
    ///
    /// The swap will be valued alongside the deal for hedged NPV calculations.
    pub fn add_hedge_swap(&mut self, swap: crate::instruments::irs::InterestRateSwap) {
        self.hedge_swaps.push(swap);
    }

    /// Add multiple hedge swaps to this instrument.
    pub fn add_hedge_swaps(&mut self, swaps: Vec<crate::instruments::irs::InterestRateSwap>) {
        self.hedge_swaps.extend(swaps);
    }

    /// Builder method to add hedge swap (chainable).
    pub fn with_hedge_swap(mut self, swap: crate::instruments::irs::InterestRateSwap) -> Self {
        self.hedge_swaps.push(swap);
        self
    }

    /// Builder method to add multiple hedge swaps (chainable).
    pub fn with_hedge_swaps(
        mut self,
        swaps: Vec<crate::instruments::irs::InterestRateSwap>,
    ) -> Self {
        self.hedge_swaps.extend(swaps);
        self
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
