use crate::instruments::convertible::pricing::engine::{
    price_convertible_bond, ConvertibleTreeType,
};
use crate::instruments::convertible::ConvertibleBond;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;

// use macro exported from crate::pricer

crate::impl_dyn_pricer!(
    name: DiscountingPricer,
    instrument: ConvertibleBond,
    instrument_key: Convertible,
    model: Discounting,
    as_of = |inst: &ConvertibleBond, market: &finstack_core::market_data::MarketContext| -> finstack_core::Result<finstack_core::dates::Date> {
        let disc = market.get_ref::<DiscountCurve>(inst.disc_id)?;
        Ok(disc.base_date())
    },
    pv    = |inst: &ConvertibleBond, market: &finstack_core::market_data::MarketContext, _as_of: finstack_core::dates::Date| -> finstack_core::Result<finstack_core::money::Money> {
        price_convertible_bond(inst, market, ConvertibleTreeType::Binomial(100))
    },
);
