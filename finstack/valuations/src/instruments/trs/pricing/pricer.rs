use crate::instruments::trs::{EquityTotalReturnSwap, FIIndexTotalReturnSwap};
use crate::pricer::{InstrumentKey, ModelKey, Pricer, PricerKey, PriceableExt, PricingError};
use finstack_core::market_data::MarketContext as Market;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;

pub struct DiscountingPricer;

impl DiscountingPricer { pub fn new() -> Self { Self } }

impl Default for DiscountingPricer { fn default() -> Self { Self::new() } }

impl Pricer for DiscountingPricer {
    fn key(&self) -> PricerKey { PricerKey::new(InstrumentKey::TRS, ModelKey::Discounting) }
    fn price_dyn(&self, instrument: &dyn PriceableExt, market: &Market) -> std::result::Result<crate::results::ValuationResult, PricingError> {
        // Equity TRS
        if let Some(eq) = instrument.as_any().downcast_ref::<EquityTotalReturnSwap>() {
            let disc = market.get_ref::<DiscountCurve>(eq.financing.disc_id.clone())?;
            let as_of = disc.base_date();
            // Delegate to instrument pv implementation
            use crate::instruments::common::traits::Instrument;
            let pv = eq.value(market, as_of)?;
            return Ok(crate::results::ValuationResult::stamped(eq.id.as_str(), as_of, pv));
        }
        // FI Index TRS
        if let Some(fi) = instrument.as_any().downcast_ref::<FIIndexTotalReturnSwap>() {
            let disc = market.get_ref::<DiscountCurve>(fi.financing.disc_id.clone())?;
            let as_of = disc.base_date();
            use crate::instruments::common::traits::Instrument;
            let pv = fi.value(market, as_of)?;
            return Ok(crate::results::ValuationResult::stamped(fi.id.as_str(), as_of, pv));
        }
        Err(PricingError::TypeMismatch { expected: InstrumentKey::TRS, got: instrument.key() })
    }
}


