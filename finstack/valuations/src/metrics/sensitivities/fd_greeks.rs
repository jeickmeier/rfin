//! Generic finite difference greek calculators for equity instruments.
//!
//! Provides reusable implementations of Delta, Gamma, and Vega calculators
//! that work with any instrument implementing the required traits.
//!
//! This eliminates code duplication across exotic options (AsianOption, Autocallable,
//! BarrierOption, LookbackOption, etc.) that all use the same finite difference pattern.

use std::marker::PhantomData;

use crate::instruments::common::traits::{EquityDependencies, Instrument};
use crate::metrics::{bump_scalar_price, bump_sizes, scale_surface};
use crate::metrics::core::finite_difference::{
    adaptive_spot_bump, central_mixed, get_bump_overrides,
};
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::dates::{Date, DayCount};
use finstack_core::Result;

// ================================================================================================
// Traits for Instruments with Expiry and DayCount Information
// ================================================================================================

/// Trait for instruments that have an expiry date.
///
/// Used for adaptive bump size calculations based on time to expiry.
/// Instruments with shorter time to expiry typically require smaller bumps
/// to maintain numerical accuracy in finite difference calculations.
///
/// # Examples
///
/// Implementing for a custom option:
///
/// ```rust,ignore
/// use finstack_valuations::metrics::sensitivities::fd_greeks::HasExpiry;
/// use finstack_core::dates::Date;
///
/// struct CustomOption {
///     expiry: Date,
///     // ... other fields
/// }
///
/// impl HasExpiry for CustomOption {
///     fn expiry(&self) -> Date {
///         self.expiry
///     }
/// }
/// ```
pub trait HasExpiry {
    /// Returns the expiry date for this instrument.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use finstack_valuations::metrics::sensitivities::fd_greeks::HasExpiry;
    ///
    /// let expiry_date = instrument.expiry();
    /// ```
    fn expiry(&self) -> Date;
}

/// Trait for instruments that have a day count convention.
///
/// Used for computing time fractions in adaptive bump calculations.
/// The day count convention determines how time between dates is measured,
/// affecting the calculation of year fractions for time-to-expiry.
///
/// # Examples
///
/// Implementing for a custom option:
///
/// ```rust,ignore
/// use finstack_valuations::metrics::sensitivities::fd_greeks::HasDayCount;
/// use finstack_core::dates::DayCount;
///
/// struct CustomOption {
///     day_count: DayCount,
///     // ... other fields
/// }
///
/// impl HasDayCount for CustomOption {
///     fn day_count(&self) -> DayCount {
///         self.day_count
///     }
/// }
/// ```
pub trait HasDayCount {
    /// Returns the day count convention for this instrument.
    ///
    /// Common conventions include Act/365F, Act/360, 30/360, etc.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use finstack_valuations::metrics::sensitivities::fd_greeks::HasDayCount;
    ///
    /// let day_count = instrument.day_count();
    /// ```
    fn day_count(&self) -> DayCount;
}

/// Trait for instruments that have pricing overrides.
///
/// This trait allows generic metric calculators to set MC seed scenarios
/// and other pricing overrides for deterministic greek calculations.
/// Only implemented by instruments that use Monte Carlo pricing.
///
/// # Purpose
///
/// For Monte Carlo priced instruments, setting different random seeds for
/// up/down bumps ensures deterministic and numerically stable greeks.
/// Without this, random variation would contaminate the finite difference
/// calculations.
///
/// # Examples
///
/// Implementing for a Monte Carlo option:
///
/// ```rust,ignore
/// use finstack_valuations::metrics::sensitivities::fd_greeks::HasPricingOverrides;
/// use finstack_valuations::instruments::PricingOverrides;
///
/// struct McOption {
///     overrides: PricingOverrides,
///     // ... other fields
/// }
///
/// impl HasPricingOverrides for McOption {
///     fn pricing_overrides_mut(&mut self) -> &mut PricingOverrides {
///         &mut self.overrides
///     }
/// }
/// ```
pub trait HasPricingOverrides {
    /// Returns mutable access to pricing overrides.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use finstack_valuations::metrics::sensitivities::fd_greeks::HasPricingOverrides;
    ///
    /// // Set deterministic MC seed for greek calculation
    /// instrument.pricing_overrides_mut().mc_seed_scenario = Some("delta_up".to_string());
    /// ```
    fn pricing_overrides_mut(&mut self) -> &mut crate::instruments::PricingOverrides;
}

