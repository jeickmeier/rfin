//! Generic finite difference greek calculators for equity instruments.
//!
//! Provides reusable implementations of Delta, Gamma, and Vega calculators
//! that work with any instrument implementing the required traits.
//!
//! This eliminates code duplication across exotic options (AsianOption, Autocallable,
//! BarrierOption, LookbackOption, etc.) that all use the same finite difference pattern.

use std::marker::PhantomData;

use crate::instruments::common::traits::{EquityDependencies, Instrument};
use crate::metrics::core::finite_difference::central_mixed;
use crate::metrics::sensitivities::config as sens_config;
use crate::metrics::{bump_scalar_price, bump_surface_vol_absolute};
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
/// use finstack_valuations::metrics::HasExpiry;
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
#[allow(dead_code)]
pub trait HasExpiry {
    /// Returns the expiry date for this instrument.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use finstack_valuations::metrics::HasExpiry;
    /// use finstack_core::dates::Date;
    ///
    /// # let instrument: &dyn HasExpiry = todo!("an instrument that has an expiry date");
    /// let expiry_date: Date = instrument.expiry();
    /// # let _ = expiry_date;
    /// ```
    #[allow(dead_code)]
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
/// use finstack_valuations::metrics::HasDayCount;
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
#[allow(dead_code)]
pub trait HasDayCount {
    /// Returns the day count convention for this instrument.
    ///
    /// Common conventions include Act/365F, Act/360, 30/360, etc.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use finstack_valuations::metrics::HasDayCount;
    /// use finstack_core::dates::DayCount;
    ///
    /// # let instrument: &dyn HasDayCount = todo!("an instrument that specifies a day count");
    /// let day_count: DayCount = instrument.day_count();
    /// # let _ = day_count;
    /// ```
    #[allow(dead_code)]
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
/// use finstack_valuations::metrics::HasPricingOverrides;
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
    /// use finstack_valuations::metrics::HasPricingOverrides;
    ///
    /// // Set deterministic MC seed for greek calculation
    /// # let instrument: &mut dyn HasPricingOverrides = todo!("a Monte Carlo priced instrument");
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
/// use finstack_valuations::instruments::BarrierOption;
/// use finstack_valuations::metrics::{GenericFdDelta, MetricId, MetricRegistry};
/// use finstack_valuations::pricer::InstrumentType;
/// use std::sync::Arc;
///
/// // Create delta calculator for barrier options (generic FD greeks apply to exotics)
/// let calculator = GenericFdDelta::<BarrierOption>::default();
///
/// // Register in metric registry
/// let mut registry = MetricRegistry::new();
/// registry.register_metric(
///     MetricId::Delta,
///     Arc::new(calculator),
///     &[InstrumentType::BarrierOption],
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
        let defaults = sens_config::from_finstack_config_or_default(context.config())?;

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

        // Fixed bump size from `FinstackConfig` (user-facing, reproducible).
        let bump_pct = defaults.spot_bump_pct;

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
        let pv_up = instrument_up.value_raw(&curves_up, as_of)?;

        // Bump spot down
        let curves_down = bump_scalar_price(&context.curves, spot_id, -bump_pct)?;
        let pv_down = instrument_down.value_raw(&curves_down, as_of)?;

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
        let defaults = sens_config::from_finstack_config_or_default(context.config())?;

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

        // Fixed bump size from `FinstackConfig` (user-facing, reproducible).
        let bump_pct = defaults.spot_bump_pct;

        let bump_size = current_spot * bump_pct;

        // Use the 3-point central difference formula directly:
        //
        //   Γ ≈ (PV(S+h) - 2 PV(S) + PV(S-h)) / h²
        //
        // This avoids "bump-of-bump" scaling artifacts when bumps are applied multiplicatively
        // (percentage bumps) and yields exact results for quadratic payoffs.
        let base_pv = instrument.value_raw(&context.curves, as_of)?;

        let mut instrument_up = instrument.clone();
        instrument_up.pricing_overrides_mut().mc_seed_scenario = Some("gamma_up".to_string());
        let pv_up = instrument_up.value_raw(
            &bump_scalar_price(&context.curves, spot_id, bump_pct)?,
            as_of,
        )?;

