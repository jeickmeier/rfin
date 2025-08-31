//! FX option specific metrics calculators

use crate::instruments::options::fx_option::FxOption;
use crate::metrics::{MetricCalculator, MetricContext, MetricId, MetricRegistry};
use finstack_core::{Result, F};
use num_traits::ToPrimitive;
use std::sync::Arc;

/// Delta calculator for FX options
pub struct DeltaCalculator;

impl MetricCalculator for DeltaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let option: &FxOption = context.instrument_as()?;
        
        let time_to_expiry = option.day_count.year_fraction(context.as_of, option.expiry)?;
        
        if time_to_expiry <= 0.0 {
            // Option expired - delta is 0 or 1/-1 based on moneyness
            let fx_matrix = context.curves.fx.as_ref().ok_or(finstack_core::error::InputError::NotFound)?;
            let spot = fx_matrix.rate(option.base_currency, option.quote_currency, context.as_of, finstack_core::money::fx::FxConversionPolicy::CashflowDate)?;
            
            let spot_f64 = spot.to_f64().unwrap_or(0.0);
            return Ok(match option.option_type {
                super::OptionType::Call => if spot_f64 > option.strike { 1.0 } else { 0.0 },
                super::OptionType::Put => if spot_f64 < option.strike { -1.0 } else { 0.0 },
            });
        }
        
        // Get market data
        let domestic_disc = context.curves.discount(option.domestic_disc_id)?;
        let foreign_disc = context.curves.discount(option.foreign_disc_id)?;
        let r_d = domestic_disc.zero(time_to_expiry);
        let r_f = foreign_disc.zero(time_to_expiry);
        
        let fx_matrix = context.curves.fx.as_ref().ok_or(finstack_core::error::InputError::NotFound)?;
        let spot = fx_matrix.rate(option.base_currency, option.quote_currency, context.as_of, finstack_core::money::fx::FxConversionPolicy::CashflowDate)?.to_f64().unwrap_or(0.0);
        
        let sigma = if let Some(impl_vol) = option.implied_vol {
            impl_vol
        } else {
            let vol_surface = context.curves.vol_surface(option.vol_id)?;
            vol_surface.value_checked(time_to_expiry, option.strike)?
        };
        
        // Scale delta by notional for full cash delta
        Ok(option.delta(spot, r_d, r_f, sigma, time_to_expiry) * option.notional.amount())
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Gamma calculator for FX options
pub struct GammaCalculator;

impl MetricCalculator for GammaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let option: &FxOption = context.instrument_as()?;
        let time_to_expiry = option.day_count.year_fraction(context.as_of, option.expiry)?;
        if time_to_expiry <= 0.0 { return Ok(0.0); }
        
        let domestic_disc = context.curves.discount(option.domestic_disc_id)?;
        let foreign_disc = context.curves.discount(option.foreign_disc_id)?;
        let r_d = domestic_disc.zero(time_to_expiry);
        let r_f = foreign_disc.zero(time_to_expiry);
        let fx_matrix = context.curves.fx.as_ref().ok_or(finstack_core::error::InputError::NotFound)?;
        let spot = fx_matrix.rate(option.base_currency, option.quote_currency, context.as_of, finstack_core::money::fx::FxConversionPolicy::CashflowDate)?.to_f64().unwrap_or(0.0);
        let sigma = if let Some(impl_vol) = option.implied_vol {
            impl_vol
        } else {
            context.curves.vol_surface(option.vol_id)?.value_checked(time_to_expiry, option.strike)?
        };
        
        Ok(option.gamma(spot, r_d, r_f, sigma, time_to_expiry) * option.notional.amount())
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Vega calculator for FX options
pub struct VegaCalculator;

impl MetricCalculator for VegaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let option: &FxOption = context.instrument_as()?;
        let time_to_expiry = option.day_count.year_fraction(context.as_of, option.expiry)?;
        if time_to_expiry <= 0.0 { return Ok(0.0); }
        
        let domestic_disc = context.curves.discount(option.domestic_disc_id)?;
        let foreign_disc = context.curves.discount(option.foreign_disc_id)?;
        let r_d = domestic_disc.zero(time_to_expiry);
        let r_f = foreign_disc.zero(time_to_expiry);
        let fx_matrix = context.curves.fx.as_ref().ok_or(finstack_core::error::InputError::NotFound)?;
        let spot = fx_matrix.rate(option.base_currency, option.quote_currency, context.as_of, finstack_core::money::fx::FxConversionPolicy::CashflowDate)?.to_f64().unwrap_or(0.0);
        let sigma = if let Some(impl_vol) = option.implied_vol {
            impl_vol
        } else {
            context.curves.vol_surface(option.vol_id)?.value_checked(time_to_expiry, option.strike)?
        };
        
        Ok(option.vega(spot, r_d, r_f, sigma, time_to_expiry) * option.notional.amount())
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Theta calculator for FX options
pub struct ThetaCalculator;

