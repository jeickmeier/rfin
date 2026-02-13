//! Equity types and implementations.
//!
//! Defines the `Equity` instrument shape and integrates with the standard
//! instrument macro. Pricing is delegated to `pricing::EquityPricer` and
//! metrics live under `metrics/`.

use crate::cashflow::traits::CashflowProvider;
use crate::impl_instrument_base;
use crate::instruments::common_impl::dependencies::MarketDependencies;
use crate::instruments::common_impl::traits::Attributes;
use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::scalars::MarketScalar;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};

/// Type alias for ticker symbols
pub type Ticker = String;

/// Simple equity (spot) instrument.
///
/// Represents a spot equity position that can be priced using market data.
/// The price can come from direct market quotes or be computed from
/// underlying fundamentals.
///
/// See unit tests and `examples/` for usage.
#[derive(
    Clone, Debug, finstack_valuations_macros::FinancialBuilder, serde::Serialize, serde::Deserialize,
)]
#[serde(deny_unknown_fields)]
// Note: JsonSchema derive requires finstack-core types to implement JsonSchema
// #[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
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
    pub div_yield_id: Option<String>,
    /// Discount curve ID for pricing
    pub discount_curve_id: CurveId,
    /// Attributes for scenario selection and tagging
    pub attributes: Attributes,
}

impl Equity {
    /// Create a canonical example equity for testing and documentation.
    ///
    /// Returns a 100-share position in AAPL with realistic market data IDs.
    pub fn example() -> Self {
        Self::new("EQUITY-AAPL", "AAPL", Currency::USD)
            .with_shares(100.0)
            .with_price_id("AAPL-SPOT")
            .with_dividend_yield_id("AAPL-DIV")
    }

    /// Create a new equity instrument with default 1 share
    pub fn new(id: impl Into<String>, ticker: impl Into<String>, currency: Currency) -> Self {
        let discount_curve_id = match currency {
            Currency::USD => CurveId::from("USD"),
            Currency::EUR => CurveId::from("EUR"),
            Currency::GBP => CurveId::from("GBP"),
            _ => CurveId::from("USD"), // Default fallback
        };

        Self {
            id: InstrumentId::new(id.into()),
            ticker: ticker.into(),
            currency,
            shares: None,
            price_quote: None,
            price_id: None,
            div_yield_id: None,
            discount_curve_id,
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
        self.div_yield_id = Some(div_id.into());
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

        push(self.div_yield_id.as_deref());
        push(self.attributes.get_meta("div_yield_id"));
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
        market: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<Money> {
        match scalar {
            MarketScalar::Price(m) => self.convert_price_to_currency(*m, market, as_of),
            MarketScalar::Unitless(v) => Ok(Money::new(*v, self.currency)),
        }
    }

    fn convert_price_to_currency(
        &self,
        price: Money,
        market: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<Money> {
        if price.currency() == self.currency {
            return Ok(price);
        }

        let matrix = market.fx().ok_or_else(|| {
            finstack_core::Error::from(finstack_core::InputError::NotFound {
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

        let provider = MatrixProvider { m: matrix.as_ref() };
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
                    finstack_core::Error::Input(finstack_core::InputError::NotFound { .. }) => {
                        continue;
                    }
                    _ => return Err(err),
                },
            }
        }

        Err(finstack_core::InputError::NotFound {
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
                    finstack_core::Error::Input(finstack_core::InputError::NotFound { .. }) => {
                        continue
                    }
                    _ => return Err(err),
                },
            }
        }
        Ok(0.0)
    }

    /// Calculate forward price per share using continuous-compound approximation
    pub fn forward_price_per_share(
        &self,
        market: &MarketContext,
        as_of: finstack_core::dates::Date,
        t: f64,
    ) -> finstack_core::Result<Money> {
        let s0 = self.price_per_share(market, as_of)?;
        let dy = self.dividend_yield(market)?;
        // Use configured discount curve ID
        let disc = market.get_discount(self.discount_curve_id.as_str())?;
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

impl crate::instruments::common_impl::traits::CurveDependencies for Equity {
    fn curve_dependencies(
        &self,
    ) -> finstack_core::Result<crate::instruments::common_impl::traits::InstrumentCurves> {
        crate::instruments::common_impl::traits::InstrumentCurves::builder()
            .discount(self.discount_curve_id.clone())
            .build()
    }
}

impl crate::instruments::common_impl::traits::Instrument for Equity {
    impl_instrument_base!(crate::pricer::InstrumentType::Equity);

    fn market_dependencies(&self) -> finstack_core::Result<MarketDependencies> {
        MarketDependencies::from_curve_dependencies(self)
    }

    fn value(
        &self,
        market: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        let spot_px = self.price_per_share(market, as_of)?;

        Ok(finstack_core::money::Money::new(
            spot_px.amount() * self.effective_shares(),
            self.currency,
        ))
    }

    fn effective_start_date(&self) -> Option<Date> {
        None
    }

    fn as_cashflow_provider(&self) -> Option<&dyn crate::cashflow::traits::CashflowProvider> {
        Some(self)
    }
}

impl CashflowProvider for Equity {
    fn notional(&self) -> Option<Money> {
        // Equity notional is shares * price (market value)
        // If price not quoted, return None to avoid incorrect estimation
        self.price_quote
            .map(|p| Money::new(self.effective_shares() * p, self.currency))
    }

    fn build_full_schedule(
        &self,
        _curves: &MarketContext,
        _as_of: Date,
    ) -> finstack_core::Result<crate::cashflow::builder::CashFlowSchedule> {
        Ok(crate::cashflow::traits::schedule_from_dated_flows(
            Vec::new(),
            self.notional(),
            finstack_core::dates::DayCount::Act365F, // Standard for equity spot
        ))
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
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
        let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");

        use crate::instruments::common_impl::traits::Instrument;
        let value = equity.value(&curves, as_of).expect("should succeed");
        assert_eq!(value.amount(), 15_000.0);
        assert_eq!(value.currency(), Currency::USD);
    }

    #[test]
    fn test_equity_no_cashflows() {
        let equity = Equity::new("AAPL", "AAPL", Currency::USD);
        let curves = MarketContext::new();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");

        let flows = equity
            .build_dated_flows(&curves, as_of)
            .expect("should succeed");
        assert!(flows.is_empty());
    }

    #[test]
    fn test_equity_metrics() {
        let equity = Equity::new("AAPL", "AAPL", Currency::USD)
            .with_shares(50.0)
            .with_price(200.0);

        let curves = MarketContext::new();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");

        use crate::instruments::common_impl::traits::Instrument;
        let result = equity
            .price_with_metrics(
                &curves,
                as_of,
                &[
                    crate::metrics::MetricId::EquityPricePerShare,
                    crate::metrics::MetricId::EquityShares,
                ],
            )
            .expect("should succeed");
        assert_eq!(result.value.amount(), 10_000.0); // This is the market value (PV)
        assert_eq!(result.measures.get("equity_price_per_share"), Some(&200.0));
        assert_eq!(result.measures.get("equity_shares"), Some(&50.0));
    }
}
