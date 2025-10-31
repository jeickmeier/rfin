//! Generic finite difference greek calculators for equity instruments.
//!
//! Provides reusable implementations of Delta, Gamma, and Vega calculators
//! that work with any instrument implementing `HasEquityUnderlying` and `Instrument`.
//!
//! This eliminates code duplication across exotic options (AsianOption, Autocallable,
//! BarrierOption, LookbackOption, etc.) that all use the same finite difference pattern.

use std::marker::PhantomData;

use crate::instruments::common::metrics::finite_difference::{
    adaptive_spot_bump, bump_scalar_price, bump_sizes, get_bump_overrides,
};
use crate::instruments::common::metrics::has_equity_underlying::HasEquityUnderlying;
use crate::instruments::common::metrics::has_pricing_overrides::HasPricingOverrides;
use crate::instruments::common::metrics::vol_expiry_helpers::{
    get_instrument_day_count, get_instrument_expiry_for_adaptive, get_instrument_vol_id,
};
use crate::instruments::common::traits::Instrument;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Generic delta calculator using finite differences.
///
/// Works with any instrument that implements `HasEquityUnderlying` and `Instrument`.
/// Computes delta by bumping spot price up and down and using central differences.
pub struct GenericFdDelta<I> {
    _phantom: PhantomData<I>,
}

impl<I> Default for GenericFdDelta<I> {
    fn default() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<I> MetricCalculator for GenericFdDelta<I>
where
    I: Instrument + HasEquityUnderlying + HasPricingOverrides + Clone + 'static,
{
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let instrument: &I = context.instrument_as()?;
        let as_of = context.as_of;

        // Get current spot for bump size calculation
        let spot_scalar = context.curves.price(instrument.spot_id())?;
        let current_spot = match spot_scalar {
            finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
            finstack_core::market_data::scalars::MarketScalar::Price(m) => m.amount(),
        };

        // Calculate adaptive or fixed bump size
        let bump_pct = if let Some(ref overrides) = context.pricing_overrides {
            let (spot_override, _, _) = get_bump_overrides(overrides);
            if overrides.adaptive_bumps {
                // Try to get vol, expiry, and day_count for adaptive calculation
                let day_count = get_instrument_day_count(instrument.as_any());
                let expiry = get_instrument_expiry_for_adaptive(instrument.as_any());
                let vol_id = get_instrument_vol_id(instrument.as_any());
                
                let time_to_expiry = if let (Some(dc), Some(exp)) = (day_count, expiry) {
                    dc.year_fraction(as_of, exp, finstack_core::dates::DayCountCtx::default()).ok().unwrap_or(0.0)
                } else {
                    0.0
                };
                
                let atm_vol = vol_id
                    .and_then(|vol_id| {
                        context.curves.surface_ref(vol_id.as_str()).ok()
                    })
                    .and_then(|vol_surface| {
                        if time_to_expiry > 0.0 {
                            Some(vol_surface.value_clamped(time_to_expiry, current_spot))
                        } else {
                            None
                        }
                    })
                    .unwrap_or(bump_sizes::VOLATILITY); // Fallback to default vol

                adaptive_spot_bump(current_spot, atm_vol, time_to_expiry, spot_override)
            } else {
                spot_override.unwrap_or(bump_sizes::SPOT)
            }
        } else {
            bump_sizes::SPOT
        };

        let bump_size = current_spot * bump_pct;

        // Clone instruments for bumping and set deterministic MC seed scenarios
        // This ensures MC-priced instruments produce identical results for up/down bumps
        let mut instrument_up = instrument.clone();
        let mut instrument_down = instrument.clone();
        
        // Set different seed scenarios for up and down bumps to ensure deterministic greeks
        instrument_up.pricing_overrides_mut().mc_seed_scenario = Some("delta_up".to_string());
        instrument_down.pricing_overrides_mut().mc_seed_scenario = Some("delta_down".to_string());

        // Bump spot up
        let curves_up = bump_scalar_price(&context.curves, instrument.spot_id(), bump_pct)?;
        let pv_up = instrument_up.value(&curves_up, as_of)?.amount();

        // Bump spot down
        let curves_down = bump_scalar_price(&context.curves, instrument.spot_id(), -bump_pct)?;
        let pv_down = instrument_down.value(&curves_down, as_of)?.amount();

        // Central difference: delta = (PV_up - PV_down) / (2 * bump_size)
        let delta = (pv_up - pv_down) / (2.0 * bump_size);

        Ok(delta)
    }
}

/// Generic gamma calculator using finite differences.
///
/// Computes gamma as the second derivative with respect to spot price:
/// Gamma = [Delta(spot + bump) - Delta(spot - bump)] / (2 * bump_size)
pub struct GenericFdGamma<I> {
    _phantom: PhantomData<I>,
}

