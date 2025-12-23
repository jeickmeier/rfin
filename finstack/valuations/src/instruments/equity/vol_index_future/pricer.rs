//! Volatility index future pricer implementation.

use crate::instruments::common::traits::Instrument;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_core::Result;

use super::VolatilityIndexFuture;

/// Simple pricer for volatility index futures.
///
/// Delegates to the instrument's `npv()` method, which implements
/// the standard futures pricing formula.
pub struct VolIndexFuturePricer;

impl VolIndexFuturePricer {
    /// Price a volatility index future.
    pub fn price(
        instrument: &VolatilityIndexFuture,
        market: &MarketContext,
        _as_of: Date,
    ) -> Result<Money> {
        instrument.npv(market)
    }

    /// Price using the Instrument trait.
    pub fn price_instrument(
        instrument: &dyn Instrument,
        market: &MarketContext,
        as_of: Date,
    ) -> Result<Money> {
        instrument.value(market, as_of)
    }
}
