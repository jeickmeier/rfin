//! Equity spot instrument implementation.

use crate::impl_attributable;
use crate::metrics::MetricId;
use crate::results::ValuationResult;
use crate::traits::{Attributes, CashflowProvider, Priceable};
use finstack_core::market_data::multicurve::CurveSet;
use finstack_core::prelude::*;
use finstack_core::F;
use indexmap::IndexMap;

/// Type alias for ticker symbols
pub type Ticker = String;

/// Simple equity (spot) instrument.
///
/// Represents a spot equity position that can be priced using market data.
/// The price can come from direct market quotes or be computed from
/// underlying fundamentals.
///
/// See unit tests and `examples/` for usage.
#[derive(Clone, Debug)]
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
    /// Attributes for scenario selection and tagging
    pub attributes: Attributes,
}

impl Equity {
    /// Create a new equity builder.
    pub fn builder() -> EquityBuilder {
        EquityBuilder::new()
    }

    /// Create a new equity instrument with default 1 share
    pub fn new(id: impl Into<String>, ticker: impl Into<String>, currency: Currency) -> Self {
        Self {
            id: id.into(),
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

// Custom Priceable implementation for Equity (doesn't use standard disc_id/day_count fields)
impl Priceable for Equity {
    fn value(&self, _curves: &CurveSet, _as_of: Date) -> finstack_core::Result<Money> {
        // For equities, we need the price from market data or quote
        let price_per_share = self.price_quote.ok_or_else(|| {
            finstack_core::Error::from(finstack_core::error::InputError::NotFound)
        })?;

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
        let mut measures = IndexMap::new();

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

        Ok(ValuationResult::stamped(self.id.clone(), as_of, value).with_measures(measures))
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

// Generate standard Attributable implementation using macro
impl_attributable!(Equity);

impl From<Equity> for crate::instruments::Instrument {
    fn from(value: Equity) -> Self {
        crate::instruments::Instrument::Equity(value)
    }
}

impl std::convert::TryFrom<crate::instruments::Instrument> for Equity {
    type Error = finstack_core::Error;

    fn try_from(value: crate::instruments::Instrument) -> finstack_core::Result<Self> {
        match value {
            crate::instruments::Instrument::Equity(v) => Ok(v),
            _ => Err(finstack_core::Error::from(
                finstack_core::error::InputError::Invalid,
            )),
        }
    }
}

/// Builder pattern for Equity instruments
#[derive(Default)]
pub struct EquityBuilder {
    id: Option<String>,
    ticker: Option<String>,
    currency: Option<Currency>,
    shares: Option<F>,
    price_quote: Option<F>,
}

impl EquityBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn id(mut self, value: impl Into<String>) -> Self {
        self.id = Some(value.into());
        self
    }

    pub fn ticker(mut self, value: impl Into<String>) -> Self {
        self.ticker = Some(value.into());
        self
    }

    pub fn currency(mut self, value: Currency) -> Self {
        self.currency = Some(value);
        self
    }

    pub fn shares(mut self, value: F) -> Self {
        self.shares = Some(value);
        self
    }

    pub fn price_quote(mut self, value: F) -> Self {
        self.price_quote = Some(value);
        self
    }

    pub fn build(self) -> finstack_core::Result<Equity> {
        Ok(Equity {
            id: self.id.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
            ticker: self.ticker.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
            currency: self.currency.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
            shares: self.shares,
            price_quote: self.price_quote,
            attributes: Attributes::new(),
        })
    }
}

impl CashflowProvider for Equity {
    fn build_schedule(
        &self,
        _curves: &CurveSet,
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
