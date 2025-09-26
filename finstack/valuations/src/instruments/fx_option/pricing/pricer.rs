use crate::instruments::fx_option::pricing::engine::FxOptionPricer;
use crate::instruments::fx_option::FxOption;

// use macro exported from crate::pricer

crate::impl_dyn_pricer!(
    name: BlackPricer,
    instrument: FxOption,
    instrument_key: FxOption,
    model: Black76,
    as_of = |inst: &FxOption, market: &finstack_core::market_data::MarketContext| -> finstack_core::Result<finstack_core::dates::Date> {
        let disc = market.get_discount_ref(inst.domestic_disc_id)?;
        Ok(disc.base_date())
    },
    pv    = |inst: &FxOption, market: &finstack_core::market_data::MarketContext, as_of: finstack_core::dates::Date| -> finstack_core::Result<finstack_core::money::Money> {
        FxOptionPricer::npv(inst, market, as_of)
    },
);
