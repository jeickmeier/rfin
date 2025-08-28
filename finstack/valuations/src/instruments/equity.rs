//! Equity spot instrument implementation.

use crate::traits::{CashflowProvider, Priceable};
use crate::pricing::result::ValuationResult;
use crate::metrics::MetricId;
use finstack_core::prelude::*;
use finstack_core::market_data::multicurve::CurveSet;
use finstack_core::F;
use hashbrown::HashMap;

/// Type alias for ticker symbols
pub type Ticker = String;

/// Simple equity (spot) instrument.
/// 
/// Represents a spot equity position that can be priced using market data.
/// The price can come from direct market quotes or be computed from
/// underlying fundamentals.
/// 
/// # Example
/// ```rust
/// use finstack_valuations::instruments::equity::Equity;
/// use finstack_core::currency::Currency;
/// 
/// let equity = Equity {
///     id: "AAPL".to_string(),
///     ticker: "AAPL".to_string(),
///     currency: Currency::USD,
///     shares: Some(100.0),
///     price_quote: None,
/// };
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct Equity {
    /// Unique identifier for the equity
    pub id: String,
    /// Ticker symbol (e.g., "AAPL", "MSFT")
    pub ticker: Ticker,
    /// Currency in which the equity is quoted
    pub currency: Currency,
    /// Optional number of shares (defaults to 1 if not specified)
    pub shares: Option<F>,
    /// Optional price quote (if not provided, will look up from market data)
    pub price_quote: Option<F>,
}

impl Equity {
    /// Create a new equity instrument with default 1 share
    pub fn new(id: impl Into<String>, ticker: impl Into<String>, currency: Currency) -> Self {
        Self {
            id: id.into(),
            ticker: ticker.into(),
            currency,
            shares: None,
            price_quote: None,
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

impl Priceable for Equity {
    fn value(&self, _curves: &CurveSet, _as_of: Date) -> finstack_core::Result<Money> {
        // For equities, we need the price from market data or quote
        let price_per_share = self.price_quote
            .ok_or_else(|| finstack_core::Error::from(
                finstack_core::error::InputError::NotFound
            ))?;
        
        let total_value = price_per_share * self.effective_shares();
        Ok(Money::new(total_value, self.currency))
    }

    fn price_with_metrics(
        &self,
        curves: &CurveSet,
        as_of: Date,
        metrics: &[MetricId],
    ) -> finstack_core::Result<ValuationResult> {
        let value = self.value(curves, as_of)?;
        
        // Equities have limited metrics - mainly just the spot price
        let mut measures = HashMap::new();
        
        for metric_id in metrics {
            match metric_id {
                MetricId::Custom(name) if name == "price_per_share" => {
                    let price = self.price_quote.unwrap_or(0.0);
                    measures.insert(name.clone(), price);
                }
                MetricId::Custom(name) if name == "shares" => {
                    measures.insert(name.clone(), self.effective_shares());
                }
                MetricId::Custom(name) if name == "market_value" => {
                    measures.insert(name.clone(), value.amount());
                }
                _ => {
                    // Skip metrics not applicable to equities
                }
            }
        }
        
        Ok(ValuationResult::stamped(
            self.id.clone(),
            as_of,
            value,
        ).with_measures(measures))
    }

    fn price(&self, curves: &CurveSet, as_of: Date) -> finstack_core::Result<ValuationResult> {
        // Default metrics for equities
        let metrics = vec![
            MetricId::custom("price_per_share"),
            MetricId::custom("shares"),
            MetricId::custom("market_value"),
        ];
        self.price_with_metrics(curves, as_of, &metrics)
    }
}

impl CashflowProvider for Equity {
    fn build_schedule(&self, _curves: &CurveSet, _as_of: Date) -> finstack_core::Result<Vec<(Date, Money)>> {
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
        
        assert_eq!(equity.id, "AAPL");
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
        
        let curves = CurveSet::new();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        
        let value = equity.value(&curves, as_of).unwrap();
        assert_eq!(value.amount(), 15_000.0);
        assert_eq!(value.currency(), Currency::USD);
    }

    #[test]
    fn test_equity_no_cashflows() {
        let equity = Equity::new("AAPL", "AAPL", Currency::USD);
        let curves = CurveSet::new();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        
        let flows = equity.build_schedule(&curves, as_of).unwrap();
        assert!(flows.is_empty());
    }

    #[test]
    fn test_equity_metrics() {
        let equity = Equity::new("AAPL", "AAPL", Currency::USD)
            .with_shares(50.0)
            .with_price(200.0);
        
        let curves = CurveSet::new();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        
        let result = equity.price(&curves, as_of).unwrap();
        assert_eq!(result.value.amount(), 10_000.0);
        assert_eq!(result.measures.get("price_per_share"), Some(&200.0));
        assert_eq!(result.measures.get("shares"), Some(&50.0));
        assert_eq!(result.measures.get("market_value"), Some(&10_000.0));
    }
}
