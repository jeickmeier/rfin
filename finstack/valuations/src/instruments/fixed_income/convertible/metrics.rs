//! Metrics for convertible bonds.
//!
//! Implements comprehensive metrics for convertible bonds including:
//! - Parity and conversion premium
//! - Greeks (Delta, Gamma, Vega, Rho, Theta)
//! - Credit-sensitive measures

use crate::metrics::{MetricCalculator, MetricContext, MetricId, MetricRegistry};
use finstack_core::{Result, F};
use std::sync::Arc;

use super::model::{
    calculate_conversion_premium, calculate_convertible_greeks, calculate_parity,
    ConvertibleTreeType,
};
use super::ConvertibleBond;

/// Register convertible bond metrics into the registry.
pub fn register_convertible_metrics(registry: &mut MetricRegistry) {
    // Parity metric
    registry.register_metric(
        MetricId::custom("parity"),
        Arc::new(ParityCalculator),
        &["ConvertibleBond"],
    );

    // Conversion Premium metric
    registry.register_metric(
        MetricId::custom("conversion_premium"),
        Arc::new(ConversionPremiumCalculator),
        &["ConvertibleBond"],
    );

    // Greeks metrics - use existing standard IDs
    registry.register_metric(
        MetricId::Delta,
        Arc::new(DeltaCalculator),
        &["ConvertibleBond"],
    );

    registry.register_metric(
        MetricId::Gamma,
        Arc::new(GammaCalculator),
        &["ConvertibleBond"],
    );

    registry.register_metric(
        MetricId::Vega,
        Arc::new(VegaCalculator),
        &["ConvertibleBond"],
    );

    registry.register_metric(MetricId::Rho, Arc::new(RhoCalculator), &["ConvertibleBond"]);

    registry.register_metric(
        MetricId::Theta,
        Arc::new(ThetaCalculator),
        &["ConvertibleBond"],
    );
}

/// Calculator for convertible bond parity
struct ParityCalculator;

impl MetricCalculator for ParityCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let bond = context.instrument_as::<ConvertibleBond>()?;

        let underlying_id = bond
            .underlying_equity_id
            .as_ref()
            .ok_or(finstack_core::Error::Internal)?;

        let spot_price = context.curves.market_scalar(underlying_id)?;
        let spot = match spot_price {
            finstack_core::market_data::primitives::MarketScalar::Price(money) => money.amount(),
            finstack_core::market_data::primitives::MarketScalar::Unitless(value) => *value,
        };

        Ok(calculate_parity(bond, spot))
    }
}

/// Calculator for conversion premium
struct ConversionPremiumCalculator;

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

        let spot_price = context.curves.market_scalar(underlying_id)?;
        let spot = match spot_price {
            finstack_core::market_data::primitives::MarketScalar::Price(money) => money.amount(),
            finstack_core::market_data::primitives::MarketScalar::Unitless(value) => *value,
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

/// Base struct for Greeks calculators
struct GreeksCalculator {
    greek_type: GreekType,
}

#[derive(Clone, Copy)]
enum GreekType {
    Delta,
    Gamma,
    Vega,
    Rho,
    Theta,
}

impl GreeksCalculator {
    fn new(greek_type: GreekType) -> Self {
        Self { greek_type }
    }
}

impl MetricCalculator for GreeksCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let bond = context.instrument_as::<ConvertibleBond>()?;

        let greeks = calculate_convertible_greeks(
            bond,
            &context.curves,
            ConvertibleTreeType::default(),
            None,
        )?;

        let value = match self.greek_type {
            GreekType::Delta => greeks.delta,
            GreekType::Gamma => greeks.gamma,
            GreekType::Vega => greeks.vega,
            GreekType::Rho => greeks.rho,
            GreekType::Theta => greeks.theta,
        };

        Ok(value)
    }
}

// Individual calculator structs
struct DeltaCalculator;
impl MetricCalculator for DeltaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        GreeksCalculator::new(GreekType::Delta).calculate(context)
    }
}

struct GammaCalculator;
impl MetricCalculator for GammaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        GreeksCalculator::new(GreekType::Gamma).calculate(context)
    }
}

struct VegaCalculator;
impl MetricCalculator for VegaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        GreeksCalculator::new(GreekType::Vega).calculate(context)
    }
}

struct RhoCalculator;
impl MetricCalculator for RhoCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        GreeksCalculator::new(GreekType::Rho).calculate(context)
    }
}

struct ThetaCalculator;
impl MetricCalculator for ThetaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        GreeksCalculator::new(GreekType::Theta).calculate(context)
    }
}
