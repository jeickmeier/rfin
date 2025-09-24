use crate::impl_dyn_pricer;
use crate::instruments::basket::pricing::engine::BasketPricer;
use crate::instruments::basket::Basket;
use finstack_core::dates::Date;

impl_dyn_pricer!(
    name: DiscountingPricer,
    instrument: Basket,
    instrument_key: Basket,
    model: Discounting,
    as_of = |_inst: &Basket, _market: &finstack_core::market_data::MarketContext| -> finstack_core::Result<Date> {
        Ok(Date::from_calendar_date(1970, time::Month::January, 1).unwrap())
    },
    pv    = |inst: &Basket, market: &finstack_core::market_data::MarketContext, as_of: Date| -> finstack_core::Result<finstack_core::money::Money> {
        BasketPricer::new().basket_value(inst, market, as_of)
    },
);
