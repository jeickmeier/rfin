use crate::instruments::trs::{EquityTotalReturnSwap, FIIndexTotalReturnSwap};
use crate::pricer::{InstrumentType, ModelKey, PriceableExt, Pricer, PricerKey, PricingError};
use finstack_core::market_data::MarketContext as Market;
use finstack_core::market_data::MarketContext;

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
        PricerKey::new(InstrumentType::TRS, ModelKey::Discounting)
    }
    fn price_dyn(
        &self,
        instrument: &dyn PriceableExt,
        market: &Market,
    ) -> std::result::Result<crate::results::ValuationResult, PricingError> {
        // Equity TRS
        if let Some(eq) = instrument.as_any().downcast_ref::<EquityTotalReturnSwap>() {
            let disc = market.get_discount_ref(eq.financing.disc_id.clone())?;
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
            let disc = market.get_discount_ref(fi.financing.disc_id.clone())?;
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
            expected: InstrumentType::TRS,
            got: instrument.key(),
        })
    }
}


// ========================= NEW SIMPLIFIED TRS PRICER =========================

/// New simplified TRS discounting pricer that handles both Equity and FI Index variants
pub struct SimpleTrsDiscountingPricer;

impl SimpleTrsDiscountingPricer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SimpleTrsDiscountingPricer {
    fn default() -> Self {
        Self::new()
    }
}

impl Pricer for SimpleTrsDiscountingPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::TRS, ModelKey::Discounting)
    }

    fn price_dyn(
        &self,
        instrument: &dyn PriceableExt,
        market: &MarketContext,
    ) -> Result<crate::results::ValuationResult, PricingError> {
        // Handle both TRS variants using the existing DiscountingPricer logic
        let existing_pricer = DiscountingPricer::new();
        existing_pricer.price_dyn(instrument, market)
    }
}