impl<I> Default for GenericFdGamma<I> {
    fn default() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<I> MetricCalculator for GenericFdGamma<I>
where
    I: Instrument + HasEquityUnderlying + HasPricingOverrides + Clone + 'static,
{
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let instrument: &I = context.instrument_as()?;
        let as_of = context.as_of;

        // Get current spot for bump size calculation
        let spot_scalar = context.curves.price(instrument.spot_id())?;
        let current_spot = match spot_scalar {
            finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
            finstack_core::market_data::scalars::MarketScalar::Price(m) => m.amount(),
        };

        // Calculate adaptive or fixed bump size (same logic as Delta)
        let bump_pct = if let Some(ref overrides) = context.pricing_overrides {
            let (spot_override, _, _) = get_bump_overrides(overrides);
            if overrides.adaptive_bumps {
                // Try to get vol, expiry, and day_count for adaptive calculation
                let day_count = get_instrument_day_count(instrument.as_any());
                let expiry = get_instrument_expiry_for_adaptive(instrument.as_any());
                let vol_id = get_instrument_vol_id(instrument.as_any());
                
                let time_to_expiry = if let (Some(dc), Some(exp)) = (day_count, expiry) {
                    dc.year_fraction(as_of, exp, finstack_core::dates::DayCountCtx::default()).ok().unwrap_or(0.0)
                } else {
                    0.0
                };
                
                let atm_vol = vol_id
                    .and_then(|vol_id| {
                        context.curves.surface_ref(vol_id.as_str()).ok()
                    })
                    .and_then(|vol_surface| {
                        if time_to_expiry > 0.0 {
                            Some(vol_surface.value_clamped(time_to_expiry, current_spot))
                        } else {
                            None
                        }
                    })
                    .unwrap_or(bump_sizes::VOLATILITY); // Fallback to default vol

                adaptive_spot_bump(current_spot, atm_vol, time_to_expiry, spot_override)
            } else {
                spot_override.unwrap_or(bump_sizes::SPOT)
            }
        } else {
            bump_sizes::SPOT
        };

        let bump_size = current_spot * bump_pct;

        // Compute delta at spot + bump
        let mut instrument_up = instrument.clone();
        instrument_up.pricing_overrides_mut().mc_seed_scenario = Some("gamma_up_up".to_string());
        let curves_up = bump_scalar_price(&context.curves, instrument.spot_id(), bump_pct)?;
        
        // Delta at spot_up: need two more bumps
        let mut instrument_up_up = instrument_up.clone();
        instrument_up_up.pricing_overrides_mut().mc_seed_scenario = Some("gamma_up_up".to_string());
        let mut instrument_up_down = instrument_up.clone();
        instrument_up_down.pricing_overrides_mut().mc_seed_scenario = Some("gamma_up_down".to_string());
        
        let pv_up_up = instrument_up_up.value(&bump_scalar_price(&curves_up, instrument.spot_id(), bump_pct)?, as_of)?.amount();
        let pv_up_down = instrument_up_down.value(&bump_scalar_price(&curves_up, instrument.spot_id(), -bump_pct)?, as_of)?.amount();
        let delta_up = (pv_up_up - pv_up_down) / (2.0 * bump_size);

        // Compute delta at spot - bump
        let mut instrument_down = instrument.clone();
        instrument_down.pricing_overrides_mut().mc_seed_scenario = Some("gamma_down_base".to_string());
        let curves_down = bump_scalar_price(&context.curves, instrument.spot_id(), -bump_pct)?;
        
        let mut instrument_down_up = instrument_down.clone();
        instrument_down_up.pricing_overrides_mut().mc_seed_scenario = Some("gamma_down_up".to_string());
        let mut instrument_down_down = instrument_down.clone();
        instrument_down_down.pricing_overrides_mut().mc_seed_scenario = Some("gamma_down_down".to_string());
        
        let pv_down_up = instrument_down_up.value(&bump_scalar_price(&curves_down, instrument.spot_id(), bump_pct)?, as_of)?.amount();
        let pv_down_down = instrument_down_down.value(&bump_scalar_price(&curves_down, instrument.spot_id(), -bump_pct)?, as_of)?.amount();
        let delta_down = (pv_down_up - pv_down_down) / (2.0 * bump_size);

        // Gamma = (Delta_up - Delta_down) / (2 * bump_size)
        let gamma = (delta_up - delta_down) / (2.0 * bump_size);

        Ok(gamma)
    }
}

// Note: Vega is not included as a generic calculator because volatility surface
// bumping is complex and requires instrument-specific handling (to_state(),
// from_grid(), etc.). Each instrument should implement its own vega calculator
// following the pattern seen in asian_option/metrics/vega.rs

