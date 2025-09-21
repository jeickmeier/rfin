//! Greeks metrics for `ConvertibleBond`.
//!
//! Provides Delta, Gamma, Vega, Rho, Theta using the tree-based greeks from
//! the pricing engine.

use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::{Result, F};

use crate::instruments::convertible::types::ConvertibleBond;
use crate::instruments::convertible::pricing::engine::{calculate_convertible_greeks, ConvertibleTreeType};

/// Internal enum to tag the greek type
#[derive(Clone, Copy)]
enum GreekType {
    Delta,
    Gamma,
    Vega,
    Rho,
    Theta,
}

/// Base calculator forwarding to pricing greeks and extracting the requested greek
struct GreeksCalculator {
    greek_type: GreekType,
}

impl GreeksCalculator {
    fn new(greek_type: GreekType) -> Self { Self { greek_type } }
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

pub struct DeltaCalculator;
impl MetricCalculator for DeltaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        GreeksCalculator::new(GreekType::Delta).calculate(context)
    }
}

pub struct GammaCalculator;
impl MetricCalculator for GammaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        GreeksCalculator::new(GreekType::Gamma).calculate(context)
    }
}

pub struct VegaCalculator;
impl MetricCalculator for VegaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        GreeksCalculator::new(GreekType::Vega).calculate(context)
    }
}

pub struct RhoCalculator;
impl MetricCalculator for RhoCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        GreeksCalculator::new(GreekType::Rho).calculate(context)
    }
}

pub struct ThetaCalculator;
impl MetricCalculator for ThetaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        GreeksCalculator::new(GreekType::Theta).calculate(context)
    }
}


