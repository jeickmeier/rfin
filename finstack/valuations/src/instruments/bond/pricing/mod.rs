//! Bond pricing entrypoints and pricers.
//!
//! Bond pricing methods are now included in the Instrument trait via impl_instrument_schedule_pv! macro.

pub mod engine;
pub mod helpers;
pub mod schedule_helpers;
pub mod tree_pricer;
pub mod ytm_solver;

use super::types::Bond;
use crate::instruments::bond::pricing::engine::BondEngine;
use crate::instruments::common::{register_pricer, register_pricer_for_key, Pricer};
use finstack_core::dates::Date;
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::Result;

/// Discounting pricer for bonds delegating to the engine.
pub struct BondDiscountPricer;

impl Pricer<Bond> for BondDiscountPricer {
    fn price(&self, instrument: &Bond, context: &MarketContext, as_of: Date) -> Result<Money> {
        // Delegate to existing engine implementation
        BondEngine::price(instrument, context, as_of)
    }
}

/// Register default pricer for `Bond`. Call this during crate initialization paths as needed.
pub fn register_default_bond_pricers() {
    // Default discounting
    register_pricer::<Bond, _>(BondDiscountPricer);
    // Example keyed registrations (users can override):
    register_pricer_for_key::<Bond, _>("discount", BondDiscountPricer);
}
