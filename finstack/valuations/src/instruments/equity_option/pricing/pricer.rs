use crate::instruments::equity_option::EquityOption;
use crate::instruments::equity_option::pricing::engine;
use crate::pricer::{expect_inst, InstrumentKey, ModelKey, Pricer, PricerKey, PriceableExt, PricingError};
use finstack_core::market_data::MarketContext as Market;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;

pub struct BlackPricer;

impl BlackPricer { pub fn new() -> Self { Self } }

impl Default for BlackPricer { fn default() -> Self { Self::new() } }

impl Pricer for BlackPricer {
    fn key(&self) -> PricerKey { PricerKey::new(InstrumentKey::EquityOption, ModelKey::Black76) }
    fn price_dyn(&self, instrument: &dyn PriceableExt, market: &Market) -> std::result::Result<crate::results::ValuationResult, PricingError> {
        let opt: &EquityOption = expect_inst(instrument, InstrumentKey::EquityOption)?;
        let disc = market.get_ref::<DiscountCurve>(opt.disc_id.as_str())?;
        let as_of = disc.base_date();
        let pv = engine::npv(opt, market, as_of)?;
        Ok(crate::results::ValuationResult::stamped(opt.id.as_str(), as_of, pv))
    }
}