        let mut instrument_down = instrument.clone();
        instrument_down.pricing_overrides_mut().mc_seed_scenario = Some("gamma_down".to_string());
        let pv_down = instrument_down.value_raw(
            &bump_scalar_price(&context.curves, spot_id, -bump_pct)?,
            as_of,
        )?;

        let gamma = (pv_up - 2.0 * base_pv + pv_down) / (bump_size * bump_size);

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
        let defaults = sens_config::from_finstack_config_or_default(context.config())?;
        let base_pv = instrument.value_raw(&context.curves, as_of)?;

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

        // Fixed bump size from `FinstackConfig` (user-facing, reproducible).
        // Interpreted as an **absolute** implied vol bump in decimal units (e.g., 0.01 = +1 vol point).
        let bump_abs = defaults.vol_bump_pct;

        let mut inst_up = instrument.clone();
        inst_up.pricing_overrides_mut().mc_seed_scenario = Some("vega_up".to_string());

        let curves_up =
            bump_surface_vol_absolute(&context.curves, vol_surface_id.as_str(), bump_abs)?;
        let pv_up = inst_up.value_raw(&curves_up, as_of)?;

        // Vega is ∂V/∂σ (per absolute vol unit). With the default bump of 0.01, this is the
        // market-standard “per 1 vol point” sensitivity.
        Ok((pv_up - base_pv) / bump_abs)
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
        let defaults = sens_config::from_finstack_config_or_default(context.config())?;
        let base_pv = instrument.value_raw(&context.curves, as_of)?;

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

        // Absolute implied vol bump (vol points).
        let bump_abs = defaults.vol_bump_pct;

        let mut inst_up = instrument.clone();
        inst_up.pricing_overrides_mut().mc_seed_scenario = Some("volga_up".to_string());
        let mut inst_down = instrument.clone();
        inst_down.pricing_overrides_mut().mc_seed_scenario = Some("volga_down".to_string());

        let curves_up =
            bump_surface_vol_absolute(&context.curves, vol_surface_id.as_str(), bump_abs)?;
        let curves_down =
            bump_surface_vol_absolute(&context.curves, vol_surface_id.as_str(), -bump_abs)?;

        let pv_up = inst_up.value_raw(&curves_up, as_of)?;
        let pv_down = inst_down.value_raw(&curves_down, as_of)?;

        Ok((pv_up - 2.0 * base_pv + pv_down) / (bump_abs * bump_abs))
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
        let defaults = sens_config::from_finstack_config_or_default(context.config())?;

        // If expired, vanna is zero (avoid bumping / repricing beyond expiry).
        let t = instrument.day_count().year_fraction(
            as_of,
            instrument.expiry(),
            finstack_core::dates::DayCountCtx::default(),
        )?;
        if t <= 0.0 {
            return Ok(0.0);
        }

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

        // Bump sizes
        let (spot_bump_pct, vol_bump_abs) = (defaults.spot_bump_pct, defaults.vol_bump_pct);

        let h_abs = current_spot * spot_bump_pct; // absolute spot change
        let k_abs = vol_bump_abs; // absolute vol change (vol points)

        // Prepare evaluators for four combinations
        let su_vu = {
            let mut inst = instrument.clone();
            inst.pricing_overrides_mut().mc_seed_scenario = Some("vanna_su_vu".to_string());
            let curves = bump_surface_vol_absolute(
                &bump_scalar_price(&context.curves, spot_id, spot_bump_pct)?,
                vol_surface_id.as_str(),
                k_abs,
            )?;
            move || inst.value_raw(&curves, as_of)
        };

        let su_vd = {
            let mut inst = instrument.clone();
            inst.pricing_overrides_mut().mc_seed_scenario = Some("vanna_su_vd".to_string());
            let curves = bump_surface_vol_absolute(
                &bump_scalar_price(&context.curves, spot_id, spot_bump_pct)?,
                vol_surface_id.as_str(),
                -k_abs,
            )?;
            move || inst.value_raw(&curves, as_of)
        };

