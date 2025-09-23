use crate::instruments::cap_floor::InterestRateOption;
use crate::instruments::cap_floor::pricing::engine::IrOptionPricer;
use crate::pricer::{expect_inst, InstrumentKey, ModelKey, Pricer, PricerKey, PriceableExt, PricingError};
use finstack_core::market_data::MarketContext as Market;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;

pub struct BlackPricer;

impl BlackPricer { pub fn new() -> Self { Self } }

impl Default for BlackPricer { fn default() -> Self { Self::new() } }

impl Pricer for BlackPricer {
    fn key(&self) -> PricerKey { PricerKey::new(InstrumentKey::CapFloor, ModelKey::Black76) }
    fn price_dyn(&self, instrument: &dyn PriceableExt, market: &Market) -> std::result::Result<crate::results::ValuationResult, PricingError> {
        let ir: &InterestRateOption = expect_inst(instrument, InstrumentKey::CapFloor)?;
        let disc = market.get_ref::<DiscountCurve>(ir.disc_id)?;
        let as_of = disc.base_date();
        let pv = IrOptionPricer::new().price(ir, market, as_of)?;
        Ok(crate::results::ValuationResult::stamped(ir.id.as_str(), as_of, pv))
    }
}