// ================================================================================================
// Generic FD Greeks Calculators
// ================================================================================================

/// Generic delta calculator using finite differences.
///
/// Calculates delta (price sensitivity to underlying spot) using the central
/// finite difference method. Works with any instrument that implements the
/// required traits: [`Instrument`], [`EquityDependencies`], [`HasExpiry`],
/// [`HasDayCount`], and [`HasPricingOverrides`].
///
/// # Mathematical Foundation
///
/// Delta is computed using the central finite difference formula:
/// ```text
/// Δ = (PV(S + h) - PV(S - h)) / (2h)
/// ```
/// where `S` is the spot price and `h` is the bump size.
///
/// # Type Parameters
///
/// * `I` - Instrument type that implements all required traits for delta calculation
///
/// # Examples
///
/// ```rust,ignore
/// use finstack_valuations::metrics::sensitivities::fd_greeks::GenericFdDelta;
/// use finstack_valuations::instruments::EquityOption;
///
/// // Create delta calculator for equity options
/// let calculator = GenericFdDelta::<EquityOption>::default();
///
/// // Register in metric registry
/// registry.register_metric(
///     MetricId::Delta,
///     Arc::new(calculator),
///     &["EquityOption"],
/// );
/// ```
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
    I: Instrument
        + EquityDependencies
        + HasExpiry
        + HasDayCount
        + HasPricingOverrides
        + Clone
        + 'static,
{
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let instrument: &I = context.instrument_as()?;
        let as_of = context.as_of;

        // Get equity dependencies
        let eq_deps = instrument.equity_dependencies();
        let spot_id = eq_deps.spot_id.as_ref().ok_or_else(|| {
            finstack_core::Error::Validation(
                "Instrument missing spot_id for delta calculation".to_string(),
            )
        })?;

        // Get current spot for bump size calculation
        let spot_scalar = context.curves.price(spot_id)?;
        let current_spot = match spot_scalar {
            finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
            finstack_core::market_data::scalars::MarketScalar::Price(m) => m.amount(),
        };

        // Calculate adaptive or fixed bump size
        let bump_pct = if let Some(ref overrides) = context.pricing_overrides {
            let bump_overrides = get_bump_overrides(overrides);
            if overrides.adaptive_bumps {
                // Use traits to get instrument properties for adaptive calculation
                let time_to_expiry = instrument
                    .day_count()
                    .year_fraction(
                        as_of,
                        instrument.expiry(),
                        finstack_core::dates::DayCountCtx::default(),
                    )
                    .ok()
                    .unwrap_or(0.0);

                let atm_vol = if time_to_expiry > 0.0 {
                    eq_deps
                        .vol_surface_id
                        .as_ref()
                        .and_then(|vol_id| context.curves.surface_ref(vol_id.as_str()).ok())
                        .map(|vol_surface| vol_surface.value_clamped(time_to_expiry, current_spot))
                        .unwrap_or(bump_sizes::VOLATILITY)
                } else {
                    bump_sizes::VOLATILITY
                };

                adaptive_spot_bump(atm_vol, time_to_expiry, bump_overrides.spot_pct)
            } else {
                bump_overrides.spot_pct.unwrap_or(bump_sizes::SPOT)
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
        let curves_up = bump_scalar_price(&context.curves, spot_id, bump_pct)?;
        let pv_up = instrument_up.value(&curves_up, as_of)?.amount();

        // Bump spot down
        let curves_down = bump_scalar_price(&context.curves, spot_id, -bump_pct)?;
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
    I: Instrument
        + EquityDependencies
        + HasExpiry
        + HasDayCount
        + HasPricingOverrides
        + Clone
        + 'static,
{
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let instrument: &I = context.instrument_as()?;
        let as_of = context.as_of;

        // Get equity dependencies
        let eq_deps = instrument.equity_dependencies();
        let spot_id = eq_deps.spot_id.as_ref().ok_or_else(|| {
            finstack_core::Error::Validation(
                "Instrument missing spot_id for gamma calculation".to_string(),
            )
        })?;

        // Get current spot for bump size calculation
        let spot_scalar = context.curves.price(spot_id)?;
        let current_spot = match spot_scalar {
            finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
            finstack_core::market_data::scalars::MarketScalar::Price(m) => m.amount(),
        };

        // Calculate adaptive or fixed bump size (same logic as Delta)
        let bump_pct = if let Some(ref overrides) = context.pricing_overrides {
            let bump_overrides = get_bump_overrides(overrides);
            if overrides.adaptive_bumps {
                // Use traits to get instrument properties for adaptive calculation
                let time_to_expiry = instrument
                    .day_count()
                    .year_fraction(
                        as_of,
                        instrument.expiry(),
                        finstack_core::dates::DayCountCtx::default(),
                    )
                    .ok()
                    .unwrap_or(0.0);

                let atm_vol = if time_to_expiry > 0.0 {
                    eq_deps
                        .vol_surface_id
                        .as_ref()
                        .and_then(|vol_id| context.curves.surface_ref(vol_id.as_str()).ok())
                        .map(|vol_surface| vol_surface.value_clamped(time_to_expiry, current_spot))
                        .unwrap_or(bump_sizes::VOLATILITY)
                } else {
                    bump_sizes::VOLATILITY
                };

                adaptive_spot_bump(atm_vol, time_to_expiry, bump_overrides.spot_pct)
            } else {
                bump_overrides.spot_pct.unwrap_or(bump_sizes::SPOT)
            }
        } else {
            bump_sizes::SPOT
        };

        let bump_size = current_spot * bump_pct;

        // Compute delta at spot + bump
        let mut instrument_up = instrument.clone();
        instrument_up.pricing_overrides_mut().mc_seed_scenario = Some("gamma_up_up".to_string());
        let curves_up = bump_scalar_price(&context.curves, spot_id, bump_pct)?;

        // Delta at spot_up: need two more bumps
        let mut instrument_up_up = instrument_up.clone();
        instrument_up_up.pricing_overrides_mut().mc_seed_scenario = Some("gamma_up_up".to_string());
        let mut instrument_up_down = instrument_up.clone();
        instrument_up_down.pricing_overrides_mut().mc_seed_scenario =
            Some("gamma_up_down".to_string());

        let pv_up_up = instrument_up_up
            .value(&bump_scalar_price(&curves_up, spot_id, bump_pct)?, as_of)?
            .amount();
        let pv_up_down = instrument_up_down
            .value(&bump_scalar_price(&curves_up, spot_id, -bump_pct)?, as_of)?
            .amount();
        let delta_up = (pv_up_up - pv_up_down) / (2.0 * bump_size);

        // Compute delta at spot - bump
        let mut instrument_down = instrument.clone();
        instrument_down.pricing_overrides_mut().mc_seed_scenario =
            Some("gamma_down_base".to_string());
        let curves_down = bump_scalar_price(&context.curves, spot_id, -bump_pct)?;

        let mut instrument_down_up = instrument_down.clone();
        instrument_down_up.pricing_overrides_mut().mc_seed_scenario =
            Some("gamma_down_up".to_string());
        let mut instrument_down_down = instrument_down.clone();
        instrument_down_down
            .pricing_overrides_mut()
            .mc_seed_scenario = Some("gamma_down_down".to_string());

        let pv_down_up = instrument_down_up
            .value(&bump_scalar_price(&curves_down, spot_id, bump_pct)?, as_of)?
            .amount();
        let pv_down_down = instrument_down_down
            .value(&bump_scalar_price(&curves_down, spot_id, -bump_pct)?, as_of)?
            .amount();
        let delta_down = (pv_down_up - pv_down_down) / (2.0 * bump_size);

        // Gamma = (Delta_up - Delta_down) / (2 * bump_size)
        let gamma = (delta_up - delta_down) / (2.0 * bump_size);

        Ok(gamma)
    }
}

/// Generic vega calculator using finite differences on a volatility surface.
///
/// Works with any instrument that has an associated volatility surface.
pub struct GenericFdVega<I> {
    _phantom: PhantomData<I>,
}

impl<I> Default for GenericFdVega<I> {
    fn default() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<I> MetricCalculator for GenericFdVega<I>
where
    I: Instrument + EquityDependencies + HasPricingOverrides + Clone + 'static,
{
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let instrument: &I = context.instrument_as()?;
        let as_of = context.as_of;
        let base_pv = context.base_value.amount();

        // Get equity dependencies
        let eq_deps = instrument.equity_dependencies();

        // Get vol surface id from instrument
        let Some(ref vol_surface_id) = eq_deps.vol_surface_id else {
            tracing::warn!(
                instrument_type = std::any::type_name::<I>(),
                "GenericFdVega: No vol surface ID found for instrument, returning 0.0"
            );
            return Ok(0.0);
        };

        // Determine bump size (relative scale of vols)
        let bump_pct = if let Some(ref overrides) = context.pricing_overrides {
            // Prefer explicit override; otherwise use default 1% vol scale
            overrides.vol_bump_pct.unwrap_or(bump_sizes::VOLATILITY)
        } else {
            bump_sizes::VOLATILITY
        };

        let mut inst_up = instrument.clone();
        inst_up.pricing_overrides_mut().mc_seed_scenario = Some("vega_up".to_string());

        let curves_up = scale_surface(&context.curves, vol_surface_id.as_str(), 1.0 + bump_pct)?;
        let pv_up = inst_up.value(&curves_up, as_of)?.amount();

        // Vega per 1% vol scaling
        Ok((pv_up - base_pv) / bump_pct)
    }
}

/// Generic volga (∂²V/∂σ²) via central differences on a volatility surface.
pub struct GenericFdVolga<I> {
    _phantom: PhantomData<I>,
}

impl<I> Default for GenericFdVolga<I> {
    fn default() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<I> MetricCalculator for GenericFdVolga<I>
where
    I: Instrument + EquityDependencies + HasPricingOverrides + Clone + 'static,
{
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let instrument: &I = context.instrument_as()?;
        let as_of = context.as_of;
        let base_pv = context.base_value.amount();

        // Get equity dependencies
        let eq_deps = instrument.equity_dependencies();

        // Get vol surface id from instrument
        let Some(ref vol_surface_id) = eq_deps.vol_surface_id else {
            tracing::warn!(
                instrument_type = std::any::type_name::<I>(),
                "GenericFdVolga: No vol surface ID found for instrument, returning 0.0"
            );
            return Ok(0.0);
        };

        let bump_pct = if let Some(ref overrides) = context.pricing_overrides {
            overrides.vol_bump_pct.unwrap_or(bump_sizes::VOLATILITY)
        } else {
            bump_sizes::VOLATILITY
        };

        let mut inst_up = instrument.clone();
        inst_up.pricing_overrides_mut().mc_seed_scenario = Some("volga_up".to_string());
        let mut inst_down = instrument.clone();
        inst_down.pricing_overrides_mut().mc_seed_scenario = Some("volga_down".to_string());

        let curves_up = scale_surface(&context.curves, vol_surface_id.as_str(), 1.0 + bump_pct)?;
        let curves_down = scale_surface(&context.curves, vol_surface_id.as_str(), 1.0 - bump_pct)?;

        let pv_up = inst_up.value(&curves_up, as_of)?.amount();
        let pv_down = inst_down.value(&curves_down, as_of)?.amount();

        Ok((pv_up - 2.0 * base_pv + pv_down) / (bump_pct * bump_pct))
    }
}

/// Generic vanna (∂²V/∂S∂σ) via central mixed differences on spot and vol.
pub struct GenericFdVanna<I> {
    _phantom: PhantomData<I>,
}

impl<I> Default for GenericFdVanna<I> {
    fn default() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<I> MetricCalculator for GenericFdVanna<I>
where
    I: Instrument
        + EquityDependencies
        + HasExpiry
        + HasDayCount
        + HasPricingOverrides
        + Clone
        + 'static,
{
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let instrument: &I = context.instrument_as()?;
        let as_of = context.as_of;

        // Get equity dependencies
        let eq_deps = instrument.equity_dependencies();
        let spot_id = eq_deps.spot_id.as_ref().ok_or_else(|| {
            finstack_core::Error::Validation(
                "Instrument missing spot_id for vanna calculation".to_string(),
            )
        })?;
        let vol_surface_id = eq_deps.vol_surface_id.as_ref().ok_or_else(|| {
            finstack_core::Error::Validation(
                "Instrument missing vol_surface_id for vanna calculation".to_string(),
            )
        })?;

        // Spot level for bump sizing
        let spot_scalar = context.curves.price(spot_id)?;
        let current_spot = match spot_scalar {
            finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
            finstack_core::market_data::scalars::MarketScalar::Price(m) => m.amount(),
        };

        // Time to expiry and ATM vol for absolute denominators
        let t = instrument
            .day_count()
            .year_fraction(
                as_of,
                instrument.expiry(),
                finstack_core::dates::DayCountCtx::default(),
            )
            .ok()
            .unwrap_or(0.0);

        let atm_vol = if t > 0.0 {
            context
                .curves
                .surface_ref(vol_surface_id.as_str())
                .ok()
                .map(|surf| surf.value_clamped(t, current_spot))
                .unwrap_or(0.2)
        } else {
            0.2
        };

        // Bump sizes
        let (spot_bump_pct, vol_bump_pct) = if let Some(ref overrides) = context.pricing_overrides {
            let bump_overrides = get_bump_overrides(overrides);
            (
                bump_overrides.spot_pct.unwrap_or(bump_sizes::SPOT),
                bump_overrides.vol_pct.unwrap_or(bump_sizes::VOLATILITY),
            )
        } else {
            (bump_sizes::SPOT, bump_sizes::VOLATILITY)
        };

        let h_abs = current_spot * spot_bump_pct; // absolute spot change
        let k_abs = (atm_vol * vol_bump_pct).abs().max(1e-12); // absolute vol change

        // Prepare evaluators for four combinations
        let su_vu = {
            let mut inst = instrument.clone();
            inst.pricing_overrides_mut().mc_seed_scenario = Some("vanna_su_vu".to_string());
            let curves = scale_surface(
                &bump_scalar_price(&context.curves, spot_id, spot_bump_pct)?,
                vol_surface_id.as_str(),
                1.0 + vol_bump_pct,
            )?;
            move || inst.value(&curves, as_of).map(|m| m.amount())
        };

        let su_vd = {
            let mut inst = instrument.clone();
            inst.pricing_overrides_mut().mc_seed_scenario = Some("vanna_su_vd".to_string());
            let curves = scale_surface(
                &bump_scalar_price(&context.curves, spot_id, spot_bump_pct)?,
                vol_surface_id.as_str(),
                1.0 - vol_bump_pct,
            )?;
            move || inst.value(&curves, as_of).map(|m| m.amount())
        };

        let sd_vu = {
            let mut inst = instrument.clone();
            inst.pricing_overrides_mut().mc_seed_scenario = Some("vanna_sd_vu".to_string());
            let curves = scale_surface(
                &bump_scalar_price(&context.curves, spot_id, -spot_bump_pct)?,
                vol_surface_id.as_str(),
                1.0 + vol_bump_pct,
            )?;
            move || inst.value(&curves, as_of).map(|m| m.amount())
        };

        let sd_vd = {
            let mut inst = instrument.clone();
            inst.pricing_overrides_mut().mc_seed_scenario = Some("vanna_sd_vd".to_string());
            let curves = scale_surface(
                &bump_scalar_price(&context.curves, spot_id, -spot_bump_pct)?,
                vol_surface_id.as_str(),
                1.0 - vol_bump_pct,
            )?;
            move || inst.value(&curves, as_of).map(|m| m.amount())
        };

        central_mixed(su_vu, su_vd, sd_vu, sd_vd, h_abs, k_abs)
    }
}
