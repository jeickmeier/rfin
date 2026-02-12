//! Conversion value calculator for convertible bonds.
//!
//! Reports the absolute conversion value based on the current spot price and
//! conversion terms. For standard policies this is `conversion_ratio * spot`.
//! For `MandatoryVariable` (PERCS/DECS/ACES) the value depends on the spot
//! relative to the upper/lower conversion prices.

use crate::instruments::fixed_income::convertible::ConvertibleBond;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Conversion value calculator.
///
/// Uses the same variable delivery logic as the pricer for `MandatoryVariable`:
/// - spot <= lower_price: `(face / lower_price) * spot` (max shares, loss)
/// - lower < spot <= upper: `face` (variable ratio delivers par)
/// - spot > upper_price: `(face / upper_price) * spot` (min shares, capped)
pub struct ConversionValueCalculator;

impl MetricCalculator for ConversionValueCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let bond: &ConvertibleBond = context.instrument_as()?;

        let underlying_id = bond.underlying_equity_id.as_ref().ok_or_else(|| {
            finstack_core::Error::from(finstack_core::InputError::NotFound {
                id: "underlying_equity_id".to_string(),
            })
        })?;

        let spot_price = context.curves.price(underlying_id)?;
        let spot = match spot_price {
            finstack_core::market_data::scalars::MarketScalar::Price(money) => money.amount(),
            finstack_core::market_data::scalars::MarketScalar::Unitless(value) => *value,
        };

        use crate::instruments::fixed_income::convertible::ConversionPolicy;

        match &bond.conversion.policy {
            ConversionPolicy::MandatoryVariable {
                upper_conversion_price,
                lower_conversion_price,
                ..
            } => {
                let face = bond.notional.amount();
                if spot <= *lower_conversion_price {
                    Ok((face / lower_conversion_price) * spot)
                } else if spot <= *upper_conversion_price {
                    Ok(face)
                } else {
                    Ok((face / upper_conversion_price) * spot)
                }
            }
            _ => {
                let conversion_ratio = bond.effective_conversion_ratio().unwrap_or(0.0);
                Ok(spot * conversion_ratio)
            }
        }
    }
}
