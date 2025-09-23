use crate::instruments::fx_spot::FxSpot;
use crate::instruments::fx_spot::pricing::engine::FxSpotPricer;
use crate::pricer::{expect_inst, InstrumentKey, ModelKey, Pricer, PricerKey, PriceableExt, PricingError};
use finstack_core::market_data::MarketContext as Market;
use finstack_core::dates::Date;

pub struct DiscountingPricer;

impl DiscountingPricer { pub fn new() -> Self { Self } }

impl Default for DiscountingPricer { fn default() -> Self { Self::new() } }

impl Pricer for DiscountingPricer {
    fn key(&self) -> PricerKey { PricerKey::new(InstrumentKey::FxSpot, ModelKey::Discounting) }
    fn price_dyn(&self, instrument: &dyn PriceableExt, market: &Market) -> std::result::Result<crate::results::ValuationResult, PricingError> {
        let fx: &FxSpot = expect_inst(instrument, InstrumentKey::FxSpot)?;
        // Use settlement if present, else a conservative default
        let as_of: Date = fx.settlement.unwrap_or_else(|| Date::from_calendar_date(1970, time::Month::January, 1).unwrap());
        let pv = FxSpotPricer.pv(fx, market, as_of)?;
        Ok(crate::results::ValuationResult::stamped(fx.id.as_str(), as_of, pv))
    }
}


