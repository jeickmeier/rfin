//! FX option specific metrics calculators

use crate::instruments::options::fx_option::FxOption;
use crate::metrics::{MetricCalculator, MetricContext, MetricId, MetricRegistry};
use finstack_core::{Result, F};
use std::sync::Arc;

/// Delta calculator for FX options
pub struct DeltaCalculator;

impl MetricCalculator for DeltaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let _option: &FxOption = context.instrument_as()?;
        // Would calculate actual delta here with FX spot and volatility
        Ok(0.5)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Gamma calculator for FX options
pub struct GammaCalculator;

impl MetricCalculator for GammaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let _option: &FxOption = context.instrument_as()?;
        // Would calculate actual gamma here
        Ok(0.02)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Vega calculator for FX options
pub struct VegaCalculator;

impl MetricCalculator for VegaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let _option: &FxOption = context.instrument_as()?;
        // Would calculate actual vega here
        Ok(0.1)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Theta calculator for FX options
pub struct ThetaCalculator;

impl MetricCalculator for ThetaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let _option: &FxOption = context.instrument_as()?;
        // Would calculate actual theta here
        Ok(-0.05)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Rho calculator for FX options (domestic rate)
pub struct RhoDomesticCalculator;

impl MetricCalculator for RhoDomesticCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let _option: &FxOption = context.instrument_as()?;
        // Would calculate actual domestic rho here
        Ok(0.03)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Rho calculator for FX options (foreign rate)
pub struct RhoForeignCalculator;

impl MetricCalculator for RhoForeignCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let _option: &FxOption = context.instrument_as()?;
        // Would calculate actual foreign rho here
        Ok(-0.02)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}
/// Implied Volatility calculator for FX options
pub struct ImpliedVolCalculator;

impl MetricCalculator for ImpliedVolCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let _option: &FxOption = context.instrument_as()?;
        Ok(0.0)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Register FX option metrics with the registry
pub fn register_fx_option_metrics(registry: &mut MetricRegistry) {
    registry.register_metric(MetricId::Delta, Arc::new(DeltaCalculator), &["FxOption"]);

    registry.register_metric(MetricId::Gamma, Arc::new(GammaCalculator), &["FxOption"]);

    registry.register_metric(MetricId::Vega, Arc::new(VegaCalculator), &["FxOption"]);

    registry.register_metric(MetricId::Theta, Arc::new(ThetaCalculator), &["FxOption"]);

    registry.register_metric(
        MetricId::custom("rho_domestic"),
        Arc::new(RhoDomesticCalculator),
        &["FxOption"],
    );

    registry.register_metric(
        MetricId::custom("rho_foreign"),
        Arc::new(RhoForeignCalculator),
        &["FxOption"],
    );

    registry.register_metric(
        MetricId::ImpliedVol,
        Arc::new(ImpliedVolCalculator),
        &["FxOption"],
    );
}
