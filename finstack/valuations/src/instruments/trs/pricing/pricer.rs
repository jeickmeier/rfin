use crate::impl_dyn_pricer;
use crate::instruments::trs::{EquityTotalReturnSwap, FIIndexTotalReturnSwap};
use crate::pricer::{InstrumentKey, ModelKey, PriceableExt, Pricer, PricerKey, PricingError};
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::market_data::MarketContext as Market;

pub struct DiscountingPricer;

impl DiscountingPricer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DiscountingPricer {
    fn default() -> Self {
        Self::new()
    }
}

impl Pricer for DiscountingPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentKey::TRS, ModelKey::Discounting)
    }
    fn price_dyn(
        &self,
        instrument: &dyn PriceableExt,
        market: &Market,
    ) -> std::result::Result<crate::results::ValuationResult, PricingError> {
        // Equity TRS
        if let Some(eq) = instrument.as_any().downcast_ref::<EquityTotalReturnSwap>() {
            let disc = market.get_ref::<DiscountCurve>(eq.financing.disc_id.clone())?;
            let as_of = disc.base_date();
            // Delegate to instrument pv implementation
            use crate::instruments::common::traits::Instrument;
            let pv = eq.value(market, as_of)?;
            return Ok(crate::results::ValuationResult::stamped(
                eq.id.as_str(),
                as_of,
                pv,
            ));
        }
        // FI Index TRS
        if let Some(fi) = instrument.as_any().downcast_ref::<FIIndexTotalReturnSwap>() {
            let disc = market.get_ref::<DiscountCurve>(fi.financing.disc_id.clone())?;
            let as_of = disc.base_date();
            use crate::instruments::common::traits::Instrument;
            let pv = fi.value(market, as_of)?;
            return Ok(crate::results::ValuationResult::stamped(
                fi.id.as_str(),
                as_of,
                pv,
            ));
        }
        Err(PricingError::TypeMismatch {
            expected: InstrumentKey::TRS,
            got: instrument.key(),
        })
    }
}

// For TRS, we expose concrete pricers for each variant for registry consumption using the common macro
impl_dyn_pricer!(
    name: EquityTrsPricer,
    instrument: EquityTotalReturnSwap,
    instrument_key: TRS,
    model: Discounting,
    as_of = |inst: &EquityTotalReturnSwap, market: &finstack_core::market_data::MarketContext| -> finstack_core::Result<finstack_core::dates::Date> {
        let disc = market.get_ref::<DiscountCurve>(inst.financing.disc_id.clone())?;
        Ok(disc.base_date())
    },
    pv    = |inst: &EquityTotalReturnSwap, market: &finstack_core::market_data::MarketContext, as_of: finstack_core::dates::Date| -> finstack_core::Result<finstack_core::money::Money> {
        use crate::instruments::common::traits::Instrument as _;
        inst.value(market, as_of)
    },
);

impl_dyn_pricer!(
    name: FiIndexTrsPricer,
    instrument: FIIndexTotalReturnSwap,
    instrument_key: TRS,
    model: Discounting,
    as_of = |inst: &FIIndexTotalReturnSwap, market: &finstack_core::market_data::MarketContext| -> finstack_core::Result<finstack_core::dates::Date> {
        let disc = market.get_ref::<DiscountCurve>(inst.financing.disc_id.clone())?;
        Ok(disc.base_date())
    },
    pv    = |inst: &FIIndexTotalReturnSwap, market: &finstack_core::market_data::MarketContext, as_of: finstack_core::dates::Date| -> finstack_core::Result<finstack_core::money::Money> {
        use crate::instruments::common::traits::Instrument as _;
        inst.value(market, as_of)
    },
);
