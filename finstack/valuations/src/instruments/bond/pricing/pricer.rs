use crate::instruments::bond::pricing::tree_pricer::TreePricer;
use crate::instruments::bond::types::Bond;

use indexmap::IndexMap;

// use macro exported from crate::pricer

crate::impl_dyn_pricer!(
    name: DiscountingPricer,
    instrument: Bond,
    instrument_key: Bond,
    model: Discounting,
    as_of = |inst: &Bond, market: &finstack_core::market_data::MarketContext| -> finstack_core::Result<finstack_core::dates::Date> {
        let disc = market.get_discount_ref(inst.disc_id.clone())?;
        Ok(disc.base_date())
    },
    pv    = |inst: &Bond, market: &finstack_core::market_data::MarketContext, as_of: finstack_core::dates::Date| -> finstack_core::Result<finstack_core::money::Money> {
        use crate::instruments::common::traits::Instrument as _;
        inst.value(market, as_of)
    },
);

pub struct OasPricer;

impl OasPricer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for OasPricer {
    fn default() -> Self {
        Self::new()
    }
}

impl crate::pricer::Pricer for OasPricer {
    fn key(&self) -> crate::pricer::PricerKey {
        crate::pricer::PricerKey::new(
            crate::pricer::InstrumentKey::Bond,
            crate::pricer::ModelKey::Tree,
        )
    }

    fn price_dyn(
        &self,
        instrument: &dyn crate::pricer::PriceableExt,
        market: &finstack_core::market_data::MarketContext,
    ) -> std::result::Result<crate::results::ValuationResult, crate::pricer::PricingError> {
        // Use the enhanced macro to perform the same logic with a custom result step
        crate::impl_dyn_pricer!(
            name: __OasMacroPricer,
            instrument: Bond,
            instrument_key: Bond,
            model: Tree,
            as_of = |inst: &Bond, market: &finstack_core::market_data::MarketContext| -> finstack_core::Result<finstack_core::dates::Date> {
                let disc = market.get_discount_ref(inst.disc_id.clone())?;
                Ok(disc.base_date())
            },
            pv    = |inst: &Bond, market: &finstack_core::market_data::MarketContext, as_of: finstack_core::dates::Date| -> finstack_core::Result<finstack_core::money::Money> {
                use crate::instruments::common::traits::Instrument as _;
                inst.value(market, as_of)
            },
            result = |inst: &Bond, market: &finstack_core::market_data::MarketContext, as_of: finstack_core::dates::Date, result: crate::results::ValuationResult| -> finstack_core::Result<crate::results::ValuationResult> {
                // OAS requires quoted clean price (%)
                let clean_pct = inst
                    .pricing_overrides
                    .quoted_clean_price
                    .ok_or(finstack_core::error::InputError::Invalid)?;

                let oas_bp = TreePricer::new().calculate_oas(inst, market, as_of, clean_pct)?;

                let mut measures: IndexMap<String, finstack_core::F> = IndexMap::new();
                measures.insert("oas_bp".to_string(), oas_bp);

                Ok(result.with_measures(measures))
            },
        );

        __OasMacroPricer::new().price_dyn(instrument, market)
    }
}
