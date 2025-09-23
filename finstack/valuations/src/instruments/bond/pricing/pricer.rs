use crate::instruments::bond::types::Bond;
use crate::instruments::bond::pricing::tree_pricer::TreePricer;
use crate::pricer::{expect_inst, InstrumentKey, ModelKey, Pricer, PricerKey, PriceableExt, PricingError};
use finstack_core::market_data::MarketContext as Market;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use indexmap::IndexMap;

pub struct DiscountingPricer;

impl DiscountingPricer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DiscountingPricer { fn default() -> Self { Self::new() } }

impl Pricer for DiscountingPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentKey::Bond, ModelKey::Discounting)
    }

    fn price_dyn(
        &self,
        instrument: &dyn PriceableExt,
        market: &Market,
    ) -> std::result::Result<crate::results::ValuationResult, PricingError> {
        let bond: &Bond = expect_inst(instrument, InstrumentKey::Bond)?;

        // Use the discount curve base date as valuation date to match instrument PV impl
        let disc = market.get_ref::<DiscountCurve>(bond.disc_id.as_str())?;
        let as_of = disc.base_date();

        use crate::instruments::common::traits::Instrument as _;
        let pv = bond.value(market, as_of)?;
        Ok(crate::results::ValuationResult::stamped(bond.id.as_str(), as_of, pv))
    }
}

pub struct OasPricer;

impl OasPricer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for OasPricer { fn default() -> Self { Self::new() } }

impl Pricer for OasPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentKey::Bond, ModelKey::Tree)
    }

    fn price_dyn(
        &self,
        instrument: &dyn PriceableExt,
        market: &Market,
    ) -> std::result::Result<crate::results::ValuationResult, PricingError> {
        let bond: &Bond = expect_inst(instrument, InstrumentKey::Bond)?;

        let disc = market.get_ref::<DiscountCurve>(bond.disc_id.as_str())?;
        let as_of = disc.base_date();

        // PV from base value implementation
        use crate::instruments::common::traits::Instrument as _;
        let pv = bond.value(market, as_of)?;

        // OAS requires quoted clean price (%)
        let clean_pct = bond
            .pricing_overrides
            .quoted_clean_price
            .ok_or_else(|| PricingError::ModelFailure("quoted_clean_price required for OAS".to_string()))?;

        let oas_bp = TreePricer::new().calculate_oas(bond, market, as_of, clean_pct)?;

        let mut measures: IndexMap<String, finstack_core::F> = IndexMap::new();
        measures.insert("oas_bp".to_string(), oas_bp);

        Ok(crate::results::ValuationResult::stamped(bond.id.as_str(), as_of, pv).with_measures(measures))
    }
}


