use crate::instruments::ir_future::pricing::engine::IrFutureEngine;
use crate::instruments::ir_future::InterestRateFuture;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;

impl_dyn_pricer!(
    name: DiscountingPricer,
    instrument: InterestRateFuture,
    instrument_key: InterestRateFuture,
    model: Discounting,
    as_of = |inst: &InterestRateFuture, market: &finstack_core::market_data::MarketContext| -> finstack_core::Result<finstack_core::dates::Date> {
        let disc = market.get_ref::<DiscountCurve>(inst.disc_id.clone())?;
        Ok(disc.base_date())
    },
    pv    = |inst: &InterestRateFuture, market: &finstack_core::market_data::MarketContext, _as_of: finstack_core::dates::Date| -> finstack_core::Result<finstack_core::money::Money> {
        IrFutureEngine::pv(inst, market)
    },
);
