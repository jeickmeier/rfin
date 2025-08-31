//! Interest rate option specific metrics calculators

use crate::instruments::options::cap_floor::InterestRateOption;
use crate::metrics::{MetricCalculator, MetricContext, MetricId, MetricRegistry};
use finstack_core::{Result, F};
use std::sync::Arc;

/// Delta calculator for interest rate options
pub struct DeltaCalculator;

impl MetricCalculator for DeltaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let option: &InterestRateOption = context.instrument_as()?;
        // Minimal example: require inputs via attributes or implied_vol
        let sigma = option.implied_vol.unwrap_or(0.20);
        // Placeholder forward and t until forward curve utilities are exposed
        let forward_rate = option.strike_rate;
        let t = 1.0;
        Ok(option.delta(forward_rate, sigma, t))
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Gamma calculator for interest rate options
pub struct GammaCalculator;

impl MetricCalculator for GammaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let option: &InterestRateOption = context.instrument_as()?;
        let sigma = option.implied_vol.unwrap_or(0.20);
        let forward_rate = option.strike_rate;
        let t = 1.0;
        Ok(option.gamma(forward_rate, sigma, t))
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Vega calculator for interest rate options
pub struct VegaCalculator;

impl MetricCalculator for VegaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let option: &InterestRateOption = context.instrument_as()?;
        let sigma = option.implied_vol.unwrap_or(0.20);
        let forward_rate = option.strike_rate;
        let t = 1.0;
        Ok(option.vega(forward_rate, sigma, t))
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Theta calculator for interest rate options
pub struct ThetaCalculator;

impl MetricCalculator for ThetaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let option: &InterestRateOption = context.instrument_as()?;
        // Approximate theta via finite difference on t (per day)
        let sigma = option.implied_vol.unwrap_or(0.20);
        let forward_rate = option.strike_rate;
        let t = 1.0;
        let dt = 1.0 / 365.25;
        let base = option.delta(forward_rate, sigma, t); // not ideal; would prefer price fn
        let later = option.delta(forward_rate, sigma, t - dt);
        Ok(-(base - later) / dt / 365.25)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Rho calculator for interest rate options
pub struct RhoCalculator;

impl MetricCalculator for RhoCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let _option: &InterestRateOption = context.instrument_as()?;
        // Placeholder: rho requires rate bump; not available here
        Ok(0.0)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Implied Volatility calculator for interest rate options
pub struct ImpliedVolCalculator;

impl MetricCalculator for ImpliedVolCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let _option: &InterestRateOption = context.instrument_as()?;
        Ok(0.0)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Register interest rate option metrics with the registry
pub fn register_interest_rate_option_metrics(registry: &mut MetricRegistry) {
    registry.register_metric(
        MetricId::Delta,
        Arc::new(DeltaCalculator),
        &["InterestRateOption"],
    );

    registry.register_metric(
        MetricId::Gamma,
        Arc::new(GammaCalculator),
        &["InterestRateOption"],
    );

    registry.register_metric(
        MetricId::Vega,
        Arc::new(VegaCalculator),
        &["InterestRateOption"],
    );

    registry.register_metric(
        MetricId::Theta,
        Arc::new(ThetaCalculator),
        &["InterestRateOption"],
    );

    registry.register_metric(
        MetricId::Rho,
        Arc::new(RhoCalculator),
        &["InterestRateOption"],
    );

    registry.register_metric(
        MetricId::ImpliedVol,
        Arc::new(ImpliedVolCalculator),
        &["InterestRateOption"],
    );
}
