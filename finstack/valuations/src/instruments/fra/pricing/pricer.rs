use crate::instruments::fra::ForwardRateAgreement;
use crate::instruments::fra::pricing::engine::FraEngine;
use crate::pricer::{expect_inst, InstrumentKey, ModelKey, Pricer, PricerKey, PriceableExt, PricingError};
use finstack_core::market_data::MarketContext as Market;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;

pub struct DiscountingPricer;

impl DiscountingPricer { pub fn new() -> Self { Self } }

impl Default for DiscountingPricer { fn default() -> Self { Self::new() } }

impl Pricer for DiscountingPricer {
    fn key(&self) -> PricerKey { PricerKey::new(InstrumentKey::IRS, ModelKey::Discounting) }
    fn price_dyn(&self, instrument: &dyn PriceableExt, market: &Market) -> std::result::Result<crate::results::ValuationResult, PricingError> {
        let fra: &ForwardRateAgreement = expect_inst(instrument, InstrumentKey::IRS)?;
        let disc = market.get_ref::<DiscountCurve>(fra.disc_id.clone())?;
        let as_of = disc.base_date();
        let pv = FraEngine::pv(fra, market)?;
        Ok(crate::results::ValuationResult::stamped(fra.id.as_str(), as_of, pv))
    }
}