impl MetricCalculator for ThetaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let option: &FxOption = context.instrument_as()?;
        let time_to_expiry = option.day_count.year_fraction(context.as_of, option.expiry)?;
        if time_to_expiry <= 0.0 { return Ok(0.0); }
        
        let domestic_disc = context.curves.discount(option.domestic_disc_id)?;
        let foreign_disc = context.curves.discount(option.foreign_disc_id)?;
        let r_d = domestic_disc.zero(time_to_expiry);
        let r_f = foreign_disc.zero(time_to_expiry);
        let fx_matrix = context.curves.fx.as_ref().ok_or(finstack_core::error::InputError::NotFound)?;
        let spot = fx_matrix.rate(option.base_currency, option.quote_currency, context.as_of, finstack_core::money::fx::FxConversionPolicy::CashflowDate)?.to_f64().unwrap_or(0.0);
        let sigma = if let Some(impl_vol) = option.implied_vol {
            impl_vol
        } else {
            context.curves.vol_surface(option.vol_id)?.value_checked(time_to_expiry, option.strike)?
        };
        
        Ok(option.theta(spot, r_d, r_f, sigma, time_to_expiry) * option.notional.amount())
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Rho calculator for FX options (domestic rate)
pub struct RhoDomesticCalculator;

impl MetricCalculator for RhoDomesticCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let option: &FxOption = context.instrument_as()?;
        let time_to_expiry = option.day_count.year_fraction(context.as_of, option.expiry)?;
        if time_to_expiry <= 0.0 { return Ok(0.0); }
        
        let domestic_disc = context.curves.discount(option.domestic_disc_id)?;
        let foreign_disc = context.curves.discount(option.foreign_disc_id)?;
        let r_d = domestic_disc.zero(time_to_expiry);
        let r_f = foreign_disc.zero(time_to_expiry);
        let fx_matrix = context.curves.fx.as_ref().ok_or(finstack_core::error::InputError::NotFound)?;
        let spot = fx_matrix.rate(option.base_currency, option.quote_currency, context.as_of, finstack_core::money::fx::FxConversionPolicy::CashflowDate)?.to_f64().unwrap_or(0.0);
        let sigma = if let Some(impl_vol) = option.implied_vol {
            impl_vol
        } else {
            context.curves.vol_surface(option.vol_id)?.value_checked(time_to_expiry, option.strike)?
        };
        
        Ok(option.rho_domestic(spot, r_d, r_f, sigma, time_to_expiry) * option.notional.amount())
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Rho calculator for FX options (foreign rate)
pub struct RhoForeignCalculator;

impl MetricCalculator for RhoForeignCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let option: &FxOption = context.instrument_as()?;
        let time_to_expiry = option.day_count.year_fraction(context.as_of, option.expiry)?;
        if time_to_expiry <= 0.0 { return Ok(0.0); }
        
        let domestic_disc = context.curves.discount(option.domestic_disc_id)?;
        let foreign_disc = context.curves.discount(option.foreign_disc_id)?;
        let r_d = domestic_disc.zero(time_to_expiry);
        let r_f = foreign_disc.zero(time_to_expiry);
        let fx_matrix = context.curves.fx.as_ref().ok_or(finstack_core::error::InputError::NotFound)?;
        let spot = fx_matrix.rate(option.base_currency, option.quote_currency, context.as_of, finstack_core::money::fx::FxConversionPolicy::CashflowDate)?.to_f64().unwrap_or(0.0);
        let sigma = if let Some(impl_vol) = option.implied_vol {
            impl_vol
        } else {
            context.curves.vol_surface(option.vol_id)?.value_checked(time_to_expiry, option.strike)?
        };
        
        Ok(option.rho_foreign(spot, r_d, r_f, sigma, time_to_expiry) * option.notional.amount())
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
