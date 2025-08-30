//! FX Spot instrument implementation.

use crate::traits::{CashflowProvider, Priceable, Attributes};
use crate::impl_attributable;
use crate::pricing::result::ValuationResult;
use crate::metrics::MetricId;
use finstack_core::prelude::*;
use finstack_core::market_data::multicurve::CurveSet;
use finstack_core::F;
use hashbrown::HashMap;

/// FX Spot instrument (1 unit of `base` priced in `quote`).
/// 
/// Represents the spot exchange rate between two currencies.
/// The value represents how many units of the quote currency
/// are needed to buy one unit of the base currency.
/// 
/// See unit tests and `examples/` for usage.
#[derive(Clone, Debug)]
pub struct FxSpot {
    /// Unique identifier for the FX pair
    pub id: String,
    /// Base currency (the currency being priced)
    pub base: Currency,
    /// Quote currency (the currency used for pricing)
    pub quote: Currency,
    /// Optional settlement date (T+2 typically for spot)
    pub settlement: Option<Date>,
    /// Optional spot rate (if not provided, will look up from market data)
    pub spot_rate: Option<F>,
    /// Optional notional amount in base currency (defaults to 1)
    pub notional: Option<Money>,
    /// Attributes for scenario selection and tagging
    pub attributes: Attributes,
}

impl FxSpot {
    /// Create a new FX spot builder.
    pub fn builder() -> FxSpotBuilder {
        FxSpotBuilder::new()
    }
    
    /// Create a new FX spot instrument
    pub fn new(id: impl Into<String>, base: Currency, quote: Currency) -> Self {
        Self {
            id: id.into(),
            base,
            quote,
            settlement: None,
            spot_rate: None,
            notional: None,
            attributes: Attributes::new(),
        }
    }

    /// Set the settlement date
    pub fn with_settlement(mut self, date: Date) -> Self {
        self.settlement = Some(date);
        self
    }

    /// Set the spot rate
    pub fn with_rate(mut self, rate: F) -> Self {
        self.spot_rate = Some(rate);
        self
    }

    /// Set the notional amount
    pub fn with_notional(mut self, notional: Money) -> Self {
        // Ensure notional is in base currency
        if notional.currency() != self.base {
            panic!("Notional currency must match base currency");
        }
        self.notional = Some(notional);
        self
    }

    /// Get the effective notional (defaults to 1 unit of base currency)
    pub fn effective_notional(&self) -> Money {
        self.notional.unwrap_or_else(|| Money::new(1.0, self.base))
    }

    /// Standard FX pair name (e.g., "EURUSD")
    pub fn pair_name(&self) -> String {
        format!("{}{}", self.base, self.quote)
    }
}

impl Priceable for FxSpot {
    fn value(&self, _curves: &CurveSet, _as_of: Date) -> finstack_core::Result<Money> {
        // For FX spot, we need the rate from market data or quote
        let rate = self.spot_rate
            .ok_or_else(|| finstack_core::Error::from(
                finstack_core::error::InputError::NotFound
            ))?;
        
        let notional_amount = self.effective_notional().amount();
        let quote_amount = notional_amount * rate;
        
        Ok(Money::new(quote_amount, self.quote))
    }

