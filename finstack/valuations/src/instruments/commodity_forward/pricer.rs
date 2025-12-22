//! Commodity forward pricer engine.
//!
//! Provides deterministic PV for `CommodityForward` instruments using
//! curve-based forward price interpolation with discounting.

use crate::instruments::commodity_forward::CommodityForward;
use crate::instruments::common::traits::Instrument;
use crate::pricer::{InstrumentType, ModelKey, Pricer, PricerKey, PricingError, PricingResult};
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
        let pv = forward
            .npv(market, as_of)
            .map_err(|e| PricingError::model_failure(e.to_string()))?;

        // Return stamped result
        Ok(ValuationResult::stamped(forward.id(), as_of, pv))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pricer::Pricer;
    use finstack_core::currency::Currency;
    use finstack_core::market_data::term_structures::DiscountCurve;
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
            .settlement_date(Date::from_calendar_date(2025, Month::June, 15).expect("valid date"))
            .currency(Currency::USD)
            .quoted_price_opt(Some(75.0))
            .forward_curve_id(CurveId::new("WTI-FORWARD"))
            .discount_curve_id(CurveId::new("USD-OIS"))
            .attributes(crate::instruments::common::traits::Attributes::new())
            .build()
            .expect("should build")
    }

    fn create_test_market() -> MarketContext {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");
        let discount_curve_ois = DiscountCurve::builder("USD-OIS")
            .base_date(base_date)
            .knots(vec![(0.0, 1.0), (1.0, 0.95), (5.0, 0.80)])
            .build()
            .expect("should succeed");
        let discount_curve_wti = DiscountCurve::builder("WTI-FORWARD")
            .base_date(base_date)
            .knots(vec![(0.0, 1.0), (1.0, 0.95), (5.0, 0.80)])
            .build()
            .expect("should succeed");

        MarketContext::new()
            .insert_discount(discount_curve_ois)
            .insert_discount(discount_curve_wti)
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
        assert!(valuation.value.amount() > 0.0);
    }
}
