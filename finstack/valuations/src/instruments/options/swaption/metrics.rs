//! Swaption-specific metrics calculators

use crate::instruments::options::swaption::Swaption;
use crate::metrics::{MetricCalculator, MetricContext, MetricId, MetricRegistry};
use finstack_core::{Result, F};
use std::sync::Arc;

/// Delta calculator for swaptions
pub struct DeltaCalculator;

impl MetricCalculator for DeltaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let option: &Swaption = context.instrument_as()?;
        // Temporary inputs until forecast/annuity wiring is added
        let disc = context.curves.discount(option.disc_id)?;
        // Approximate forward swap rate and annuity from pricer helpers
        let forward = option.forward_swap_rate(disc.as_ref())?;
        let t = option.year_fraction(disc.base_date(), option.expiry, option.day_count)?;
        let sigma = option.sabr_params.as_ref().map(|p| p.alpha).unwrap_or(0.20);

        // Use Black delta approximation via d1 CDF (dimensionless w.r.t forward)
        // Here we approximate delta as N(d1) for payer, -N(-d1) for receiver
        let variance = sigma * sigma * t;
        let d1 = if variance > 0.0 {
            ((forward / option.strike_rate).ln() + 0.5 * variance) / variance.sqrt()
        } else {
            0.0
        };
        let delta = match option.option_type {
            super::OptionType::Call => crate::instruments::options::models::norm_cdf(d1),
            super::OptionType::Put => -crate::instruments::options::models::norm_cdf(-d1),
        };
        Ok(delta)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Gamma calculator for swaptions
pub struct GammaCalculator;

impl MetricCalculator for GammaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let option: &Swaption = context.instrument_as()?;
        let disc = context.curves.discount(option.disc_id)?;
        let forward = option.forward_swap_rate(disc.as_ref())?;
        let t = option.year_fraction(disc.base_date(), option.expiry, option.day_count)?;
        let sigma = option.sabr_params.as_ref().map(|p| p.alpha).unwrap_or(0.20);
        if t <= 0.0 || sigma <= 0.0 || forward <= 0.0 {
            return Ok(0.0);
        }
        let variance = sigma * sigma * t;
        let d1 = ((forward / option.strike_rate).ln() + 0.5 * variance) / variance.sqrt();
        let gamma =
            crate::instruments::options::models::norm_pdf(d1) / (forward * sigma * t.sqrt());
        Ok(gamma)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Vega calculator for swaptions
pub struct VegaCalculator;

impl MetricCalculator for VegaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let option: &Swaption = context.instrument_as()?;
        let disc = context.curves.discount(option.disc_id)?;
        let forward = option.forward_swap_rate(disc.as_ref())?;
        let t = option.year_fraction(disc.base_date(), option.expiry, option.day_count)?;
        let sigma = option.sabr_params.as_ref().map(|p| p.alpha).unwrap_or(0.20);
        let variance = sigma * sigma * t;
        let d1 = if variance > 0.0 {
            ((forward / option.strike_rate).ln() + 0.5 * variance) / variance.sqrt()
        } else {
            0.0
        };
        let vega = forward * crate::instruments::options::models::norm_pdf(d1) * t.sqrt() / 100.0;
        Ok(vega)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Theta calculator for swaptions (daily)
pub struct ThetaCalculator;

impl MetricCalculator for ThetaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let option: &Swaption = context.instrument_as()?;
        let disc = context.curves.discount(option.disc_id)?;
        let base = disc.base_date();
        let t = option.year_fraction(base, option.expiry, option.day_count)?;
        if t <= 0.0 {
            return Ok(0.0);
        }
        let forward = option.forward_swap_rate(disc.as_ref())?;
        let sigma = option.sabr_params.as_ref().map(|p| p.alpha).unwrap_or(0.20);
        let variance = sigma * sigma * t;
        let d1 = if variance > 0.0 {
            ((forward / option.strike_rate).ln() + 0.5 * variance) / variance.sqrt()
        } else {
            0.0
        };
        let d2 = d1 - variance.sqrt();
        let sqrt_t = t.sqrt();
        let term1 =
            -forward * crate::instruments::options::models::norm_pdf(d1) * sigma / (2.0 * sqrt_t);
        let theta = match option.option_type {
            super::OptionType::Call => {
                // Payer
                let term3 = -0.0 * crate::instruments::options::models::norm_cdf(d2);
                (term1 + term3) * 10000.0 / 365.0 // scaled similar to IR options
            }
            super::OptionType::Put => {
                // Receiver
                let term3 = 0.0 * crate::instruments::options::models::norm_cdf(-d2);
                (term1 + term3) * 10000.0 / 365.0
            }
        };
        Ok(theta)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Rho calculator for swaptions (per 1%)
pub struct RhoCalculator;

impl MetricCalculator for RhoCalculator {
    fn calculate(&self, _context: &mut MetricContext) -> Result<F> {
        // Requires a full rate bump across the curve; return 0.0 placeholder for now
        Ok(0.0)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Implied Volatility calculator for swaptions
pub struct ImpliedVolCalculator;

impl MetricCalculator for ImpliedVolCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let _option: &Swaption = context.instrument_as()?;
        Ok(0.0)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Register swaption metrics with the registry
pub fn register_swaption_metrics(registry: &mut MetricRegistry) {
    registry.register_metric(MetricId::Delta, Arc::new(DeltaCalculator), &["Swaption"]);
    registry.register_metric(MetricId::Gamma, Arc::new(GammaCalculator), &["Swaption"]);
    registry.register_metric(MetricId::Vega, Arc::new(VegaCalculator), &["Swaption"]);
    registry.register_metric(MetricId::Theta, Arc::new(ThetaCalculator), &["Swaption"]);
    registry.register_metric(MetricId::Rho, Arc::new(RhoCalculator), &["Swaption"]);
    registry.register_metric(
        MetricId::ImpliedVol,
        Arc::new(ImpliedVolCalculator),
        &["Swaption"],
    );
}
