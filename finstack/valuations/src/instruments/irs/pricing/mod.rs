//! IRS pricing facade and engine re-export.
//!
//! Provides the pricing entrypoints for `InterestRateSwap`. Core pricing
//! logic lives in `engine`. IRS pricing methods are now included in
//! the Instrument trait via impl_instrument_schedule_pv! macro.

pub mod engine;

// Re-export engine for backward compatibility
pub use engine::IrsEngine;

use crate::instruments::common::{register_pricer, register_pricer_for_key, Pricer};
use crate::instruments::irs::types::InterestRateSwap;
use finstack_core::dates::Date;
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::Result;

/// Discounting pricer for vanilla IRS delegating to the engine.
pub struct IrsDiscountPricer;

impl Pricer<InterestRateSwap> for IrsDiscountPricer {
    fn price(&self, instrument: &InterestRateSwap, context: &MarketContext, _as_of: Date) -> Result<Money> {
        engine::IrsEngine::pv(instrument, context)
    }
}

/// Register default pricer for `InterestRateSwap`.
pub fn register_default_irs_pricers() {
    register_pricer::<InterestRateSwap, _>(IrsDiscountPricer);
    register_pricer_for_key::<InterestRateSwap, _>("discount", IrsDiscountPricer);
}
