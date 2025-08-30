//! Equity option specific metrics calculators

use crate::metrics::{MetricCalculator, MetricContext, MetricId, MetricRegistry};
use finstack_core::{Result, F};
use std::sync::Arc;

/// Delta calculator for equity options
pub struct DeltaCalculator;

impl MetricCalculator for DeltaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        use crate::instruments::Instrument;

        if let Instrument::EquityOption(_option) = &*context.instrument {
            // Would calculate actual delta here with spot price and volatility
            Ok(0.5)
        } else {
            Err(finstack_core::Error::from(
                finstack_core::error::InputError::NotFound,
            ))
        }
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Gamma calculator for equity options
pub struct GammaCalculator;

impl MetricCalculator for GammaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        use crate::instruments::Instrument;

        if let Instrument::EquityOption(_option) = &*context.instrument {
            // Would calculate actual gamma here
            Ok(0.02)
        } else {
            Err(finstack_core::Error::from(
                finstack_core::error::InputError::NotFound,
            ))
        }
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Vega calculator for equity options
pub struct VegaCalculator;

impl MetricCalculator for VegaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        use crate::instruments::Instrument;

        if let Instrument::EquityOption(_option) = &*context.instrument {
            // Would calculate actual vega here
            Ok(0.1)
        } else {
            Err(finstack_core::Error::from(
                finstack_core::error::InputError::NotFound,
            ))
        }
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Theta calculator for equity options
pub struct ThetaCalculator;

impl MetricCalculator for ThetaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        use crate::instruments::Instrument;

        if let Instrument::EquityOption(_option) = &*context.instrument {
            // Would calculate actual theta here
            Ok(-0.05)
        } else {
            Err(finstack_core::Error::from(
                finstack_core::error::InputError::NotFound,
            ))
        }
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Rho calculator for equity options
pub struct RhoCalculator;

impl MetricCalculator for RhoCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        use crate::instruments::Instrument;

        if let Instrument::EquityOption(_option) = &*context.instrument {
            // Would calculate actual rho here
            Ok(0.03)
        } else {
            Err(finstack_core::Error::from(
                finstack_core::error::InputError::NotFound,
            ))
        }
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Register equity option metrics with the registry
pub fn register_equity_option_metrics(registry: &mut MetricRegistry) {
    registry.register_metric(
        MetricId::Delta,
        Arc::new(DeltaCalculator),
        &["EquityOption"],
    );

    registry.register_metric(
        MetricId::Gamma,
        Arc::new(GammaCalculator),
        &["EquityOption"],
    );

    registry.register_metric(MetricId::Vega, Arc::new(VegaCalculator), &["EquityOption"]);

    registry.register_metric(
        MetricId::Theta,
        Arc::new(ThetaCalculator),
        &["EquityOption"],
    );

    registry.register_metric(MetricId::Rho, Arc::new(RhoCalculator), &["EquityOption"]);
}
