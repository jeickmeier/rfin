//! Equity types and implementations.
//!
//! Defines the `Equity` instrument shape and integrates with the standard
//! instrument macro. Pricing is delegated to `pricing::EquityPricer` and
//! metrics live under `metrics/`.

use crate::cashflow::traits::CashflowProvider;
use crate::instruments::common::traits::Attributes;
use finstack_core::market_data::scalars::MarketScalar;
use finstack_core::market_data::MarketContext;
use finstack_core::prelude::*;
use finstack_core::types::InstrumentId;

/// Type alias for ticker symbols
pub type Ticker = String;

/// Simple equity (spot) instrument.
///
/// Represents a spot equity position that can be priced using market data.
/// The price can come from direct market quotes or be computed from
/// underlying fundamentals.
///
/// See unit tests and `examples/` for usage.
#[derive(Clone, Debug, finstack_valuations_macros::FinancialBuilder)]
pub struct Equity {
    /// Unique identifier for the equity
    pub id: InstrumentId,
    /// Ticker symbol (e.g., "AAPL", "MSFT")
    pub ticker: Ticker,
    /// Currency in which the equity is quoted
    pub currency: Currency,
    /// Optional number of shares (defaults to 1 if not specified)
    pub shares: Option<f64>,
    /// Optional price quote (if not provided, will look up from market data)
    pub price_quote: Option<f64>,
    /// Explicit market data identifier to resolve the spot price
    pub price_id: Option<String>,
    /// Explicit market data identifier to resolve the dividend yield
    pub dividend_yield_id: Option<String>,
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
            price_id: None,
            dividend_yield_id: None,
            attributes: Attributes::new(),
        }
    }

    /// Set the number of shares
    pub fn with_shares(mut self, shares: f64) -> Self {
        self.shares = Some(shares);
        self
    }

    /// Set a price quote
    pub fn with_price(mut self, price: f64) -> Self {
        self.price_quote = Some(price);
        self
    }

    /// Override the market data identifier used to resolve the spot price
    pub fn with_price_id(mut self, price_id: impl Into<String>) -> Self {
        self.price_id = Some(price_id.into());
        self
    }

    /// Override the market data identifier used to resolve the dividend yield
    pub fn with_dividend_yield_id(mut self, div_id: impl Into<String>) -> Self {
        self.dividend_yield_id = Some(div_id.into());
        self
    }

    fn price_id_candidates(&self) -> Vec<String> {
        let mut ids: Vec<String> = Vec::new();
        let mut push = |candidate: Option<&str>| {
            if let Some(value) = candidate {
                if !value.is_empty() && !ids.iter().any(|existing| existing == value) {
                    ids.push(value.to_string());
                }
            }
        };

        push(self.price_id.as_deref());
        push(self.attributes.get_meta("price_id"));
        push(self.attributes.get_meta("spot_id"));
        push(self.attributes.get_meta("market_price_id"));
        push(Some(self.ticker.as_str()));
        push(Some(self.id.as_str()));
        let ticker_spot = format!("{}-SPOT", self.ticker);
        push(Some(ticker_spot.as_str()));
        let id_spot = format!("{}-SPOT", self.id.as_str());
        push(Some(id_spot.as_str()));
        push(Some("EQUITY-SPOT"));

        ids
    }

    fn dividend_yield_id_candidates(&self) -> Vec<String> {
        let mut ids: Vec<String> = Vec::new();
        let mut push = |candidate: Option<&str>| {
            if let Some(value) = candidate {
                if !value.is_empty() && !ids.iter().any(|existing| existing == value) {
                    ids.push(value.to_string());
                }
            }
        };

        push(self.dividend_yield_id.as_deref());
        push(self.attributes.get_meta("dividend_yield_id"));
        push(self.attributes.get_meta("dividend_yield_key"));
        push(self.attributes.get_meta("div_yield_id"));
        let ticker_div = format!("{}-DIVYIELD", self.ticker);
        push(Some(ticker_div.as_str()));
        let id_div = format!("{}-DIVYIELD", self.id.as_str());
        push(Some(id_div.as_str()));
        push(Some("EQUITY-DIVYIELD"));

        ids
    }

    fn money_from_scalar(
        &self,
        scalar: &MarketScalar,
        curves: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<Money> {
        match scalar {
            MarketScalar::Price(m) => self.convert_price_to_currency(*m, curves, as_of),
            MarketScalar::Unitless(v) => Ok(Money::new(*v, self.currency)),
        }
    }

    fn convert_price_to_currency(
        &self,
        price: Money,
        curves: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<Money> {
        if price.currency() == self.currency {
            return Ok(price);
        }

        let matrix = curves.fx.as_ref().ok_or_else(|| {
            finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                id: "fx_matrix".to_string(),
            })
        })?;

        struct MatrixProvider<'a> {
            m: &'a finstack_core::money::fx::FxMatrix,
        }
        impl finstack_core::money::fx::FxProvider for MatrixProvider<'_> {
            fn rate(
                &self,
                from: finstack_core::currency::Currency,
                to: finstack_core::currency::Currency,
                on: finstack_core::dates::Date,
                policy: finstack_core::money::fx::FxConversionPolicy,
            ) -> finstack_core::Result<finstack_core::money::fx::FxRate> {
                let r = self.m.rate(finstack_core::money::fx::FxQuery::with_policy(
                    from, to, on, policy,
                ))?;
                Ok(r.rate)
            }
        }

        let provider = MatrixProvider { m: matrix };
        price.convert(
            self.currency,
            as_of,
            &provider,
            finstack_core::money::fx::FxConversionPolicy::CashflowDate,
        )
    }

    /// Get the effective number of shares (defaults to 1)
    pub fn effective_shares(&self) -> f64 {
        self.shares.unwrap_or(1.0)
    }

    /// Calculate the net present value of this equity position
    pub fn npv(
        &self,
        curves: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<Money> {
        let px = self.price_per_share(curves, as_of)?;
        Ok(Money::new(
            px.amount() * self.effective_shares(),
            self.currency,
        ))
    }

    /// Resolve price per share for the equity
    pub fn price_per_share(
        &self,
        curves: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<Money> {
        if let Some(px) = self.price_quote {
            return Ok(Money::new(px, self.currency));
        }

        let candidates = self.price_id_candidates();
        for key in &candidates {
            match curves.price(key) {
                Ok(scalar) => {
                    return self.money_from_scalar(scalar, curves, as_of);
                }
                Err(err) => match err {
                    finstack_core::Error::Input(finstack_core::error::InputError::NotFound {
                        ..
                    }) => {
                        continue;
                    }
                    _ => return Err(err),
                },
            }
        }

        Err(finstack_core::error::InputError::NotFound {
            id: format!("equity price (candidates: {})", candidates.join(", ")),
        }
        .into())
    }

    /// Resolve dividend yield (annualized, decimal) for the equity
    pub fn dividend_yield(&self, curves: &MarketContext) -> finstack_core::Result<f64> {
        let candidates = self.dividend_yield_id_candidates();
        for key in &candidates {
            match curves.price(key) {
                Ok(MarketScalar::Unitless(v)) => return Ok(*v),
                Ok(MarketScalar::Price(_)) => continue,
                Err(err) => match err {
                    finstack_core::Error::Input(finstack_core::error::InputError::NotFound {
                        ..
                    }) => continue,
                    _ => return Err(err),
                },
            }
        }
        Ok(0.0)
    }

    /// Calculate forward price per share using continuous-compound approximation
    pub fn forward_price_per_share(
        &self,
        curves: &MarketContext,
        as_of: finstack_core::dates::Date,
        t: f64,
    ) -> finstack_core::Result<Money> {
        let s0 = self.price_per_share(curves, as_of)?;
        let dy = self.dividend_yield(curves)?;
        let discount_id = format!("{}-OIS", self.currency);
        let disc = curves.get_discount_ref(&discount_id)?;
        let r = disc.zero(t);
        let fwd = s0.amount() * ((r - dy) * t).exp();
        Ok(Money::new(fwd, self.currency))
    }

    /// Calculate forward total value for the position
    pub fn forward_value(
        &self,
        curves: &MarketContext,
        as_of: finstack_core::dates::Date,
        t: f64,
    ) -> finstack_core::Result<Money> {
        let per_share = self.forward_price_per_share(curves, as_of, t)?;
        Ok(Money::new(
            per_share.amount() * self.effective_shares(),
            self.currency,
        ))
    }
}

impl_instrument!(
    Equity,
    crate::pricer::InstrumentType::Equity,
    "Equity",
    pv = |s, curves, as_of| {
        // Call the instrument's own NPV method
        s.npv(curves, as_of)
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

        use crate::instruments::common::traits::Instrument;
        let value = equity.value(&curves, as_of).unwrap();
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

        use crate::instruments::common::traits::Instrument;
        let result = equity
            .price_with_metrics(
                &curves,
                as_of,
                &[
                    crate::metrics::MetricId::EquityPricePerShare,
                    crate::metrics::MetricId::EquityShares,
                ],
            )
            .unwrap();
        assert_eq!(result.value.amount(), 10_000.0); // This is the market value (PV)
        assert_eq!(result.measures.get("equity_price_per_share"), Some(&200.0));
        assert_eq!(result.measures.get("equity_shares"), Some(&50.0));
    }
}