        let sd_vu = {
            let mut inst = instrument.clone();
            inst.pricing_overrides_mut().mc_seed_scenario = Some("vanna_sd_vu".to_string());
            let curves = bump_surface_vol_absolute(
                &bump_scalar_price(&context.curves, spot_id, -spot_bump_pct)?,
                vol_surface_id.as_str(),
                k_abs,
            )?;
            move || inst.value_raw(&curves, as_of)
        };

        let sd_vd = {
            let mut inst = instrument.clone();
            inst.pricing_overrides_mut().mc_seed_scenario = Some("vanna_sd_vd".to_string());
            let curves = bump_surface_vol_absolute(
                &bump_scalar_price(&context.curves, spot_id, -spot_bump_pct)?,
                vol_surface_id.as_str(),
                -k_abs,
            )?;
            move || inst.value_raw(&curves, as_of)
        };

        central_mixed(su_vu, su_vd, sd_vu, sd_vd, h_abs, k_abs)
    }
}

// ================================================================================================
// Unit tests (internal)
// ================================================================================================

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic, dead_code)]
mod tests {
    use super::*;

    use crate::instruments::common::traits::{
        Attributes, EquityDependencies, EquityInstrumentDeps,
    };
    use crate::instruments::PricingOverrides;
    use crate::metrics::{MetricContext, MetricId, MetricRegistry};
    use crate::pricer::InstrumentType;
    use finstack_core::currency::Currency;
    use finstack_core::market_data::context::MarketContext;
    use finstack_core::market_data::scalars::MarketScalar;
    use finstack_core::money::Money;
    use std::sync::Arc;
    use time::macros::date;

    #[allow(dead_code)]
    #[derive(Clone)]
    struct TestFdInstrument {
        id: String,
        expiry: Date,
        day_count: DayCount,
        spot_id: String,
        overrides: PricingOverrides,
        attributes: Attributes,
    }

    impl TestFdInstrument {
        fn new(id: &str, expiry: Date, spot_id: &str) -> Self {
            Self {
                id: id.to_string(),
                expiry,
                day_count: DayCount::Act365F,
                spot_id: spot_id.to_string(),
                overrides: PricingOverrides::default(),
                attributes: Attributes::new(),
            }
        }
    }

    #[allow(dead_code)]
    #[derive(Clone)]
    struct RoundingSensitiveInstrument {
        id: String,
        expiry: Date,
        day_count: DayCount,
        spot_id: String,
        overrides: PricingOverrides,
        attributes: Attributes,
    }

    impl RoundingSensitiveInstrument {
        fn new(id: &str, expiry: Date, spot_id: &str) -> Self {
            Self {
                id: id.to_string(),
                expiry,
                day_count: DayCount::Act365F,
                spot_id: spot_id.to_string(),
                overrides: PricingOverrides::default(),
                attributes: Attributes::new(),
            }
        }

        fn raw_pv(&self, market: &MarketContext) -> finstack_core::Result<f64> {
            let spot_scalar = market.price(self.spot_id.as_str())?;
            let spot = match spot_scalar {
                MarketScalar::Price(m) => m.amount(),
                MarketScalar::Unitless(v) => *v,
            };
            Ok(spot * 0.00003)
        }
    }

    impl EquityDependencies for TestFdInstrument {
        fn equity_dependencies(&self) -> EquityInstrumentDeps {
            EquityInstrumentDeps::builder()
                .spot(self.spot_id.clone())
                .build()
        }
    }

    impl HasExpiry for TestFdInstrument {
        fn expiry(&self) -> Date {
            self.expiry
        }
    }

    impl HasDayCount for TestFdInstrument {
        fn day_count(&self) -> DayCount {
            self.day_count
        }
    }

    impl HasPricingOverrides for TestFdInstrument {
        fn pricing_overrides_mut(&mut self) -> &mut PricingOverrides {
            &mut self.overrides
        }
    }

    impl EquityDependencies for RoundingSensitiveInstrument {
        fn equity_dependencies(&self) -> EquityInstrumentDeps {
            EquityInstrumentDeps::builder()
                .spot(self.spot_id.clone())
                .build()
        }
    }

    impl HasExpiry for RoundingSensitiveInstrument {
        fn expiry(&self) -> Date {
            self.expiry
        }
    }

    impl HasDayCount for RoundingSensitiveInstrument {
        fn day_count(&self) -> DayCount {
            self.day_count
        }
    }

