//! Swaption-specific metrics calculators

use crate::instruments::options::swaption::Swaption;
use crate::instruments::options::OptionType;
use crate::metrics::{MetricCalculator, MetricContext, MetricId, MetricRegistry};
use finstack_core::dates::Date;
use finstack_core::market_data::traits::{Discount, TermStructure};
use finstack_core::prelude::Result;
use finstack_core::types::CurveId;
use finstack_core::F;
use std::sync::Arc;

/// Wrapper for a discount curve with a parallel rate bump applied.
///
/// This applies the formula: df_bumped(t) = df_original(t) * exp(-bump * t)
/// where bump is in rate units (e.g., 0.0001 for 1bp).
struct BumpedDiscountCurve {
    original: Arc<dyn Discount + Send + Sync>,
    bump_rate: F,
}

impl BumpedDiscountCurve {
    fn new(original: Arc<dyn Discount + Send + Sync>, bump_bp: F) -> Self {
        Self {
            original,
            bump_rate: bump_bp / 10_000.0, // Convert bp to rate
        }
    }
}

impl TermStructure for BumpedDiscountCurve {
    fn id(&self) -> &CurveId {
        self.original.id()
    }
}

impl Discount for BumpedDiscountCurve {
    #[inline]
    fn base_date(&self) -> Date {
        self.original.base_date()
    }

    #[inline]
    fn df(&self, t: F) -> F {
        let original_df = self.original.df(t);
        original_df * (-self.bump_rate * t).exp()
    }
}

/// Delta calculator for swaptions
pub struct DeltaCalculator;

impl MetricCalculator for DeltaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let option: &Swaption = context.instrument_as()?;
        let disc = context.curves.disc(option.disc_id)?;
        let t = option.year_fraction(disc.base_date(), option.expiry, option.day_count)?;

        if t <= 0.0 {
            return Ok(0.0);
        }

        let forward = option.forward_swap_rate(disc.as_ref())?;
        let annuity = option.swap_annuity(disc.as_ref())?;

        let sigma = if let Some(sabr) = &option.sabr_params {
            let model = crate::instruments::options::models::SABRModel::new(sabr.clone());
            model.implied_volatility(forward, option.strike_rate, t)?
        } else if let Some(impl_vol) = option.pricing_overrides.implied_volatility {
            impl_vol
        } else {
            context
                .curves
                .surface(option.vol_id)?
                .value_clamped(t, option.strike_rate)
        };

        let variance = sigma * sigma * t;
        let d1 = if variance > 0.0 {
            ((forward / option.strike_rate).ln() + 0.5 * variance) / variance.sqrt()
        } else {
            0.0
        };

        let delta = match option.option_type {
            OptionType::Call => finstack_core::math::norm_cdf(d1),
            OptionType::Put => -finstack_core::math::norm_cdf(-d1),
        };

        // Scale by notional and annuity for cash delta
        Ok(delta * option.notional.amount() * annuity)
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
        let disc = context.curves.disc(option.disc_id)?;
        let t = option.year_fraction(disc.base_date(), option.expiry, option.day_count)?;

        if t <= 0.0 {
            return Ok(0.0);
        }

        let forward = option.forward_swap_rate(disc.as_ref())?;
        let annuity = option.swap_annuity(disc.as_ref())?;

        let sigma = if let Some(sabr) = &option.sabr_params {
            let model = crate::instruments::options::models::SABRModel::new(sabr.clone());
            model.implied_volatility(forward, option.strike_rate, t)?
        } else if let Some(impl_vol) = option.pricing_overrides.implied_volatility {
            impl_vol
        } else {
            context
                .curves
                .surface(option.vol_id)?
                .value_clamped(t, option.strike_rate)
        };

        if sigma <= 0.0 || forward <= 0.0 {
            return Ok(0.0);
        }

        let variance = sigma * sigma * t;
        let d1 = ((forward / option.strike_rate).ln() + 0.5 * variance) / variance.sqrt();
        let gamma = finstack_core::math::norm_pdf(d1) / (forward * sigma * t.sqrt());

        // Scale by notional and annuity for cash gamma
        Ok(gamma * option.notional.amount() * annuity)
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
        let disc = context.curves.disc(option.disc_id)?;
        let t = option.year_fraction(disc.base_date(), option.expiry, option.day_count)?;

        if t <= 0.0 {
            return Ok(0.0);
        }

        let forward = option.forward_swap_rate(disc.as_ref())?;
        let annuity = option.swap_annuity(disc.as_ref())?;

        let sigma = if let Some(sabr) = &option.sabr_params {
            let model = crate::instruments::options::models::SABRModel::new(sabr.clone());
            model.implied_volatility(forward, option.strike_rate, t)?
        } else if let Some(impl_vol) = option.pricing_overrides.implied_volatility {
            impl_vol
        } else {
            context
                .curves
                .surface(option.vol_id)?
                .value_clamped(t, option.strike_rate)
        };

        let variance = sigma * sigma * t;
        let d1 = if variance > 0.0 {
            ((forward / option.strike_rate).ln() + 0.5 * variance) / variance.sqrt()
        } else {
            0.0
        };

        let vega = forward * finstack_core::math::norm_pdf(d1) * t.sqrt() / 100.0;
        // Scale by notional and annuity for cash vega
        Ok(vega * option.notional.amount() * annuity)
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
        let disc = context.curves.disc(option.disc_id)?;
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
        let term1 = -forward * finstack_core::math::norm_pdf(d1) * sigma / (2.0 * sqrt_t);
        let theta = match option.option_type {
            OptionType::Call => {
                // Payer
                let term3 = -0.0 * finstack_core::math::norm_cdf(d2);
                (term1 + term3) * 10000.0 / 365.0 // scaled similar to IR options
            }
            OptionType::Put => {
                // Receiver
                let term3 = 0.0 * finstack_core::math::norm_cdf(-d2);
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
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let option: &Swaption = context.instrument_as()?;
        let disc = context.curves.disc(option.disc_id)?;

        // Base price from context
        let base_price = context.base_value.amount();

        // Get volatility from surface using original curves (vol held constant)
        let time_to_expiry =
            option.year_fraction(disc.base_date(), option.expiry, option.day_count)?;
        let vol = if let Some(impl_vol) = option.pricing_overrides.implied_volatility {
            impl_vol
        } else {
            let vol_surface = context.curves.surface(option.vol_id)?;
            vol_surface.value_clamped(time_to_expiry, option.strike_rate)
        };

        // Create bumped discount curve (+1bp)
        let bumped_disc = BumpedDiscountCurve::new(disc, 1.0);

        // Reprice with bumped curve
        let bumped_price = if option.sabr_params.is_some() {
            option.sabr_price(&bumped_disc)?.amount()
        } else {
            option.black_price(&bumped_disc, vol)?.amount()
        };

        // Rho per 1% = (PV_bumped_1bp - PV_base) * 100
        // This gives sensitivity to a 100bp (1%) parallel shift
        let rho_1bp = bumped_price - base_price;
        Ok(rho_1bp * 100.0)
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
