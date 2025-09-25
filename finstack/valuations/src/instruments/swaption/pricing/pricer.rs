use crate::instruments::swaption::pricing::engine::SwaptionPricer;
use crate::instruments::swaption::Swaption;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;

use crate::impl_dyn_pricer;

impl_dyn_pricer!(
    name: BlackPricer,
    instrument: Swaption,
    instrument_key: Swaption,
    model: Black76,
    as_of = |inst: &Swaption, market: &finstack_core::market_data::MarketContext| -> finstack_core::Result<finstack_core::dates::Date> {
        let disc = market.get_ref::<DiscountCurve>(inst.disc_id.clone())?;
        Ok(disc.base_date())
    },
    pv    = |inst: &Swaption, market: &finstack_core::market_data::MarketContext, as_of: finstack_core::dates::Date| -> finstack_core::Result<finstack_core::money::Money> {
        SwaptionPricer.npv(inst, market, as_of)
    },
);