    impl HasPricingOverrides for RoundingSensitiveInstrument {
        fn pricing_overrides_mut(&mut self) -> &mut PricingOverrides {
            &mut self.overrides
        }
    }

    impl crate::instruments::common::traits::Instrument for TestFdInstrument {
        fn id(&self) -> &str {
            &self.id
        }

        fn key(&self) -> InstrumentType {
            InstrumentType::Equity
        }

        fn as_any(&self) -> &dyn std::any::Any {
            self
        }

        fn clone_box(&self) -> Box<dyn crate::instruments::common::traits::Instrument> {
            Box::new(self.clone())
        }

        fn attributes(&self) -> &Attributes {
            &self.attributes
        }

        fn attributes_mut(&mut self) -> &mut Attributes {
            &mut self.attributes
        }

        fn value(&self, market: &MarketContext, as_of: Date) -> finstack_core::Result<Money> {
            if as_of >= self.expiry {
                return Ok(Money::new(0.0, Currency::USD));
            }
            let spot_scalar = market.price(self.spot_id.as_str())?;
            let spot = match spot_scalar {
                MarketScalar::Price(m) => m.amount(),
                MarketScalar::Unitless(v) => *v,
            };
            // Simple analytic PV = S^2 (currency USD)
            Ok(Money::new(spot * spot, Currency::USD))
        }

        fn price_with_metrics(
            &self,
            market: &MarketContext,
            as_of: Date,
            metrics: &[MetricId],
        ) -> finstack_core::Result<crate::results::ValuationResult> {
            let base_value = self.value(market, as_of)?;
            crate::instruments::common::helpers::build_with_metrics_dyn(
                Arc::from(self.clone_box()),
                Arc::new(market.clone()),
                as_of,
                base_value,
                metrics,
                None,
                None,
            )
        }
    }

    impl crate::instruments::common::traits::Instrument for RoundingSensitiveInstrument {
        fn id(&self) -> &str {
            &self.id
        }

        fn key(&self) -> InstrumentType {
            InstrumentType::Equity
        }

        fn as_any(&self) -> &dyn std::any::Any {
            self
        }

        fn clone_box(&self) -> Box<dyn crate::instruments::common::traits::Instrument> {
            Box::new(self.clone())
        }

        fn attributes(&self) -> &Attributes {
            &self.attributes
        }

        fn attributes_mut(&mut self) -> &mut Attributes {
            &mut self.attributes
        }

        fn value(&self, market: &MarketContext, _as_of: Date) -> finstack_core::Result<Money> {
            // This intentionally rounds away most of the PV to exercise the raw path
            let raw = self.raw_pv(market)?;
            Ok(Money::new(raw, Currency::USD))
        }

        fn value_raw(&self, market: &MarketContext, _as_of: Date) -> finstack_core::Result<f64> {
            self.raw_pv(market)
        }

        fn price_with_metrics(
            &self,
            market: &MarketContext,
            as_of: Date,
            metrics: &[MetricId],
        ) -> finstack_core::Result<crate::results::ValuationResult> {
            let base_value = self.value(market, as_of)?;
            crate::instruments::common::helpers::build_with_metrics_dyn(
                Arc::from(self.clone_box()),
                Arc::new(market.clone()),
                as_of,
                base_value,
                metrics,
                None,
                None,
            )
        }
    }

    fn registry_for_test<I>() -> MetricRegistry
    where
        I: crate::instruments::common::traits::Instrument
            + EquityDependencies
            + HasExpiry
            + HasDayCount
            + HasPricingOverrides
            + Clone
            + 'static,
    {
        let mut registry = MetricRegistry::new();
        registry.register_metric(
            MetricId::Delta,
            Arc::new(GenericFdDelta::<I>::default()),
            &[InstrumentType::Equity],
        );
        registry.register_metric(
            MetricId::Gamma,
            Arc::new(GenericFdGamma::<I>::default()),
            &[InstrumentType::Equity],
        );
        registry
    }

    fn market_with_spot(spot_id: &str, price: f64) -> MarketContext {
        MarketContext::new().insert_price(
            spot_id,
            MarketScalar::Price(Money::new(price, Currency::USD)),
        )
    }

