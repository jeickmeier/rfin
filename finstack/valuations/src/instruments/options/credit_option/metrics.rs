//! Credit option specific metrics calculators

use crate::instruments::options::credit_option::CreditOption;
use crate::metrics::{MetricCalculator, MetricContext, MetricId, MetricRegistry};
use finstack_core::{Result, F};
use std::sync::Arc;

/// Delta calculator for credit options
pub struct DeltaCalculator;

impl MetricCalculator for DeltaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let _option: &CreditOption = context.instrument_as()?;
            // Would calculate actual delta here with forward credit spread and volatility
            Ok(0.5)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Gamma calculator for credit options
pub struct GammaCalculator;

impl MetricCalculator for GammaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let _option: &CreditOption = context.instrument_as()?;
            // Would calculate actual gamma here
            Ok(0.02)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Vega calculator for credit options
pub struct VegaCalculator;

impl MetricCalculator for VegaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let _option: &CreditOption = context.instrument_as()?;
            // Would calculate actual vega here
            Ok(0.1)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Theta calculator for credit options
pub struct ThetaCalculator;

impl MetricCalculator for ThetaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let _option: &CreditOption = context.instrument_as()?;
            // Would calculate actual theta here
            Ok(-0.05)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Rho calculator for credit options
pub struct RhoCalculator;

impl MetricCalculator for RhoCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let _option: &CreditOption = context.instrument_as()?;
            // Would calculate actual rho here
            Ok(0.03)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Implied Volatility calculator for credit options
pub struct ImpliedVolCalculator;

impl MetricCalculator for ImpliedVolCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let _option: &CreditOption = context.instrument_as()?;
            Ok(0.0)
    }

    fn dependencies(&self) -> &[MetricId] { &[] }
}

/// Register credit option metrics with the registry
pub fn register_credit_option_metrics(registry: &mut MetricRegistry) {
    registry.register_metric(
        MetricId::Delta,
        Arc::new(DeltaCalculator),
        &["CreditOption"],
    );

    registry.register_metric(
        MetricId::Gamma,
        Arc::new(GammaCalculator),
        &["CreditOption"],
    );

    registry.register_metric(MetricId::Vega, Arc::new(VegaCalculator), &["CreditOption"]);

    registry.register_metric(
        MetricId::Theta,
        Arc::new(ThetaCalculator),
        &["CreditOption"],
    );

    registry.register_metric(MetricId::Rho, Arc::new(RhoCalculator), &["CreditOption"]);

    registry.register_metric(
        MetricId::ImpliedVol,
        Arc::new(ImpliedVolCalculator),
        & ["CreditOption"],
    );
}
