use crate::instruments::common::traits::Instrument;
use crate::instruments::common::GenericDiscountingPricer;
use crate::instruments::trs::{EquityTotalReturnSwap, FIIndexTotalReturnSwap};
use crate::pricer::{InstrumentType, ModelKey, Pricer, PricerKey, PricingError};
use finstack_core::market_data::MarketContext;

/// Generic discounting pricer for Equity Total Return Swaps.
pub type SimpleEquityTrsDiscountingPricer = GenericDiscountingPricer<EquityTotalReturnSwap>;

/// Generic discounting pricer for Fixed Income Index Total Return Swaps.
pub type SimpleFIIndexTrsDiscountingPricer = GenericDiscountingPricer<FIIndexTotalReturnSwap>;

/// Combined TRS discounting pricer that handles both Equity and FI Index variants.
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
        instrument: &dyn Instrument,
        market: &MarketContext,
    ) -> Result<crate::results::ValuationResult, PricingError> {
        // Handle Equity TRS
        if let Some(equity_trs) = instrument.as_any().downcast_ref::<EquityTotalReturnSwap>() {
            let equity_pricer = SimpleEquityTrsDiscountingPricer::new();
            return equity_pricer.price_dyn(equity_trs, market);
        }

        // Handle FI Index TRS
        if let Some(fi_trs) = instrument.as_any().downcast_ref::<FIIndexTotalReturnSwap>() {
            let fi_pricer = SimpleFIIndexTrsDiscountingPricer::new();
            return fi_pricer.price_dyn(fi_trs, market);
        }

        Err(PricingError::TypeMismatch {
            expected: InstrumentType::TRS,
            got: instrument.key(),
        })
    }
}
