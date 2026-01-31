//! Generic option greeks metric adapters.
//!
//! These calculators eliminate the per-instrument explosion of:
//! `metrics/{delta,gamma,vega,rho,theta,vanna,volga}.rs`
//! by delegating to small instrument-provided traits:
//! - [`OptionDeltaProvider`]
//! - [`OptionGammaProvider`]
//! - [`OptionVegaProvider`]
//! - [`OptionThetaProvider`]
//! - [`OptionRhoProvider`]
//! - [`OptionForeignRhoProvider`]
//! - [`OptionVannaProvider`]
//! - [`OptionVolgaProvider`]

use std::marker::PhantomData;

use crate::instruments::common::traits::{
    Instrument, OptionDeltaProvider, OptionForeignRhoProvider, OptionGammaProvider,
    OptionRhoProvider, OptionThetaProvider, OptionVannaProvider, OptionVegaProvider,
    OptionVolgaProvider,
};
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::Result;

/// Delta metric calculator (cash delta).
pub struct OptionDeltaCalculator<I> {
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
    I: Instrument + OptionDeltaProvider + 'static,
{
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let inst: &I = context.instrument_as()?;
        inst.option_delta(&context.curves, context.as_of)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Gamma metric calculator.
pub struct OptionGammaCalculator<I> {
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
    I: Instrument + OptionGammaProvider + 'static,
{
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let inst: &I = context.instrument_as()?;
        inst.option_gamma(&context.curves, context.as_of)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Vega metric calculator (per 1% vol point).
pub struct OptionVegaCalculator<I> {
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
    I: Instrument + OptionVegaProvider + 'static,
{
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let inst: &I = context.instrument_as()?;
        inst.option_vega(&context.curves, context.as_of)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Theta metric calculator (instrument theta convention; typically per day).
pub struct OptionThetaCalculator<I> {
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
    I: Instrument + OptionThetaProvider + 'static,
{
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let inst: &I = context.instrument_as()?;
        inst.option_theta(&context.curves, context.as_of)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Rho metric calculator (domestic rate sensitivity per 1bp).
pub struct OptionRhoCalculator<I> {
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
    I: Instrument + OptionRhoProvider + 'static,
{
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let inst: &I = context.instrument_as()?;
        inst.option_rho_bp(&context.curves, context.as_of)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Foreign/dividend rho metric calculator (per 1bp).
///
/// Only instruments that support `MetricId::ForeignRho` should register this calculator.
pub struct OptionForeignRhoCalculator<I> {
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
    I: Instrument + OptionForeignRhoProvider + 'static,
{
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let inst: &I = context.instrument_as()?;
        inst.option_foreign_rho_bp(&context.curves, context.as_of)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Vanna metric calculator (∂Δ/∂σ) computed via vol surface bumps.
///
/// Uses a central difference on **delta** under an absolute parallel vol bump:
/// \[
/// \text{vanna} \approx \frac{\Delta(\sigma+\Delta\sigma)-\Delta(\sigma-\Delta\sigma)}{2\Delta\sigma}
/// \]
pub struct OptionVannaCalculator<I> {
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
    I: Instrument + OptionVannaProvider + 'static,
{
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let inst: &I = context.instrument_as()?;
        inst.option_vanna(&context.curves, context.as_of)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Volga metric calculator (∂²V/∂σ²) computed via PV bumps.
///
/// Uses a central second difference on PV under an absolute parallel vol bump.
pub struct OptionVolgaCalculator<I> {
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
    I: Instrument + OptionVolgaProvider + 'static,
{
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let inst: &I = context.instrument_as()?;
        inst.option_volga(&context.curves, context.as_of, context.base_value.amount())
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}
