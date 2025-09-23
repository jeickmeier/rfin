use crate::instruments::convertible::ConvertibleBond;
use crate::instruments::convertible::pricing::engine::{price_convertible_bond, ConvertibleTreeType};
use crate::pricer::{expect_inst, InstrumentKey, ModelKey, Pricer, PricerKey, PriceableExt, PricingError};
use finstack_core::market_data::MarketContext as Market;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;

pub struct DiscountingPricer;

impl DiscountingPricer { pub fn new() -> Self { Self } }

impl Default for DiscountingPricer { fn default() -> Self { Self::new() } }

impl Pricer for DiscountingPricer {
    fn key(&self) -> PricerKey { PricerKey::new(InstrumentKey::Convertible, ModelKey::Discounting) }
    fn price_dyn(&self, instrument: &dyn PriceableExt, market: &Market) -> std::result::Result<crate::results::ValuationResult, PricingError> {
        let cb: &ConvertibleBond = expect_inst(instrument, InstrumentKey::Convertible)?;
        let disc = market.get_ref::<DiscountCurve>(cb.disc_id)?;
        let as_of = disc.base_date();
        let pv = price_convertible_bond(cb, market, ConvertibleTreeType::Binomial(100))?;
        Ok(crate::results::ValuationResult::stamped(cb.id.as_str(), as_of, pv))
    }
}


