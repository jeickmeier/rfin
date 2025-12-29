//! Dollar roll pricing.
//!
//! Dollar roll value is the net of the two TBA legs plus any
//! implied carry benefit or cost.

use super::DollarRoll;
use crate::instruments::agency_tba::pricer::price_tba;
use crate::pricer::{InstrumentType, ModelKey, Pricer, PricerKey, PricingError, PricingResult};
use crate::results::ValuationResult;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_core::Result;

/// Price a dollar roll.
///
/// The dollar roll value is computed as:
/// - Front leg: Short TBA (receive price at front settlement)
/// - Back leg: Long TBA (pay price at back settlement)
///
/// Net value = Front leg value - Back leg value
///
/// Positive value means the roll is profitable (you receive more
/// from the front sale than you pay for the back purchase).
pub fn price_dollar_roll(roll: &DollarRoll, market: &MarketContext, as_of: Date) -> Result<Money> {
    let front_leg = roll.front_leg()?;
    let back_leg = roll.back_leg()?;

    // Price each leg
    let front_value = price_tba(&front_leg, market, as_of)?;
    let back_value = price_tba(&back_leg, market, as_of)?;

    // For the roll:
    // - We're short the front leg (receive proceeds)
    // - We're long the back leg (pay proceeds)
    // So net value = -front_value - back_value (negate both since TBA prices are
    // from buyer perspective)

    // Actually, TBA value is (pool_value - trade_payment) from buyer's perspective
    // For the roll:
    // - Short front: We receive trade payment (positive), lose pool (negative) = -front_value
    // - Long back: We pay trade payment (negative), gain pool (positive) = +back_value
    // Net = back_value - front_value

    let value = back_value.amount() - front_value.amount();

    Ok(Money::new(value, roll.notional.currency()))
}

/// Dollar roll discounting pricer.
#[derive(Clone, Debug, Default)]
pub struct DollarRollDiscountingPricer;

impl Pricer for DollarRollDiscountingPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::DollarRoll, ModelKey::Discounting)
    }

    fn price_dyn(
        &self,
        instrument: &dyn crate::instruments::common::traits::Instrument,
        market: &MarketContext,
        as_of: Date,
    ) -> PricingResult<ValuationResult> {
        let roll =
            crate::pricer::expect_inst::<DollarRoll>(instrument, InstrumentType::DollarRoll)?;

        let pv = price_dollar_roll(roll, market, as_of)
            .map_err(|e| PricingError::ModelFailure(e.to_string()))?;

        Ok(ValuationResult::stamped(roll.id.as_str(), as_of, pv))
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
    use finstack_core::math::interp::InterpStyle;
    use time::Month;

    fn create_test_market(as_of: Date) -> MarketContext {
        let disc = DiscountCurve::builder("USD-OIS")
            .base_date(as_of)
            .knots([
                (0.0, 1.0),
                (0.25, 0.99),
                (1.0, 0.96),
                (5.0, 0.80),
                (30.0, 0.30),
            ])
            .set_interp(InterpStyle::Linear)
            .build()
            .expect("valid curve");

        MarketContext::new().insert_discount(disc)
    }

    #[test]
    fn test_price_dollar_roll() {
        let roll = DollarRoll::example();
        let as_of = Date::from_calendar_date(2024, Month::January, 15).expect("valid");
        let market = create_test_market(as_of);

        let pv = price_dollar_roll(&roll, &market, as_of).expect("should price");

        // Value should be reasonable
        assert!(pv.amount().abs() < roll.notional.amount());
    }
}
