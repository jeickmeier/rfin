//! Commodity swap pricer engine.
//!
//! Provides deterministic PV for `CommoditySwap` instruments using
//! fixed and floating leg calculations with discounting.

use crate::instruments::commodity_swap::CommoditySwap;
use crate::instruments::common::traits::Instrument;
use crate::pricer::{
    InstrumentType, ModelKey, Pricer, PricerKey, PricingError, PricingErrorContext, PricingResult,
};
use crate::results::ValuationResult;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;

/// Commodity swap discounting pricer.
///
/// Prices commodity swaps by calculating the present value of the fixed
/// and floating legs, with the final NPV based on the pay/receive direction.
pub struct CommoditySwapDiscountingPricer;

impl CommoditySwapDiscountingPricer {
    /// Create a new commodity swap discounting pricer.
    pub fn new() -> Self {
        Self
    }
}

impl Default for CommoditySwapDiscountingPricer {
    fn default() -> Self {
        Self::new()
    }
}

impl Pricer for CommoditySwapDiscountingPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::CommoditySwap, ModelKey::Discounting)
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
        as_of: Date,
    ) -> PricingResult<ValuationResult> {
        // Type-safe downcasting
        let swap = instrument
            .as_any()
            .downcast_ref::<CommoditySwap>()
            .ok_or_else(|| {
                PricingError::type_mismatch(InstrumentType::CommoditySwap, instrument.key())
            })?;

        // Calculate NPV
        let pv = swap.npv(market, as_of).map_err(|e| {
            PricingError::model_failure_ctx(e.to_string(), PricingErrorContext::default())
        })?;

        // Return stamped result
        Ok(ValuationResult::stamped(swap.id(), as_of, pv))
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::pricer::Pricer;
    use finstack_core::currency::Currency;
    use finstack_core::dates::{Tenor, TenorUnit};
    use finstack_core::market_data::term_structures::{DiscountCurve, PriceCurve};
    use finstack_core::types::{CurveId, InstrumentId};
    use time::Month;

    fn create_test_swap() -> CommoditySwap {
        CommoditySwap::builder()
            .id(InstrumentId::new("TEST-SWAP"))
            .commodity_type("Energy".to_string())
            .ticker("NG".to_string())
            .unit("MMBTU".to_string())
            .currency(Currency::USD)
            .notional_quantity(10000.0)
            .fixed_price(3.50)
            .floating_index_id(CurveId::new("NG-SPOT-AVG"))
            .pay_fixed(true)
            .start_date(Date::from_calendar_date(2025, Month::January, 1).expect("valid date"))
            .end_date(Date::from_calendar_date(2025, Month::June, 30).expect("valid date"))
            .payment_frequency(Tenor::new(1, TenorUnit::Months))
            .discount_curve_id(CurveId::new("USD-OIS"))
            .attributes(crate::instruments::common::traits::Attributes::new())
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

        // Create price curve for NG forward prices
        let price_curve = PriceCurve::builder("NG-SPOT-AVG")
            .base_date(base_date)
            .spot_price(3.50)
            .knots(vec![(0.0, 3.50), (0.25, 3.55), (0.5, 3.60), (1.0, 3.70)])
            .build()
            .expect("should succeed");

        MarketContext::new()
            .insert_discount(discount_curve)
            .insert_price_curve(price_curve)
    }

    #[test]
    fn test_pricer_key() {
        let pricer = CommoditySwapDiscountingPricer::new();
        let key = pricer.key();

        assert_eq!(key.instrument, InstrumentType::CommoditySwap);
        assert_eq!(key.model, ModelKey::Discounting);
    }

    #[test]
    fn test_swap_pricing() {
        let pricer = CommoditySwapDiscountingPricer::new();
        let swap = create_test_swap();
        let market = create_test_market();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");

        let result = pricer.price_dyn(&swap, &market, as_of);
        assert!(result.is_ok(), "Pricing failed: {:?}", result.err());

        let valuation = result.expect("should succeed");
        assert_eq!(valuation.instrument_id, "TEST-SWAP");

        // In contango (forward > spot), pay-fixed should have positive NPV
        // because floating leg receives higher forward prices
        assert!(
            valuation.value.amount() > 0.0,
            "Pay-fixed swap in contango should have positive NPV, got {}",
            valuation.value.amount()
        );
    }
}
