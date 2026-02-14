//! Commodity forward pricer engine.
//!
//! Provides deterministic PV for `CommodityForward` instruments using
//! curve-based forward price interpolation with discounting.

use crate::instruments::commodity::commodity_forward::CommodityForward;
use crate::instruments::common_impl::traits::Instrument;
use crate::pricer::{
    InstrumentType, ModelKey, Pricer, PricerKey, PricingError, PricingErrorContext, PricingResult,
};
use crate::results::ValuationResult;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;

/// Commodity forward discounting pricer.
///
/// Prices commodity forwards using the cost-of-carry model with curve-based
/// forward prices and discounting to present value.
pub struct CommodityForwardDiscountingPricer;

impl CommodityForwardDiscountingPricer {
    /// Create a new commodity forward discounting pricer.
    pub fn new() -> Self {
        Self
    }
}

impl Default for CommodityForwardDiscountingPricer {
    fn default() -> Self {
        Self::new()
    }
}

impl Pricer for CommodityForwardDiscountingPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::CommodityForward, ModelKey::Discounting)
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
        as_of: Date,
    ) -> PricingResult<ValuationResult> {
        // Type-safe downcasting
        let forward = instrument
            .as_any()
            .downcast_ref::<CommodityForward>()
            .ok_or_else(|| {
                PricingError::type_mismatch(InstrumentType::CommodityForward, instrument.key())
            })?;

        // Calculate NPV
        let pv = forward.value(market, as_of).map_err(|e| {
            PricingError::model_failure_with_context(e.to_string(), PricingErrorContext::default())
        })?;

        // Return stamped result
        Ok(ValuationResult::stamped(forward.id(), as_of, pv))
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::instruments::commodity::commodity_forward::Position;
    use crate::pricer::Pricer;
    use finstack_core::currency::Currency;
    use finstack_core::market_data::term_structures::{DiscountCurve, PriceCurve};
    use finstack_core::types::{CurveId, InstrumentId};
    use time::Month;

    fn create_test_forward() -> CommodityForward {
        CommodityForward::builder()
            .id(InstrumentId::new("TEST-FWD"))
            .commodity_type("Energy".to_string())
            .ticker("CL".to_string())
            .quantity(1000.0)
            .unit("BBL".to_string())
            .multiplier(1.0)
            .maturity(Date::from_calendar_date(2025, Month::June, 15).expect("valid date"))
            .currency(Currency::USD)
            .position(Position::Long)
            .contract_price_opt(Some(72.0)) // Entry price below market for positive MTM
            .forward_curve_id(CurveId::new("WTI-FORWARD"))
            .discount_curve_id(CurveId::new("USD-OIS"))
            .attributes(crate::instruments::common_impl::traits::Attributes::new())
            .build()
            .expect("should build")
    }

    fn create_test_market() -> MarketContext {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");

        // Create discount curve
        let discount_curve = DiscountCurve::builder("USD-OIS")
            .base_date(base_date)
            .knots(vec![(0.0, 1.0), (0.5, 0.975), (1.0, 0.95), (5.0, 0.80)])
            .build()
            .expect("should succeed");

        // Create price curve for WTI forward prices
        let price_curve = PriceCurve::builder("WTI-FORWARD")
            .base_date(base_date)
            .spot_price(75.0)
            .knots(vec![
                (0.0, 75.0),
                (0.25, 76.0),
                (0.5, 77.0),
                (1.0, 78.0),
                (2.0, 80.0),
            ])
            .build()
            .expect("should succeed");

        MarketContext::new()
            .insert_discount(discount_curve)
            .insert_price_curve(price_curve)
    }

    #[test]
    fn test_pricer_key() {
        let pricer = CommodityForwardDiscountingPricer::new();
        let key = pricer.key();

        assert_eq!(key.instrument, InstrumentType::CommodityForward);
        assert_eq!(key.model, ModelKey::Discounting);
    }

    #[test]
    fn test_forward_pricing() {
        let pricer = CommodityForwardDiscountingPricer::new();
        let forward = create_test_forward();
        let market = create_test_market();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");

        let result = pricer.price_dyn(&forward, &market, as_of);
        assert!(result.is_ok());

        let valuation = result.expect("should succeed");
        assert_eq!(valuation.instrument_id, "TEST-FWD");
        // Long position with F (~77) > K (72), so NPV should be positive
        assert!(
            valuation.value.amount() > 0.0,
            "Expected positive NPV for long with F > K, got {}",
            valuation.value.amount()
        );
    }

    #[test]
    fn test_forward_pricing_at_market() {
        let pricer = CommodityForwardDiscountingPricer::new();
        let market = create_test_market();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");

        // At-market forward (no contract_price)
        let at_market = CommodityForward::builder()
            .id(InstrumentId::new("AT-MARKET"))
            .commodity_type("Energy".to_string())
            .ticker("CL".to_string())
            .quantity(1000.0)
            .unit("BBL".to_string())
            .multiplier(1.0)
            .maturity(Date::from_calendar_date(2025, Month::June, 15).expect("valid date"))
            .currency(Currency::USD)
            .position(Position::Long)
            .forward_curve_id(CurveId::new("WTI-FORWARD"))
            .discount_curve_id(CurveId::new("USD-OIS"))
            .build()
            .expect("should build");

        let result = pricer.price_dyn(&at_market, &market, as_of);
        assert!(result.is_ok());

        let valuation = result.expect("should succeed");
        // At-market: K = F, so NPV should be ~0
        assert!(
            valuation.value.amount().abs() < 1e-10,
            "At-market NPV should be ~0, got {}",
            valuation.value.amount()
        );
    }
}
