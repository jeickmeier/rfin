//! Equity types and implementations.
//!
//! Defines the `Equity` instrument shape and integrates with the standard
//! instrument macro. Pricing is delegated to `pricing::EquityPricer` and
//! metrics live under `metrics/`.

use crate::cashflow::traits::CashflowProvider;
use crate::instruments::common::traits::Attributes;
use finstack_core::market_data::MarketContext;
use finstack_core::prelude::*;
use finstack_core::types::InstrumentId;
use finstack_core::F;

/// Type alias for ticker symbols
pub type Ticker = String;

/// Simple equity (spot) instrument.
///
/// Represents a spot equity position that can be priced using market data.
/// The price can come from direct market quotes or be computed from
/// underlying fundamentals.
///
/// See unit tests and `examples/` for usage.
#[derive(Clone, Debug, finstack_macros::FinancialBuilder)]
pub struct Equity {
    /// Unique identifier for the equity
    pub id: InstrumentId,
    /// Ticker symbol (e.g., "AAPL", "MSFT")
    pub ticker: Ticker,
    /// Currency in which the equity is quoted
    pub currency: Currency,
    /// Optional number of shares (defaults to 1 if not specified)
    pub shares: Option<F>,
    /// Optional price quote (if not provided, will look up from market data)
    pub price_quote: Option<F>,
    /// Attributes for scenario selection and tagging
    pub attributes: Attributes,
}

impl Equity {
    /// Create a new equity instrument with default 1 share
    pub fn new(id: impl Into<String>, ticker: impl Into<String>, currency: Currency) -> Self {
        Self {
            id: InstrumentId::new(id.into()),
            ticker: ticker.into(),
            currency,
            shares: None,
            price_quote: None,
            attributes: Attributes::new(),
        }
    }

    /// Set the number of shares
    pub fn with_shares(mut self, shares: F) -> Self {
        self.shares = Some(shares);
        self
    }

    /// Set a price quote
    pub fn with_price(mut self, price: F) -> Self {
        self.price_quote = Some(price);
        self
    }

    /// Get the effective number of shares (defaults to 1)
    pub fn effective_shares(&self) -> F {
        self.shares.unwrap_or(1.0)
    }
}

impl_instrument!(
    Equity,
    "Equity",
    pv = |s, curves, as_of| {
        let pricer = crate::instruments::equity::pricing::EquityPricer;
        pricer.pv(s, curves, as_of)
    }
);

// Conversions and Attributable provided by macro

impl CashflowProvider for Equity {
    fn build_schedule(
        &self,
        _curves: &MarketContext,
        _as_of: Date,
    ) -> finstack_core::Result<Vec<(Date, Money)>> {
        // Spot equities have no scheduled cashflows (dividends would be separate)
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::Month;

    #[test]
    fn test_equity_creation() {
        let equity = Equity::new("AAPL", "AAPL", Currency::USD)
            .with_shares(100.0)
            .with_price(150.0);

        assert_eq!(equity.id.as_str(), "AAPL");
        assert_eq!(equity.ticker, "AAPL");
        assert_eq!(equity.currency, Currency::USD);
        assert_eq!(equity.effective_shares(), 100.0);
        assert_eq!(equity.price_quote, Some(150.0));
    }

    #[test]
    fn test_equity_default_shares() {
        let equity = Equity::new("MSFT", "MSFT", Currency::USD);
        assert_eq!(equity.effective_shares(), 1.0);
    }

    #[test]
    fn test_equity_valuation() {
        let equity = Equity::new("AAPL", "AAPL", Currency::USD)
            .with_shares(100.0)
            .with_price(150.0);

        let curves = MarketContext::new();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();

        use crate::instruments::common::traits::Priceable;
        let value = Priceable::value(&equity, &curves, as_of).unwrap();
        assert_eq!(value.amount(), 15_000.0);
        assert_eq!(value.currency(), Currency::USD);
    }

    #[test]
    fn test_equity_no_cashflows() {
        let equity = Equity::new("AAPL", "AAPL", Currency::USD);
        let curves = MarketContext::new();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();

        let flows = equity.build_schedule(&curves, as_of).unwrap();
        assert!(flows.is_empty());
    }

    #[test]
    fn test_equity_metrics() {
        let equity = Equity::new("AAPL", "AAPL", Currency::USD)
            .with_shares(50.0)
            .with_price(200.0);

        let curves = MarketContext::new();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();

        use crate::instruments::common::traits::Priceable;
        let result = equity
            .price_with_metrics(
                &curves,
                as_of,
                &[
                    crate::metrics::MetricId::EquityPricePerShare,
                    crate::metrics::MetricId::EquityShares,
                    crate::metrics::MetricId::EquityMarketValue,
                ],
            )
            .unwrap();
        assert_eq!(result.value.amount(), 10_000.0);
        assert_eq!(result.measures.get("equity_price_per_share"), Some(&200.0));
        assert_eq!(result.measures.get("equity_shares"), Some(&50.0));
        assert_eq!(result.measures.get("equity_market_value"), Some(&10_000.0));
    }
}
