use crate::instruments::fx_spot::pricing::engine::FxSpotPricer;
use crate::instruments::fx_spot::FxSpot;
use finstack_core::dates::Date;

// use macro exported from crate::pricer

crate::impl_dyn_pricer!(
    name: DiscountingPricer,
    instrument: FxSpot,
    instrument_key: FxSpot,
    model: Discounting,
    as_of = |_inst: &FxSpot, _market: &finstack_core::market_data::MarketContext| -> finstack_core::Result<Date> {
        Ok(Date::from_calendar_date(1970, time::Month::January, 1).unwrap())
    },
    pv    = |inst: &FxSpot, market: &finstack_core::market_data::MarketContext, as_of: Date| -> finstack_core::Result<finstack_core::money::Money> {
        FxSpotPricer.pv(inst, market, as_of)
    },
);
