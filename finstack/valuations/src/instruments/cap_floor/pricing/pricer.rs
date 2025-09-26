use crate::instruments::cap_floor::pricing::engine::IrOptionPricer;
use crate::instruments::cap_floor::InterestRateOption;

// use macro exported from crate::pricer

crate::impl_dyn_pricer!(
    name: BlackPricer,
    instrument: InterestRateOption,
    instrument_key: CapFloor,
    model: Black76,
    as_of = |inst: &InterestRateOption, market: &finstack_core::market_data::MarketContext| -> finstack_core::Result<finstack_core::dates::Date> {
        let disc = market.get_discount_ref(inst.disc_id.clone())?;
        Ok(disc.base_date())
    },
    pv    = |inst: &InterestRateOption, market: &finstack_core::market_data::MarketContext, as_of: finstack_core::dates::Date| -> finstack_core::Result<finstack_core::money::Money> {
        IrOptionPricer::new().price(inst, market, as_of)
    },
);
