use crate::instruments::fx_option::FxOption;
use crate::instruments::fx_option::pricing::engine::FxOptionPricer;
use crate::pricer::{expect_inst, InstrumentKey, ModelKey, Pricer, PricerKey, PriceableExt, PricingError};
use finstack_core::market_data::MarketContext as Market;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;

pub struct BlackPricer;

impl BlackPricer { pub fn new() -> Self { Self } }

impl Default for BlackPricer { fn default() -> Self { Self::new() } }

impl Pricer for BlackPricer {
    fn key(&self) -> PricerKey { PricerKey::new(InstrumentKey::FxOption, ModelKey::Black76) }
    fn price_dyn(&self, instrument: &dyn PriceableExt, market: &Market) -> std::result::Result<crate::results::ValuationResult, PricingError> {
        let opt: &FxOption = expect_inst(instrument, InstrumentKey::FxOption)?;
        let disc = market.get_ref::<DiscountCurve>(opt.domestic_disc_id)?;
        let as_of = disc.base_date();
        let pv = FxOptionPricer::npv(opt, market, as_of)?;
        Ok(crate::results::ValuationResult::stamped(opt.id.as_str(), as_of, pv))
    }
}


