//! Conversion premium metric for `ConvertibleBond`.
//!
//! Computes conversion premium = bond_price / (spot * conversion_ratio) - 1.

use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::{Result, F};

use crate::instruments::convertible::pricing::engine::calculate_conversion_premium;
use crate::instruments::convertible::types::ConvertibleBond;

/// Calculator for conversion premium.
pub struct ConversionPremiumCalculator;

impl MetricCalculator for ConversionPremiumCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let bond = context.instrument_as::<ConvertibleBond>()?;

        // Get current bond price from context
        let bond_price = context.base_value.amount();

        // Get current spot price
        let underlying_id = bond
            .underlying_equity_id
            .as_ref()
            .ok_or(finstack_core::Error::Internal)?;

        let spot_price = context.curves.price(underlying_id)?;
        let spot = match spot_price {
            finstack_core::market_data::scalars::MarketScalar::Price(money) => money.amount(),
            finstack_core::market_data::scalars::MarketScalar::Unitless(value) => *value,
        };

        // Get conversion ratio
        let conversion_ratio = if let Some(ratio) = bond.conversion.ratio {
            ratio
        } else if let Some(price) = bond.conversion.price {
            bond.notional.amount() / price
        } else {
            return Err(finstack_core::Error::Internal);
        };

        Ok(calculate_conversion_premium(
            bond_price,
            spot,
            conversion_ratio,
        ))
    }
}
