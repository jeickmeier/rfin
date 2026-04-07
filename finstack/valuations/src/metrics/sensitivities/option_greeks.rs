//! Generic option greeks metric adapters.
//!
//! These calculators eliminate the per-instrument explosion of:
//! `metrics/{delta,gamma,vega,rho,theta,vanna,volga}.rs`
//! by delegating to the consolidated [`OptionGreeksProvider`] trait.

use std::marker::PhantomData;

use crate::instruments::common_impl::traits::{
    Instrument, OptionGreekKind, OptionGreeks, OptionGreeksProvider, OptionGreeksRequest,
};
use crate::metrics::{metric_not_found, MetricCalculator, MetricContext, MetricId};
use finstack_core::Result;

fn requested_value<I>(
    context: &mut MetricContext,
    request: OptionGreeksRequest,
    metric_id: MetricId,
    extract: impl FnOnce(OptionGreeks) -> Option<f64>,
) -> Result<f64>
where
    I: Instrument + OptionGreeksProvider + 'static,
{
    let inst: &I = context.instrument_as()?;
    let greeks = inst.option_greeks(&context.curves, context.as_of, &request)?;
    extract(greeks).ok_or_else(|| metric_not_found(metric_id))
}

/// Delta metric calculator (cash delta).
pub(crate) struct OptionDeltaCalculator<I> {
    _phantom: PhantomData<I>,
}

impl<I> Default for OptionDeltaCalculator<I> {
    fn default() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<I> MetricCalculator for OptionDeltaCalculator<I>
where
    I: Instrument + OptionGreeksProvider + 'static,
{
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        requested_value::<I>(
            context,
            OptionGreeksRequest {
                greek: OptionGreekKind::Delta,
                base_pv: None,
            },
            MetricId::Delta,
            |greeks| greeks.delta,
        )
    }
}

/// Gamma metric calculator.
pub(crate) struct OptionGammaCalculator<I> {
    _phantom: PhantomData<I>,
}

impl<I> Default for OptionGammaCalculator<I> {
    fn default() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<I> MetricCalculator for OptionGammaCalculator<I>
where
    I: Instrument + OptionGreeksProvider + 'static,
{
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        requested_value::<I>(
            context,
            OptionGreeksRequest {
                greek: OptionGreekKind::Gamma,
                base_pv: None,
            },
            MetricId::Gamma,
            |greeks| greeks.gamma,
        )
    }
}

/// Vega metric calculator (per 1% vol point).
pub(crate) struct OptionVegaCalculator<I> {
    _phantom: PhantomData<I>,
}

impl<I> Default for OptionVegaCalculator<I> {
    fn default() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<I> MetricCalculator for OptionVegaCalculator<I>
where
    I: Instrument + OptionGreeksProvider + 'static,
{
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        requested_value::<I>(
            context,
            OptionGreeksRequest {
                greek: OptionGreekKind::Vega,
                base_pv: None,
            },
            MetricId::Vega,
            |greeks| greeks.vega,
        )
    }
}

/// Theta metric calculator (instrument theta convention; typically per day).
pub(crate) struct OptionThetaCalculator<I> {
    _phantom: PhantomData<I>,
}

impl<I> Default for OptionThetaCalculator<I> {
    fn default() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<I> MetricCalculator for OptionThetaCalculator<I>
where
    I: Instrument + OptionGreeksProvider + 'static,
{
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        requested_value::<I>(
            context,
            OptionGreeksRequest {
                greek: OptionGreekKind::Theta,
                base_pv: None,
            },
            MetricId::Theta,
            |greeks| greeks.theta,
        )
    }
}

/// Rho metric calculator (domestic rate sensitivity per 1bp).
pub(crate) struct OptionRhoCalculator<I> {
    _phantom: PhantomData<I>,
}

impl<I> Default for OptionRhoCalculator<I> {
    fn default() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<I> MetricCalculator for OptionRhoCalculator<I>
where
    I: Instrument + OptionGreeksProvider + 'static,
{
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        requested_value::<I>(
            context,
            OptionGreeksRequest {
                greek: OptionGreekKind::Rho,
                base_pv: None,
            },
            MetricId::Rho,
            |greeks| greeks.rho_bp,
        )
    }
}

/// Foreign/dividend rho metric calculator (per 1bp).
///
/// Only instruments that support `MetricId::ForeignRho` should register this calculator.
pub(crate) struct OptionForeignRhoCalculator<I> {
    _phantom: PhantomData<I>,
}

impl<I> Default for OptionForeignRhoCalculator<I> {
    fn default() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<I> MetricCalculator for OptionForeignRhoCalculator<I>
where
    I: Instrument + OptionGreeksProvider + 'static,
{
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        requested_value::<I>(
            context,
            OptionGreeksRequest {
                greek: OptionGreekKind::ForeignRho,
                base_pv: None,
            },
            MetricId::ForeignRho,
            |greeks| greeks.foreign_rho_bp,
        )
    }
}

/// Vanna metric calculator (∂Δ/∂σ) computed via vol surface bumps.
///
/// Uses a central difference on **delta** under an absolute parallel vol bump:
/// \[
/// \text{vanna} \approx \frac{\Delta(\sigma+\Delta\sigma)-\Delta(\sigma-\Delta\sigma)}{2\Delta\sigma}
/// \]
pub(crate) struct OptionVannaCalculator<I> {
    _phantom: PhantomData<I>,
}

impl<I> Default for OptionVannaCalculator<I> {
    fn default() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<I> MetricCalculator for OptionVannaCalculator<I>
where
    I: Instrument + OptionGreeksProvider + 'static,
{
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        requested_value::<I>(
            context,
            OptionGreeksRequest {
                greek: OptionGreekKind::Vanna,
                base_pv: None,
            },
            MetricId::Vanna,
            |greeks| greeks.vanna,
        )
    }
}

/// Volga metric calculator (∂²V/∂σ²) computed via PV bumps.
///
/// Uses a central second difference on PV under an absolute parallel vol bump.
pub(crate) struct OptionVolgaCalculator<I> {
    _phantom: PhantomData<I>,
}

impl<I> Default for OptionVolgaCalculator<I> {
    fn default() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<I> MetricCalculator for OptionVolgaCalculator<I>
where
    I: Instrument + OptionGreeksProvider + 'static,
{
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        requested_value::<I>(
            context,
            OptionGreeksRequest {
                greek: OptionGreekKind::Volga,
                base_pv: Some(context.base_value.amount()),
            },
            MetricId::Volga,
            |greeks| greeks.volga,
        )
    }
}
