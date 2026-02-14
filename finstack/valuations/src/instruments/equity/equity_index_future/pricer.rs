//! Equity index future pricer engine.
//!
//! Provides deterministic PV for `EquityIndexFuture` instruments using
//! mark-to-market or cost-of-carry fair value pricing.

use crate::instruments::common_impl::traits::Instrument;
use crate::instruments::equity::equity_index_future::EquityIndexFuture;
use crate::pricer::{
    InstrumentType, ModelKey, Pricer, PricerKey, PricingError, PricingErrorContext, PricingResult,
};
use crate::results::ValuationResult;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;

/// Equity index future discounting pricer.
///
/// Prices equity index futures using:
/// 1. Mark-to-market (if quoted price available)
/// 2. Cost-of-carry fair value model (otherwise)
pub struct EquityIndexFutureDiscountingPricer;

impl EquityIndexFutureDiscountingPricer {
    /// Create a new equity index future discounting pricer.
    pub fn new() -> Self {
        Self
    }
}

impl Default for EquityIndexFutureDiscountingPricer {
    fn default() -> Self {
        Self::new()
    }
}

impl Pricer for EquityIndexFutureDiscountingPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::EquityIndexFuture, ModelKey::Discounting)
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
        as_of: Date,
    ) -> PricingResult<ValuationResult> {
        // Type-safe downcasting
        let future = instrument
            .as_any()
            .downcast_ref::<EquityIndexFuture>()
            .ok_or_else(|| {
                PricingError::type_mismatch(InstrumentType::EquityIndexFuture, instrument.key())
            })?;

        // Calculate NPV
        let pv = future.value(market, as_of).map_err(|e| {
            PricingError::model_failure_with_context(e.to_string(), PricingErrorContext::default())
        })?;

        // Return stamped result
        Ok(ValuationResult::stamped(future.id(), as_of, pv))
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::instruments::rates::ir_future::Position;
    use crate::pricer::Pricer;
    use finstack_core::currency::Currency;
    use finstack_core::market_data::scalars::MarketScalar;
    use finstack_core::market_data::term_structures::DiscountCurve;
    use finstack_core::types::{CurveId, InstrumentId};
    use time::Month;

    fn create_test_market() -> MarketContext {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");

        // Create flat 5% discount curve
        let discount_curve = DiscountCurve::builder("USD-OIS")
            .base_date(base_date)
            .knots(vec![(0.0, 1.0), (1.0, 0.9512), (2.0, 0.9048)]) // ~5% rate
            .build()
            .expect("should succeed");

        // Create market context with spot price
        MarketContext::new()
            .insert_discount(discount_curve)
            .insert_price("SPX-SPOT", MarketScalar::Unitless(4500.0))
    }

    fn create_test_future_with_quoted_price() -> EquityIndexFuture {
        use crate::instruments::common_impl::traits::Attributes;
        use crate::instruments::equity::equity_index_future::EquityFutureSpecs;

        EquityIndexFuture::builder()
            .id(InstrumentId::new("ES-TEST"))
            .underlying_ticker("SPX".to_string())
            .currency(Currency::USD)
            .quantity(10.0)
            .expiry_date(Date::from_calendar_date(2025, Month::June, 20).expect("valid date"))
            .last_trading_date(Date::from_calendar_date(2025, Month::June, 19).expect("valid date"))
            .entry_price_opt(Some(4500.0))
            .quoted_price_opt(Some(4550.0))
            .position(Position::Long)
            .contract_specs(EquityFutureSpecs::sp500_emini())
            .discount_curve_id(CurveId::new("USD-OIS"))
            .index_price_id("SPX-SPOT".to_string())
            .attributes(Attributes::new())
            .build()
            .expect("should build")
    }

    fn create_test_future_without_quoted_price() -> EquityIndexFuture {
        use crate::instruments::common_impl::traits::Attributes;
        use crate::instruments::equity::equity_index_future::EquityFutureSpecs;

        EquityIndexFuture::builder()
            .id(InstrumentId::new("ES-FAIR"))
            .underlying_ticker("SPX".to_string())
            .currency(Currency::USD)
            .quantity(10.0)
            .expiry_date(Date::from_calendar_date(2025, Month::June, 20).expect("valid date"))
            .last_trading_date(Date::from_calendar_date(2025, Month::June, 19).expect("valid date"))
            .entry_price_opt(Some(4500.0))
            .position(Position::Long)
            .contract_specs(EquityFutureSpecs::sp500_emini())
            .discount_curve_id(CurveId::new("USD-OIS"))
            .index_price_id("SPX-SPOT".to_string())
            .attributes(Attributes::new())
            .build()
            .expect("should build")
    }

    #[test]
    fn test_pricer_key() {
        let pricer = EquityIndexFutureDiscountingPricer::new();
        let key = pricer.key();

        assert_eq!(key.instrument, InstrumentType::EquityIndexFuture);
        assert_eq!(key.model, ModelKey::Discounting);
    }

    #[test]
    fn test_quoted_price_long_profit() {
        let pricer = EquityIndexFutureDiscountingPricer::new();
        let future = create_test_future_with_quoted_price();
        let market = create_test_market();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");

        let result = pricer.price_dyn(&future, &market, as_of);
        assert!(result.is_ok());

        let valuation = result.expect("should succeed");
        assert_eq!(valuation.instrument_id, "ES-TEST");

        // Long 10 contracts, entry 4500, quoted 4550
        // PV = (4550 - 4500) × 50 × 10 × 1 = 50 × 50 × 10 = 25,000
        assert!((valuation.value.amount() - 25_000.0).abs() < 0.01);
    }

    #[test]
    fn test_quoted_price_short_loss() {
        let pricer = EquityIndexFutureDiscountingPricer::new();
        let mut future = create_test_future_with_quoted_price();
        future.position = Position::Short;
        let market = create_test_market();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");

        let result = pricer.price_dyn(&future, &market, as_of);
        assert!(result.is_ok());

        let valuation = result.expect("should succeed");

        // Short 10 contracts, entry 4500, quoted 4550
        // PV = (4550 - 4500) × 50 × 10 × (-1) = -25,000
        assert!((valuation.value.amount() + 25_000.0).abs() < 0.01);
    }

    #[test]
    fn test_fair_value_pricing() {
        let pricer = EquityIndexFutureDiscountingPricer::new();
        let future = create_test_future_without_quoted_price();
        let market = create_test_market();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");

        let result = pricer.price_dyn(&future, &market, as_of);
        assert!(result.is_ok());

        let valuation = result.expect("should succeed");
        assert_eq!(valuation.instrument_id, "ES-FAIR");

        // Fair value should be positive (spot at 4500, entry at 4500, with positive carry)
        // F = 4500 × exp(0.05 × 0.47) ≈ 4607 (approximately)
        // PV = (4607 - 4500) × 50 × 10 ≈ 53,500
        assert!(valuation.value.amount() > 0.0);
    }

    #[test]
    fn test_discrete_dividends_reduce_fair_value() {
        let pricer = EquityIndexFutureDiscountingPricer::new();
        let future_no_divs = create_test_future_without_quoted_price();
        let mut future_with_divs = create_test_future_without_quoted_price();
        future_with_divs.discrete_dividends = vec![
            (
                Date::from_calendar_date(2025, Month::March, 15).expect("valid date"),
                20.0,
            ),
            (
                Date::from_calendar_date(2025, Month::May, 15).expect("valid date"),
                20.0,
            ),
        ];

        let market = create_test_market();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");

        let pv_no_divs = pricer
            .price_dyn(&future_no_divs, &market, as_of)
            .expect("pricing without discrete dividends")
            .value
            .amount();
        let pv_with_divs = pricer
            .price_dyn(&future_with_divs, &market, as_of)
            .expect("pricing with discrete dividends")
            .value
            .amount();

        assert!(
            pv_with_divs < pv_no_divs,
            "Discrete dividends should reduce fair forward and PV"
        );
    }

    #[test]
    fn test_expired_future_zero_value() {
        let pricer = EquityIndexFutureDiscountingPricer::new();
        let future = create_test_future_with_quoted_price();
        let market = create_test_market();
        // Valuation date after expiry
        let as_of = Date::from_calendar_date(2025, Month::July, 1).expect("valid date");

        let result = pricer.price_dyn(&future, &market, as_of);
        assert!(result.is_ok());

        let valuation = result.expect("should succeed");
        assert_eq!(valuation.value.amount(), 0.0);
    }
}
