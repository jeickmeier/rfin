//! Parity metric for `ConvertibleBond`.
//!
//! Computes the parity ratio: equity conversion value divided by bond face value.
//! Leverages pricing helpers from `pricing`.

use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::{Result, F};

use crate::instruments::convertible::pricing::engine::calculate_parity;
use crate::instruments::convertible::types::ConvertibleBond;

/// Calculator for convertible bond parity.
pub struct ParityCalculator;

impl MetricCalculator for ParityCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let bond = context.instrument_as::<ConvertibleBond>()?;

        let underlying_id = bond
            .underlying_equity_id
            .as_ref()
            .ok_or(finstack_core::Error::Internal)?;

        let spot_price = context.curves.price(underlying_id)?;
        let spot = match spot_price {
            finstack_core::market_data::scalars::MarketScalar::Price(money) => money.amount(),
            finstack_core::market_data::scalars::MarketScalar::Unitless(value) => *value,
        };

        Ok(calculate_parity(bond, spot))
    }
}
