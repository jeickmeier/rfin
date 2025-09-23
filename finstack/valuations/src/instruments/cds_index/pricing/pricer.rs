use crate::instruments::cds_index::CDSIndex;
use crate::instruments::cds_index::pricing::engine::CDSIndexPricer;
use crate::pricer::{expect_inst, InstrumentKey, ModelKey, Pricer, PricerKey, PriceableExt, PricingError};
use finstack_core::market_data::MarketContext as Market;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;

pub struct DiscountingPricer;

impl DiscountingPricer { pub fn new() -> Self { Self } }

impl Default for DiscountingPricer { fn default() -> Self { Self::new() } }

impl Pricer for DiscountingPricer {
    fn key(&self) -> PricerKey { PricerKey::new(InstrumentKey::CDSIndex, ModelKey::HazardRate) }
    fn price_dyn(&self, instrument: &dyn PriceableExt, market: &Market) -> std::result::Result<crate::results::ValuationResult, PricingError> {
        let idx: &CDSIndex = expect_inst(instrument, InstrumentKey::CDSIndex)?;
        let disc = market.get_ref::<DiscountCurve>(idx.premium.disc_id)?;
        let as_of = disc.base_date();
        let pv = CDSIndexPricer::new().npv(idx, market, as_of)?;
        Ok(crate::results::ValuationResult::stamped(idx.id.as_str(), as_of, pv))
    }
}


