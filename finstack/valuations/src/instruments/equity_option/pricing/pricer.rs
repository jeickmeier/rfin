use crate::instruments::equity_option::pricing::engine;
use crate::instruments::equity_option::EquityOption;

use crate::impl_dyn_pricer;

impl_dyn_pricer!(
    name: BlackPricer,
    instrument: EquityOption,
    instrument_key: EquityOption,
    model: Black76,
    as_of = |inst: &EquityOption, market: &finstack_core::market_data::MarketContext| -> finstack_core::Result<finstack_core::dates::Date> {
        let disc = market.get_discount_ref(inst.disc_id.clone())?;
        Ok(disc.base_date())
    },
    pv    = |inst: &EquityOption, market: &finstack_core::market_data::MarketContext, as_of: finstack_core::dates::Date| -> finstack_core::Result<finstack_core::money::Money> {
        engine::npv(inst, market, as_of)
    },
);
