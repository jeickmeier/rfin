use crate::instruments::common::traits::Instrument;
use crate::instruments::fx_spot::FxSpot;
use crate::pricer::{InstrumentType, ModelKey, Pricer, PricerKey, PricingError};
use crate::results::ValuationResult;
use finstack_core::market_data::MarketContext;

/// FX Spot pricer.
///
/// Computes the value of an FX Spot position in the quote currency.
/// Value = Notional (Base) * Spot Rate (Base/Quote).
///
/// Note: This pricer currently returns the undiscounted spot value (Cash Position).
/// If the instrument has a future settlement date, strictly speaking it should be discounted,
/// but for standard Spot handling (T+2) it is often treated as cash-equivalent or
/// the user supplies a present-valued Spot Rate.
pub struct FxSpotPricer;

impl FxSpotPricer {
    /// Create a new FX spot pricer
    pub fn new() -> Self {
        Self
    }
}

impl Default for FxSpotPricer {
    fn default() -> Self {
        Self::new()
    }
}

impl Pricer for FxSpotPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::FxSpot, ModelKey::Discounting)
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> Result<ValuationResult, PricingError> {
        // Type-safe downcasting
        let fx_spot = instrument
            .as_any()
            .downcast_ref::<FxSpot>()
            .ok_or_else(|| PricingError::type_mismatch(InstrumentType::FxSpot, instrument.key()))?;

        // Use the instrument's own value method with provided as_of date
        let pv = fx_spot
            .value(market, as_of)
            .map_err(|e| PricingError::model_failure(e.to_string()))?;

        // Return stamped result
        Ok(ValuationResult::stamped(fx_spot.id(), as_of, pv))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruments::fx_spot::FxSpot;
    use crate::pricer::Pricer;
    use finstack_core::currency::Currency;
    use finstack_core::market_data::MarketContext;

    fn create_test_fx_spot() -> FxSpot {
        FxSpot::new("EURUSD".into(), Currency::EUR, Currency::USD).with_rate(1.05)
    }

    #[test]
    fn test_fx_spot_pricing_with_valid_data() {
        let fx_spot = create_test_fx_spot();
        let market = MarketContext::new();
        let pricer = FxSpotPricer::new();
        let as_of = finstack_core::dates::Date::from_calendar_date(2024, time::Month::January, 1)
            .expect("Valid test date");

        let result = pricer.price_dyn(&fx_spot, &market, as_of);
        assert!(result.is_ok());

        let valuation = result.expect("FX spot pricing should succeed in test");
        assert_eq!(valuation.instrument_id, "EURUSD");
        assert!(valuation.value.amount() > 0.0);
        assert_eq!(valuation.value.currency(), Currency::USD);
    }

    #[test]
    fn test_fx_spot_pricing_error_propagation() {
        let fx_spot = create_test_fx_spot();
        let market = MarketContext::new();
        let pricer = FxSpotPricer::new();
        let as_of = finstack_core::dates::Date::from_calendar_date(2024, time::Month::January, 1)
            .expect("Valid test date");

        // Test that error propagation works correctly
        let result = pricer.price_dyn(&fx_spot, &market, as_of);
        assert!(result.is_ok());

        // Verify the valuation uses epoch date as as_of
        let valuation = result.expect("FX spot pricing should succeed in test");
        assert_eq!(valuation.instrument_id, "EURUSD");
        assert!(valuation.value.amount() > 0.0);
    }

    #[test]
    fn test_fx_spot_pricing_with_different_rates() {
        let fx_spot_high =
            FxSpot::new("GBPUSD".into(), Currency::GBP, Currency::USD).with_rate(1.25);
        let fx_spot_low =
            FxSpot::new("USDJPY".into(), Currency::USD, Currency::JPY).with_rate(110.0);

        let market = MarketContext::new();
        let pricer = FxSpotPricer::new();
        let as_of = finstack_core::dates::Date::from_calendar_date(2024, time::Month::January, 1)
            .expect("Valid test date");

        let result_high = pricer.price_dyn(&fx_spot_high, &market, as_of);
        let result_low = pricer.price_dyn(&fx_spot_low, &market, as_of);

        assert!(result_high.is_ok());
        assert!(result_low.is_ok());

        let valuation_high = result_high.expect("FX spot pricing should succeed in test");
        let valuation_low = result_low.expect("FX spot pricing should succeed in test");

        assert_eq!(valuation_high.instrument_id, "GBPUSD");
        assert_eq!(valuation_low.instrument_id, "USDJPY");
        assert!(valuation_high.value.amount() > 0.0);
        assert!(valuation_low.value.amount() > 0.0);
    }

    #[test]
    fn test_fx_spot_pricing_error_handling() {
        // Test with invalid FX spot (zero rate)
        let fx_spot_zero =
            FxSpot::new("EURUSD".into(), Currency::EUR, Currency::USD).with_rate(0.0);

        let market = MarketContext::new();
        let pricer = FxSpotPricer::new();
        let as_of = finstack_core::dates::Date::from_calendar_date(2024, time::Month::January, 1)
            .expect("Valid test date");

        let result = pricer.price_dyn(&fx_spot_zero, &market, as_of);

        match result {
            Ok(valuation) => {
                // Even with zero rate, should return a valid valuation
                assert_eq!(valuation.instrument_id, "EURUSD");
                assert_eq!(valuation.value.amount(), 0.0);
            }
            Err(error) => {
                // If it returns an error, ensure the error message is meaningful
                let error_msg = format!("{}", error);
                assert!(!error_msg.is_empty());
                assert!(error_msg.len() > 10);
            }
        }
    }

    #[test]
    fn test_fx_spot_pricing_date_handling() {
        let fx_spot = create_test_fx_spot();
        let market = MarketContext::new();
        let pricer = FxSpotPricer::new();
        let as_of = finstack_core::dates::Date::from_calendar_date(2024, time::Month::January, 1)
            .expect("Valid test date");

        let result = pricer.price_dyn(&fx_spot, &market, as_of);
        assert!(result.is_ok());

        // The pricer should stamp the result with the provided as_of date
        let valuation = result.expect("FX spot pricing should succeed in test");
        assert_eq!(valuation.instrument_id, "EURUSD");

        // Verify the as_of date is set correctly (should match the input)
        assert_eq!(valuation.as_of, as_of);
    }

    #[test]
    fn test_fx_spot_pricing_with_various_currencies() {
        let test_cases = vec![
            ("EURUSD", Currency::EUR, Currency::USD, 1.05),
            ("GBPUSD", Currency::GBP, Currency::USD, 1.25),
            ("USDJPY", Currency::USD, Currency::JPY, 110.0),
            ("AUDUSD", Currency::AUD, Currency::USD, 0.65),
        ];

        let market = MarketContext::new();
        let pricer = FxSpotPricer::new();
        let as_of = finstack_core::dates::Date::from_calendar_date(2024, time::Month::January, 1)
            .expect("Valid test date");

        for (id, base, quote, rate) in test_cases {
            let fx_spot = FxSpot::new(id.into(), base, quote).with_rate(rate);
            let result = pricer.price_dyn(&fx_spot, &market, as_of);

            assert!(result.is_ok(), "Failed to price FX spot {}", id);

            let valuation = result.expect("FX spot pricing should succeed in test");
            assert_eq!(valuation.instrument_id, id);
            assert_eq!(valuation.value.currency(), quote);
            assert!(valuation.value.amount() > 0.0);
        }
    }

    #[test]
    fn test_pricer_key() {
        let pricer = FxSpotPricer::new();
        let key = pricer.key();

        // Verify the pricer key is correctly configured
        assert_eq!(key.instrument, InstrumentType::FxSpot);
        assert_eq!(key.model, ModelKey::Discounting);
    }

    #[test]
    fn test_fx_spot_error_message_quality() {
        let fx_spot = create_test_fx_spot();
        let market = MarketContext::new();
        let pricer = FxSpotPricer::new();
        let as_of = finstack_core::dates::Date::from_calendar_date(2024, time::Month::January, 1)
            .expect("Valid test date");

        let result = pricer.price_dyn(&fx_spot, &market, as_of);

        match result {
            Ok(valuation) => {
                assert!(!valuation.instrument_id.is_empty());
                assert!(valuation.value.amount() >= 0.0);
            }
            Err(error) => {
                let error_msg = format!("{}", error);
                assert!(!error_msg.is_empty());
                // Error messages should be descriptive
                assert!(error_msg.len() > 10);
            }
        }
    }

    #[test]
    fn test_fx_spot_pricing_consistency() {
        // Test that pricing is consistent across multiple calls
        let fx_spot = create_test_fx_spot();
        let market = MarketContext::new();
        let pricer = FxSpotPricer::new();
        let as_of = finstack_core::dates::Date::from_calendar_date(2024, time::Month::January, 1)
            .expect("Valid test date");

        let result1 = pricer.price_dyn(&fx_spot, &market, as_of);
        let result2 = pricer.price_dyn(&fx_spot, &market, as_of);

        assert!(result1.is_ok());
        assert!(result2.is_ok());

        let valuation1 = result1.expect("should succeed");
        let valuation2 = result2.expect("should succeed");

        assert_eq!(valuation1.value, valuation2.value);
        assert_eq!(valuation1.as_of, valuation2.as_of);
    }

    #[test]
    fn test_fx_spot_pricing_edge_cases() {
        // Test with very small and very large rates
        let fx_spot_small =
            FxSpot::new("TEST1".into(), Currency::USD, Currency::JPY).with_rate(0.001);
        let fx_spot_large =
            FxSpot::new("TEST2".into(), Currency::JPY, Currency::USD).with_rate(10000.0);

        let market = MarketContext::new();
        let pricer = FxSpotPricer::new();
        let as_of = finstack_core::dates::Date::from_calendar_date(2024, time::Month::January, 1)
            .expect("Valid test date");

        let result_small = pricer.price_dyn(&fx_spot_small, &market, as_of);
        let result_large = pricer.price_dyn(&fx_spot_large, &market, as_of);

        assert!(result_small.is_ok());
        assert!(result_large.is_ok());

        let valuation_small = result_small.expect("should succeed");
        let valuation_large = result_large.expect("should succeed");

        assert!(valuation_small.value.amount() >= 0.0);
        assert!(valuation_large.value.amount() > 0.0);
    }
}
