use crate::instruments::common_impl::traits as internal_traits;
use crate::metrics::MetricId;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;

/// Lean user-facing instrument trait for desk-quant pricing workflows.
///
/// This trait exposes the public pricing surface without the internal
/// trait-object plumbing used by portfolio/scenario/registry internals.
/// All built-in Finstack instruments implement this trait automatically via
/// a blanket implementation over the internal instrument trait.
pub trait Instrument: Send + Sync {
    /// Get the instrument's unique identifier.
    fn id(&self) -> &str;

    /// Compute the present value only (fast path, no metrics).
    fn value(&self, market: &MarketContext, as_of: Date) -> finstack_core::Result<Money>;

    /// Compute present value with requested risk metrics.
    fn price_with_metrics(
        &self,
        market: &MarketContext,
        as_of: Date,
        metrics: &[MetricId],
        options: crate::instruments::PricingOptions,
    ) -> finstack_core::Result<crate::results::ValuationResult>;

    /// Get the instrument's expiry or maturity date, if applicable.
    fn expiry(&self) -> Option<Date>;
}

impl<T: internal_traits::Instrument + ?Sized> Instrument for T {
    fn id(&self) -> &str {
        internal_traits::Instrument::id(self)
    }

    fn value(&self, market: &MarketContext, as_of: Date) -> finstack_core::Result<Money> {
        internal_traits::Instrument::value(self, market, as_of)
    }

    fn price_with_metrics(
        &self,
        market: &MarketContext,
        as_of: Date,
        metrics: &[MetricId],
        options: crate::instruments::PricingOptions,
    ) -> finstack_core::Result<crate::results::ValuationResult> {
        internal_traits::Instrument::price_with_metrics(self, market, as_of, metrics, options)
    }

    fn expiry(&self) -> Option<Date> {
        internal_traits::Instrument::expiry(self)
    }
}