    fn price_with_metrics(
        &self,
        curves: &CurveSet,
        as_of: Date,
        metrics: &[MetricId],
    ) -> finstack_core::Result<ValuationResult> {
        let value = self.value(curves, as_of)?;
        
        let mut measures = HashMap::new();
        
        for metric_id in metrics {
            match metric_id {
                MetricId::Custom(name) if name == "spot_rate" => {
                    let rate = self.spot_rate.unwrap_or(0.0);
                    measures.insert(name.clone(), rate);
                }
                MetricId::Custom(name) if name == "base_amount" => {
                    measures.insert(name.clone(), self.effective_notional().amount());
                }
                MetricId::Custom(name) if name == "quote_amount" => {
                    measures.insert(name.clone(), value.amount());
                }
                MetricId::Custom(name) if name == "inverse_rate" => {
                    if let Some(rate) = self.spot_rate {
                        if rate != 0.0 {
                            measures.insert(name.clone(), 1.0 / rate);
                        }
                    }
                }
                _ => {
                    // Skip metrics not applicable to FX spot
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
        // Default metrics for FX spot
        let metrics = vec![
            MetricId::custom("spot_rate"),
            MetricId::custom("base_amount"),
            MetricId::custom("quote_amount"),
            MetricId::custom("inverse_rate"),
        ];
        self.price_with_metrics(curves, as_of, &metrics)
    }
}

impl CashflowProvider for FxSpot {
    fn build_schedule(&self, _curves: &CurveSet, as_of: Date) -> finstack_core::Result<Vec<(Date, Money)>> {
        // FX spot settles on settlement date (or T+2 if not specified)
        let settle_date = self.settlement.unwrap_or_else(|| {
            // Simple T+2 calculation (should use proper business day adjustment in production)
            as_of.saturating_add(time::Duration::days(2))
        });
        
        if settle_date > as_of {
            // Future settlement
            let value = Money::new(
                self.effective_notional().amount() * self.spot_rate.unwrap_or(0.0),
                self.quote
            );
            Ok(vec![(settle_date, value)])
        } else {
            // Already settled
            Ok(vec![])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::Month;

    #[test]
    fn test_fx_spot_creation() {
        let fx = FxSpot::new("EURUSD", Currency::EUR, Currency::USD)
            .with_rate(1.08)
            .with_notional(Money::new(100_000.0, Currency::EUR));
        
        assert_eq!(fx.id, "EURUSD");
        assert_eq!(fx.base, Currency::EUR);
        assert_eq!(fx.quote, Currency::USD);
        assert_eq!(fx.spot_rate, Some(1.08));
        assert_eq!(fx.effective_notional(), Money::new(100_000.0, Currency::EUR));
    }

    #[test]
    fn test_fx_spot_default_notional() {
        let fx = FxSpot::new("GBPUSD", Currency::GBP, Currency::USD);
        assert_eq!(fx.effective_notional(), Money::new(1.0, Currency::GBP));
    }

    #[test]
    fn test_fx_spot_pair_name() {
        let fx = FxSpot::new("fx1", Currency::EUR, Currency::USD);
        assert_eq!(fx.pair_name(), "EURUSD");
    }

    #[test]
    fn test_fx_spot_valuation() {
        let fx = FxSpot::new("EURUSD", Currency::EUR, Currency::USD)
            .with_rate(1.08)
            .with_notional(Money::new(1_000_000.0, Currency::EUR));
        
        let curves = CurveSet::new();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        
        let value = fx.value(&curves, as_of).unwrap();
        assert_eq!(value.amount(), 1_080_000.0);
        assert_eq!(value.currency(), Currency::USD);
    }

    #[test]
    fn test_fx_spot_cashflow_future_settlement() {
        let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let settlement = Date::from_calendar_date(2025, Month::January, 3).unwrap();
        
        let fx = FxSpot::new("EURUSD", Currency::EUR, Currency::USD)
            .with_rate(1.08)
            .with_settlement(settlement);
        
        let curves = CurveSet::new();
        let flows = fx.build_schedule(&curves, as_of).unwrap();
        
        assert_eq!(flows.len(), 1);
        assert_eq!(flows[0].0, settlement);
        assert_eq!(flows[0].1.amount(), 1.08);
        assert_eq!(flows[0].1.currency(), Currency::USD);
    }

    #[test]
    fn test_fx_spot_cashflow_past_settlement() {
        let as_of = Date::from_calendar_date(2025, Month::January, 5).unwrap();
        let settlement = Date::from_calendar_date(2025, Month::January, 3).unwrap();
        
        let fx = FxSpot::new("EURUSD", Currency::EUR, Currency::USD)
            .with_rate(1.08)
            .with_settlement(settlement);
        
        let curves = CurveSet::new();
        let flows = fx.build_schedule(&curves, as_of).unwrap();
        
        assert!(flows.is_empty()); // Already settled
    }

    #[test]
    fn test_fx_spot_metrics() {
        let fx = FxSpot::new("EURUSD", Currency::EUR, Currency::USD)
            .with_rate(1.25)
            .with_notional(Money::new(100.0, Currency::EUR));
        
        let curves = CurveSet::new();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        
        let result = fx.price(&curves, as_of).unwrap();
        assert_eq!(result.value.amount(), 125.0);
        assert_eq!(result.measures.get("spot_rate"), Some(&1.25));
        assert_eq!(result.measures.get("base_amount"), Some(&100.0));
        assert_eq!(result.measures.get("quote_amount"), Some(&125.0));
        assert_eq!(result.measures.get("inverse_rate"), Some(&0.8));
    }

    #[test]
    #[should_panic(expected = "Notional currency must match base currency")]
    fn test_fx_spot_wrong_notional_currency() {
        FxSpot::new("EURUSD", Currency::EUR, Currency::USD)
            .with_notional(Money::new(100.0, Currency::GBP));
    }
}


#[derive(Default)]
pub struct FxSpotBuilder {
    id: Option<String>,
    base: Option<Currency>,
    quote: Option<Currency>,
    settlement: Option<Date>,
    spot_rate: Option<F>,
    notional: Option<Money>,
}

impl FxSpotBuilder {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn id(mut self, value: impl Into<String>) -> Self {
        self.id = Some(value.into());
        self
    }
    
    pub fn base(mut self, value: Currency) -> Self {
        self.base = Some(value);
        self
    }
    
    pub fn quote(mut self, value: Currency) -> Self {
        self.quote = Some(value);
        self
    }
    
    pub fn settlement(mut self, value: Date) -> Self {
        self.settlement = Some(value);
        self
    }
    
    pub fn spot_rate(mut self, value: F) -> Self {
        self.spot_rate = Some(value);
        self
    }
    
    pub fn notional(mut self, value: Money) -> Self {
        self.notional = Some(value);
        self
    }
    
    pub fn build(self) -> finstack_core::Result<FxSpot> {
        Ok(FxSpot {
            id: self.id.ok_or_else(|| finstack_core::Error::from(
                finstack_core::error::InputError::Invalid
            ))?,
            base: self.base.ok_or_else(|| finstack_core::Error::from(
                finstack_core::error::InputError::Invalid
            ))?,
            quote: self.quote.ok_or_else(|| finstack_core::Error::from(
                finstack_core::error::InputError::Invalid
            ))?,
            settlement: self.settlement,
            spot_rate: self.spot_rate,
            notional: self.notional,
            attributes: Attributes::new(),
        })
    }
}

// Generate standard Attributable implementation using macro
impl_attributable!(FxSpot);

impl From<FxSpot> for crate::instruments::Instrument {
    fn from(value: FxSpot) -> Self {
        crate::instruments::Instrument::FxSpot(value)
    }
}

impl std::convert::TryFrom<crate::instruments::Instrument> for FxSpot {
    type Error = finstack_core::Error;
    
    fn try_from(value: crate::instruments::Instrument) -> finstack_core::Result<Self> {
        match value {
            crate::instruments::Instrument::FxSpot(v) => Ok(v),
            _ => Err(finstack_core::Error::from(finstack_core::error::InputError::Invalid)),
        }
    }
}