    #[test]
    fn fd_delta_matches_analytic_for_quadratic_pv() {
        let as_of = date!(2025 - 01 - 01);
        let spot = 100.0;
        let inst = TestFdInstrument::new("FD-TEST", date!(2026 - 01 - 01), "SPOT");
        let market = market_with_spot("SPOT", spot);

        let base_value = inst.value(&market, as_of).expect("base pv");
        let registry = registry_for_test::<TestFdInstrument>();
        let mut ctx = MetricContext::new(
            Arc::new(inst),
            Arc::new(market),
            as_of,
            base_value,
            MetricContext::default_config(),
        );

        let result = registry
            .compute(&[MetricId::Delta], &mut ctx)
            .expect("delta");
        let delta = *result.get(&MetricId::Delta).expect("delta value");

        // For PV = S^2, dPV/dS = 2S.
        assert!((delta - 2.0 * spot).abs() < 1e-8, "delta mismatch");
    }

    #[test]
    fn fd_gamma_matches_analytic_for_quadratic_pv() {
        let as_of = date!(2025 - 01 - 01);
        let spot = 80.0;
        let inst = TestFdInstrument::new("FD-TEST", date!(2026 - 01 - 01), "SPOT");
        let market = market_with_spot("SPOT", spot);

        let base_value = inst.value(&market, as_of).expect("base pv");
        let registry = registry_for_test::<TestFdInstrument>();
        let mut ctx = MetricContext::new(
            Arc::new(inst),
            Arc::new(market),
            as_of,
            base_value,
            MetricContext::default_config(),
        );

        let result = registry
            .compute(&[MetricId::Gamma], &mut ctx)
            .expect("gamma");
        let gamma = *result.get(&MetricId::Gamma).expect("gamma value");

        // For PV = S^2, d2PV/dS2 = 2.
        assert!((gamma - 2.0).abs() < 1e-8, "gamma mismatch");
    }

    #[test]
    fn delta_uses_raw_value_when_money_rounds_down() {
        let as_of = date!(2025 - 01 - 01);
        let spot = 100.0;
        let inst = RoundingSensitiveInstrument::new("ROUNDING", date!(2026 - 01 - 01), "SENS");
        let market = market_with_spot("SENS", spot);

        // Rounded Money path collapses to zero
        let base_value = inst.value(&market, as_of).expect("base pv");
        assert_eq!(base_value.amount(), 0.0, "rounded Money should be zero");

        let registry = registry_for_test::<RoundingSensitiveInstrument>();
        let mut ctx = MetricContext::new(
            Arc::new(inst),
            Arc::new(market),
            as_of,
            base_value,
            MetricContext::default_config(),
        );

        let result = registry
            .compute(&[MetricId::Delta], &mut ctx)
            .expect("delta");
        let delta = *result.get(&MetricId::Delta).expect("delta value");

        // Raw path preserves sensitivity: raw PV = spot * 0.00003, so delta ≈ 3e-5
        assert!(
            (delta - 0.00003).abs() < 1e-8,
            "delta should reflect raw pv"
        );
    }

    #[test]
    fn fd_greeks_zero_when_expired() {
        let as_of = date!(2027 - 01 - 02);
        let spot = 50.0;
        let inst = TestFdInstrument::new("FD-TEST", date!(2027 - 01 - 01), "SPOT");
        let market = market_with_spot("SPOT", spot);

        let base_value = inst.value(&market, as_of).expect("base pv");
        assert_eq!(base_value.amount(), 0.0, "expired base pv should be zero");

        let registry = registry_for_test::<TestFdInstrument>();
        let mut ctx = MetricContext::new(
            Arc::new(inst),
            Arc::new(market),
            as_of,
            base_value,
            MetricContext::default_config(),
        );

        let result = registry
            .compute(&[MetricId::Delta, MetricId::Gamma], &mut ctx)
            .expect("greeks");
        assert_eq!(
            *result.get(&MetricId::Delta).expect("delta"),
            0.0,
            "expired delta should be zero"
        );
        assert_eq!(
            *result.get(&MetricId::Gamma).expect("gamma"),
            0.0,
            "expired gamma should be zero"
        );
    }
}
