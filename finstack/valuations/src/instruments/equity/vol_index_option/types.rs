//! Volatility Index Option types and implementation.
//!
//! Defines the `VolatilityIndexOption` instrument for options on VIX, VXN,
//! VSTOXX, and similar volatility indices. These options are European-style,
//! cash-settled, and priced using the Black model.
//!
//! # Contract Specifications
//!
//! VIX options are traded on CBOE with the following standard specs:
//! - Multiplier: $100 per index point
//! - Settlement: Cash-settled to VRO (VIX Settlement Value)
//! - Style: European only
//!
//! # Pricing Model
//!
//! VIX options use the Black model (not Black-Scholes) because:
//! 1. The underlying is the VIX forward, not spot
//! 2. VIX has no cost of carry or dividends
//! 3. The forward price is directly observable from futures
//!
//! ```text
//! Call = DF × [F × N(d1) - K × N(d2)]
//! Put  = DF × [K × N(-d2) - F × N(-d1)]
//!
//! where:
//!   d1 = [ln(F/K) + 0.5σ²T] / (σ√T)
//!   d2 = d1 - σ√T
//!   DF = discount factor to expiry
//! ```
//!
//! # Vol-of-Vol Surface
//!
//! The σ in the Black model is the "vol-of-vol" - the volatility of the
//! volatility index itself. This is typically quoted in a 2D surface indexed
//! by expiry and strike (or moneyness).
//!
//! # References
//!
//! - CBOE (2019). "VIX Options Contract Specifications."
//! - Carr, P., & Lee, R. (2009). "Volatility Derivatives."
//!   *Annual Review of Financial Economics*, 1, 319-339.

use crate::cashflow::traits::CashflowProvider;
use crate::impl_instrument_base;
use crate::instruments::common_impl::models::volatility::black::{d1_black76, d2_black76};
use crate::instruments::common_impl::traits::Attributes;
use crate::instruments::{ExerciseStyle, OptionType};
use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount, DayCountCtx};
use finstack_core::market_data::context::MarketContext;
use finstack_core::math::norm_cdf;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use time::macros::date;

/// Volatility Index Option instrument.
///
/// European-style options on volatility indices (VIX, VXN, VSTOXX).
/// Priced using the Black model with the vol index forward as underlying.
///
/// # Examples
///
/// ```rust
/// use finstack_valuations::instruments::equity::vol_index_option::VolatilityIndexOption;
/// use finstack_valuations::instruments::OptionType;
/// use finstack_core::currency::Currency;
/// use finstack_core::dates::{Date, DayCount};
/// use finstack_core::money::Money;
/// use finstack_core::types::{CurveId, InstrumentId};
/// use time::Month;
///
/// let option = VolatilityIndexOption::builder()
///     .id(InstrumentId::new("VIX-CALL-20-2025M03"))
///     .notional(Money::new(10_000.0, Currency::USD))
///     .strike(20.0)
///     .option_type(OptionType::Call)
///     .expiry(Date::from_calendar_date(2025, Month::March, 19).unwrap())
///     .day_count(DayCount::Act365F)
///     .discount_curve_id(CurveId::new("USD-OIS"))
///     .vol_index_curve_id(CurveId::new("VIX"))
///     .vol_of_vol_surface_id(CurveId::new("VIX-VOLVOL"))
///     .build()
///     .expect("Valid option");
/// ```
#[derive(
    Clone, Debug, finstack_valuations_macros::FinancialBuilder, serde::Serialize, serde::Deserialize,
)]
#[serde(deny_unknown_fields)]
pub struct VolatilityIndexOption {
    /// Unique identifier.
    pub id: InstrumentId,
    /// Notional exposure in currency units.
    pub notional: Money,
    /// Strike price (in index points, e.g., 20.0).
    pub strike: f64,
    /// Option type (Call or Put).
    pub option_type: OptionType,
    /// Exercise style (always European for VIX options).
    #[builder(default)]
    #[serde(default)]
    pub exercise_style: ExerciseStyle,
    /// Option expiry date.
    pub expiry: Date,
    /// Settlement date (typically same as expiry for cash-settled).
    #[builder(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settlement_date: Option<Date>,
    /// Contract specifications.
    #[builder(default)]
    #[serde(default)]
    pub contract_specs: VolIndexOptionSpecs,
    /// Day count convention (default: Act365F).
    pub day_count: DayCount,
    /// Discount curve identifier for present value calculations.
    pub discount_curve_id: CurveId,
    /// Volatility index forward curve identifier.
    pub vol_index_curve_id: CurveId,
    /// Vol-of-vol surface identifier for option implied volatility.
    pub vol_of_vol_surface_id: CurveId,
    /// Attributes for tagging and selection.
    #[builder(default)]
    #[serde(default)]
    #[builder(default)]
    pub pricing_overrides: crate::instruments::PricingOverrides,
    /// Attributes for scenario selection and tagging
    pub attributes: Attributes,
}

/// Contract specifications for volatility index options.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VolIndexOptionSpecs {
    /// Contract multiplier (USD per index point).
    /// VIX options standard: 100 (each point = $100)
    pub multiplier: f64,
    /// Index identifier (e.g., "VIX", "VXN", "VSTOXX").
    pub index_id: String,
}

impl Default for VolIndexOptionSpecs {
    fn default() -> Self {
        Self {
            multiplier: 100.0,
            index_id: "VIX".to_string(),
        }
    }
}

impl VolIndexOptionSpecs {
    /// Create specs for standard VIX options.
    pub fn vix() -> Self {
        Self::default()
    }

    /// Create specs for VSTOXX options.
    pub fn vstoxx() -> Self {
        Self {
            multiplier: 100.0,
            index_id: "VSTOXX".to_string(),
        }
    }
}

impl VolatilityIndexOption {
    /// Create a canonical example VIX call option for testing.
    pub fn example() -> Self {
        // SAFETY: All inputs are compile-time validated constants
        Self::builder()
            .id(InstrumentId::new("VIX-CALL-20-2025M03"))
            .notional(Money::new(10_000.0, Currency::USD))
            .strike(20.0)
            .option_type(OptionType::Call)
            .exercise_style(ExerciseStyle::European)
            .expiry(date!(2025 - 03 - 19))
            .contract_specs(VolIndexOptionSpecs::vix())
            .day_count(DayCount::Act365F)
            .discount_curve_id(CurveId::new("USD-OIS"))
            .vol_index_curve_id(CurveId::new("VIX"))
            .vol_of_vol_surface_id(CurveId::new("VIX-VOLVOL"))
            .attributes(Attributes::new())
            .build()
            .unwrap_or_else(|_| {
                unreachable!("Example VIX option with valid constants should never fail")
            })
    }

    /// Create a VIX call option.
    ///
    /// # Errors
    ///
    /// Returns an error if the builder fails validation.
    pub fn vix_call(
        id: impl Into<String>,
        strike: f64,
        expiry: Date,
        notional: Money,
    ) -> finstack_core::Result<Self> {
        Self::builder()
            .id(InstrumentId::new(id.into()))
            .notional(notional)
            .strike(strike)
            .option_type(OptionType::Call)
            .exercise_style(ExerciseStyle::European)
            .expiry(expiry)
            .contract_specs(VolIndexOptionSpecs::vix())
            .day_count(DayCount::Act365F)
            .discount_curve_id(CurveId::new("USD-OIS"))
            .vol_index_curve_id(CurveId::new("VIX"))
            .vol_of_vol_surface_id(CurveId::new("VIX-VOLVOL"))
            .attributes(Attributes::new())
            .build()
    }

    /// Create a VIX put option.
    ///
    /// # Errors
    ///
    /// Returns an error if the builder fails validation.
    pub fn vix_put(
        id: impl Into<String>,
        strike: f64,
        expiry: Date,
        notional: Money,
    ) -> finstack_core::Result<Self> {
        Self::builder()
            .id(InstrumentId::new(id.into()))
            .notional(notional)
            .strike(strike)
            .option_type(OptionType::Put)
            .exercise_style(ExerciseStyle::European)
            .expiry(expiry)
            .contract_specs(VolIndexOptionSpecs::vix())
            .day_count(DayCount::Act365F)
            .discount_curve_id(CurveId::new("USD-OIS"))
            .vol_index_curve_id(CurveId::new("VIX"))
            .vol_of_vol_surface_id(CurveId::new("VIX-VOLVOL"))
            .attributes(Attributes::new())
            .build()
    }

    /// Calculate the number of contracts based on notional.
    ///
    /// # Formula
    /// ```text
    /// contracts = notional / (multiplier × strike)
    /// ```
    pub fn num_contracts(&self) -> f64 {
        let contract_value = self.contract_specs.multiplier * self.strike;
        if contract_value > 0.0 {
            self.notional.amount() / contract_value
        } else {
            0.0
        }
    }

    /// Calculate the raw present value as f64.
    ///
    /// # Arguments
    ///
    /// * `context` - Market context with vol index curves and surfaces
    /// * `as_of` - Valuation date for time to expiry calculation
    pub fn npv_raw(&self, context: &MarketContext, as_of: Date) -> finstack_core::Result<f64> {
        // Get discount curve for DF
        let disc = context.get_discount(&self.discount_curve_id)?;

        // Get the vol index curve for forward level
        let vol_curve = context.get_vol_index(&self.vol_index_curve_id)?;

        // Get the vol-of-vol surface for implied vol
        let vol_surface = context.surface(&self.vol_of_vol_surface_id)?;

        // Calculate time to expiry from as_of date
        let t = self
            .day_count
            .year_fraction(as_of, self.expiry, DayCountCtx::default())?
            .max(0.0);

        if t <= 0.0 {
            // Option has expired, return intrinsic value
            let forward = vol_curve.spot_level();
            let intrinsic = match self.option_type {
                OptionType::Call => (forward - self.strike).max(0.0),
                OptionType::Put => (self.strike - forward).max(0.0),
            };
            return Ok(intrinsic * self.contract_specs.multiplier * self.num_contracts());
        }

        // Get forward volatility level at expiry
        let forward = vol_curve.forward_level(t);

        // Get vol-of-vol from surface (using strike and time to expiry)
        // Use value_clamped for flat extrapolation at boundaries
        let vol_of_vol = vol_surface.value_clamped(t, self.strike);

        // Get discount factor from as_of to expiry
        let df = disc.df_between_dates(as_of, self.expiry)?;

        // Black model price (undiscounted)
        let black_price = self.black_price(forward, vol_of_vol, t);

        // Total PV = Black_Price × Multiplier × Contracts × DF
        let contracts = self.num_contracts();
        let pv_total = black_price * self.contract_specs.multiplier * contracts * df;

        Ok(pv_total)
    }

    /// Calculate the Black model option price (undiscounted).
    ///
    /// # Arguments
    /// * `forward` - Forward vol index level
    /// * `sigma` - Vol-of-vol (volatility of the vol index)
    /// * `t` - Time to expiry in years
    ///
    /// # Returns
    /// Undiscounted option price per index point
    pub fn black_price(&self, forward: f64, sigma: f64, t: f64) -> f64 {
        if t <= 0.0 || sigma <= 0.0 {
            return match self.option_type {
                OptionType::Call => (forward - self.strike).max(0.0),
                OptionType::Put => (self.strike - forward).max(0.0),
            };
        }

        let d1 = d1_black76(forward, self.strike, sigma, t);
        let d2 = d2_black76(forward, self.strike, sigma, t);

        match self.option_type {
            OptionType::Call => forward * norm_cdf(d1) - self.strike * norm_cdf(d2),
            OptionType::Put => self.strike * norm_cdf(-d2) - forward * norm_cdf(-d1),
        }
    }

    /// Get the forward volatility level at expiry.
    ///
    /// # Arguments
    /// * `context` - Market context with vol index curves
    /// * `as_of` - Valuation date for time to expiry calculation
    pub fn forward_vol(&self, context: &MarketContext, as_of: Date) -> finstack_core::Result<f64> {
        let vol_curve = context.get_vol_index(&self.vol_index_curve_id)?;
        let t = self
            .day_count
            .year_fraction(as_of, self.expiry, DayCountCtx::default())?
            .max(0.0);
        Ok(vol_curve.forward_level(t))
    }

    /// Calculate delta (sensitivity to forward vol level).
    ///
    /// # Arguments
    /// * `context` - Market context with curves and surfaces
    /// * `as_of` - Valuation date for time to expiry calculation
    ///
    /// # Returns
    /// Delta per contract (change in option value per 1-point change in forward)
    pub fn delta(&self, context: &MarketContext, as_of: Date) -> finstack_core::Result<f64> {
        let vol_curve = context.get_vol_index(&self.vol_index_curve_id)?;
        let vol_surface = context.surface(&self.vol_of_vol_surface_id)?;
        let disc = context.get_discount(&self.discount_curve_id)?;

        let t = self
            .day_count
            .year_fraction(as_of, self.expiry, DayCountCtx::default())?
            .max(0.0);

        if t <= 0.0 {
            // Expired option: delta is 1 if ITM, 0 if OTM
            let forward = vol_curve.spot_level();
            let itm = match self.option_type {
                OptionType::Call => forward > self.strike,
                OptionType::Put => forward < self.strike,
            };
            return Ok(if itm { 1.0 } else { 0.0 });
        }

        let forward = vol_curve.forward_level(t);
        let sigma = vol_surface.value_clamped(t, self.strike);
        let df = disc.df_between_dates(as_of, self.expiry)?;

        let d1 = d1_black76(forward, self.strike, sigma, t);

        let delta_per_point = match self.option_type {
            OptionType::Call => norm_cdf(d1),
            OptionType::Put => norm_cdf(d1) - 1.0,
        };

        // Scale by contract multiplier, contracts, and DF
        let contracts = self.num_contracts();
        Ok(delta_per_point * self.contract_specs.multiplier * contracts * df)
    }

    /// Calculate gamma (second derivative w.r.t. forward vol level).
    ///
    /// # Arguments
    /// * `context` - Market context with curves and surfaces
    /// * `as_of` - Valuation date for time to expiry calculation
    pub fn gamma(&self, context: &MarketContext, as_of: Date) -> finstack_core::Result<f64> {
        let vol_curve = context.get_vol_index(&self.vol_index_curve_id)?;
        let vol_surface = context.surface(&self.vol_of_vol_surface_id)?;
        let disc = context.get_discount(&self.discount_curve_id)?;

        let t = self
            .day_count
            .year_fraction(as_of, self.expiry, DayCountCtx::default())?
            .max(0.0);

        if t <= 0.0 {
            return Ok(0.0);
        }

        let forward = vol_curve.forward_level(t);
        let sigma = vol_surface.value_clamped(t, self.strike);
        let df = disc.df_between_dates(as_of, self.expiry)?;

        let d1 = d1_black76(forward, self.strike, sigma, t);
        let n_prime_d1 = (-0.5 * d1 * d1).exp() / (2.0 * std::f64::consts::PI).sqrt();

        let gamma_per_point = n_prime_d1 / (forward * sigma * t.sqrt());

        let contracts = self.num_contracts();
        Ok(gamma_per_point * self.contract_specs.multiplier * contracts * df)
    }

    /// Calculate vega (sensitivity to vol-of-vol).
    ///
    /// # Arguments
    /// * `context` - Market context with curves and surfaces
    /// * `as_of` - Valuation date for time to expiry calculation
    ///
    /// # Returns
    /// Change in option value for a 1% change in vol-of-vol
    pub fn vega(&self, context: &MarketContext, as_of: Date) -> finstack_core::Result<f64> {
        let vol_curve = context.get_vol_index(&self.vol_index_curve_id)?;
        let vol_surface = context.surface(&self.vol_of_vol_surface_id)?;
        let disc = context.get_discount(&self.discount_curve_id)?;

        let t = self
            .day_count
            .year_fraction(as_of, self.expiry, DayCountCtx::default())?
            .max(0.0);

        if t <= 0.0 {
            return Ok(0.0);
        }

        let forward = vol_curve.forward_level(t);
        let sigma = vol_surface.value_clamped(t, self.strike);
        let df = disc.df_between_dates(as_of, self.expiry)?;

        let d1 = d1_black76(forward, self.strike, sigma, t);
        let n_prime_d1 = (-0.5 * d1 * d1).exp() / (2.0 * std::f64::consts::PI).sqrt();

        // Vega per point = F × N'(d1) × √T
        let vega_per_point = forward * n_prime_d1 * t.sqrt();

        // Scale by contract multiplier, contracts, and DF
        // Vega is typically quoted per 1% vol change, so multiply by 0.01
        let contracts = self.num_contracts();
        Ok(vega_per_point * self.contract_specs.multiplier * contracts * df * 0.01)
    }

    /// Calculate theta (time decay, per day).
    ///
    /// # Arguments
    /// * `context` - Market context with curves and surfaces
    /// * `as_of` - Valuation date for time to expiry calculation
    ///
    /// # Returns
    /// Change in option value for 1 day passing
    pub fn theta(&self, context: &MarketContext, as_of: Date) -> finstack_core::Result<f64> {
        let vol_curve = context.get_vol_index(&self.vol_index_curve_id)?;
        let vol_surface = context.surface(&self.vol_of_vol_surface_id)?;
        let disc = context.get_discount(&self.discount_curve_id)?;

        let t = self
            .day_count
            .year_fraction(as_of, self.expiry, DayCountCtx::default())?
            .max(0.0);

        if t <= 0.0 {
            return Ok(0.0);
        }

        let forward = vol_curve.forward_level(t);
        let sigma = vol_surface.value_clamped(t, self.strike);
        let df = disc.df_between_dates(as_of, self.expiry)?;
        let r = -df.ln() / t; // Implied rate

        let d1 = d1_black76(forward, self.strike, sigma, t);
        let n_prime_d1 = (-0.5 * d1 * d1).exp() / (2.0 * std::f64::consts::PI).sqrt();

        // Theta = -F × N'(d1) × σ / (2√T) - r × Option_Price
        let time_decay = -forward * n_prime_d1 * sigma / (2.0 * t.sqrt());
        let option_price = self.black_price(forward, sigma, t);
        let carry_cost = -r * option_price;

        let theta_per_year = time_decay + carry_cost;
        let theta_per_day = theta_per_year / 365.0;

        let contracts = self.num_contracts();
        Ok(theta_per_day * self.contract_specs.multiplier * contracts * df)
    }

    /// Calculate intrinsic value.
    ///
    /// # Arguments
    /// * `context` - Market context with vol index curves
    /// * `as_of` - Valuation date for time to expiry calculation
    pub fn intrinsic_value(
        &self,
        context: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<f64> {
        let forward = self.forward_vol(context, as_of)?;
        let intrinsic = match self.option_type {
            OptionType::Call => (forward - self.strike).max(0.0),
            OptionType::Put => (self.strike - forward).max(0.0),
        };
        Ok(intrinsic * self.contract_specs.multiplier * self.num_contracts())
    }

    /// Calculate time value (option value minus intrinsic value).
    ///
    /// # Arguments
    /// * `context` - Market context with curves and surfaces
    /// * `as_of` - Valuation date for time to expiry calculation
    pub fn time_value(&self, context: &MarketContext, as_of: Date) -> finstack_core::Result<f64> {
        let npv = self.npv_raw(context, as_of)?;
        let intrinsic = self.intrinsic_value(context, as_of)?;
        Ok(npv - intrinsic)
    }
}

// ================================================================================================
// Option risk metric providers (metrics adapters)
// ================================================================================================

impl crate::instruments::common_impl::traits::OptionDeltaProvider for VolatilityIndexOption {
    fn option_delta(&self, market: &MarketContext, as_of: Date) -> finstack_core::Result<f64> {
        self.delta(market, as_of)
    }
}

impl crate::instruments::common_impl::traits::OptionVegaProvider for VolatilityIndexOption {
    fn option_vega(&self, market: &MarketContext, as_of: Date) -> finstack_core::Result<f64> {
        self.vega(market, as_of)
    }
}

// =============================================================================
// Trait Implementations
// =============================================================================

impl crate::instruments::common_impl::traits::Instrument for VolatilityIndexOption {
    impl_instrument_base!(crate::pricer::InstrumentType::VolatilityIndexOption);

    fn value(&self, curves: &MarketContext, as_of: Date) -> finstack_core::Result<Money> {
        let pv = self.npv_raw(curves, as_of)?;
        Ok(finstack_core::money::Money::new(
            pv,
            self.notional.currency(),
        ))
    }

    fn value_raw(&self, curves: &MarketContext, as_of: Date) -> finstack_core::Result<f64> {
        self.npv_raw(curves, as_of)
    }

    fn effective_start_date(&self) -> Option<Date> {
        None
    }

    fn as_cashflow_provider(&self) -> Option<&dyn CashflowProvider> {
        Some(self)
    }

    fn scenario_overrides_mut(
        &mut self,
    ) -> Option<&mut crate::instruments::pricing_overrides::PricingOverrides> {
        Some(&mut self.pricing_overrides)
    }

    fn scenario_overrides(
        &self,
    ) -> Option<&crate::instruments::pricing_overrides::PricingOverrides> {
        Some(&self.pricing_overrides)
    }
}

impl CashflowProvider for VolatilityIndexOption {
    fn notional(&self) -> Option<Money> {
        Some(self.notional)
    }

    fn build_full_schedule(
        &self,
        _curves: &MarketContext,
        _as_of: Date,
    ) -> finstack_core::Result<crate::cashflow::builder::CashFlowSchedule> {
        Ok(crate::cashflow::traits::schedule_from_dated_flows(
            Vec::new(),
            self.notional(),
            self.day_count,
        ))
    }
}

impl crate::instruments::common_impl::traits::CurveDependencies for VolatilityIndexOption {
    fn curve_dependencies(
        &self,
    ) -> finstack_core::Result<crate::instruments::common_impl::traits::InstrumentCurves> {
        crate::instruments::common_impl::traits::InstrumentCurves::builder()
            .discount(self.discount_curve_id.clone())
            .build()
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::instruments::common_impl::traits::Instrument;
    use finstack_core::market_data::surfaces::VolSurface;
    use finstack_core::market_data::term_structures::DiscountCurve;
    use finstack_core::market_data::term_structures::VolatilityIndexCurve;
    use time::Month;

    fn setup_market() -> (MarketContext, Date) {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");

        // Create discount curve (4% flat rate)
        let disc = DiscountCurve::builder("USD-OIS")
            .base_date(base_date)
            .knots([(0.0, 1.0), (1.0, 0.96)])
            .build()
            .expect("valid discount curve");

        // Create VIX forward curve - contango structure
        let vix = VolatilityIndexCurve::builder("VIX")
            .base_date(base_date)
            .spot_level(18.0)
            .knots([(0.0, 18.0), (0.25, 20.0), (0.5, 21.0), (1.0, 22.0)])
            .build()
            .expect("valid VIX curve");

        // Create vol-of-vol surface (flat 80% vol-of-vol)
        let volvol = VolSurface::builder("VIX-VOLVOL")
            .expiries(&[0.25, 0.5, 1.0])
            .strikes(&[15.0, 20.0, 25.0])
            .row(&[0.8, 0.8, 0.8])
            .row(&[0.8, 0.8, 0.8])
            .row(&[0.8, 0.8, 0.8])
            .build()
            .expect("valid vol-of-vol surface");

        let ctx = MarketContext::new()
            .insert_discount(disc)
            .insert_vol_index(vix)
            .insert_surface(volvol);

        (ctx, base_date)
    }

    #[test]
    fn test_atm_call_has_positive_value() {
        let (market, as_of) = setup_market();

        // Create ATM call (strike = forward ~20)
        let option = VolatilityIndexOption::builder()
            .id(InstrumentId::new("VIX-CALL-ATM"))
            .notional(Money::new(2_000.0, Currency::USD)) // 1 contract
            .strike(20.0)
            .option_type(OptionType::Call)
            .expiry(Date::from_calendar_date(2025, Month::April, 1).expect("valid date"))
            .day_count(DayCount::Act365F)
            .discount_curve_id(CurveId::new("USD-OIS"))
            .vol_index_curve_id(CurveId::new("VIX"))
            .vol_of_vol_surface_id(CurveId::new("VIX-VOLVOL"))
            .build()
            .expect("valid option");

        let npv = option.value(&market, as_of).expect("value calculation");
        assert!(npv.amount() > 0.0, "ATM call should have positive value");
    }

    #[test]
    fn test_put_call_parity() {
        let (market, as_of) = setup_market();
        let expiry = Date::from_calendar_date(2025, Month::April, 1).expect("valid date");
        let strike = 20.0;

        let call = VolatilityIndexOption::builder()
            .id(InstrumentId::new("VIX-CALL"))
            .notional(Money::new(2_000.0, Currency::USD))
            .strike(strike)
            .option_type(OptionType::Call)
            .expiry(expiry)
            .day_count(DayCount::Act365F)
            .discount_curve_id(CurveId::new("USD-OIS"))
            .vol_index_curve_id(CurveId::new("VIX"))
            .vol_of_vol_surface_id(CurveId::new("VIX-VOLVOL"))
            .build()
            .expect("valid call");

        let put = VolatilityIndexOption::builder()
            .id(InstrumentId::new("VIX-PUT"))
            .notional(Money::new(2_000.0, Currency::USD))
            .strike(strike)
            .option_type(OptionType::Put)
            .expiry(expiry)
            .day_count(DayCount::Act365F)
            .discount_curve_id(CurveId::new("USD-OIS"))
            .vol_index_curve_id(CurveId::new("VIX"))
            .vol_of_vol_surface_id(CurveId::new("VIX-VOLVOL"))
            .build()
            .expect("valid put");

        let call_pv = call.npv_raw(&market, as_of).expect("call npv");
        let put_pv = put.npv_raw(&market, as_of).expect("put npv");
        let forward = call.forward_vol(&market, as_of).expect("forward vol");

        // Get DF for put-call parity
        let disc = market.get_discount("USD-OIS").expect("discount curve");
        let df = disc
            .df_between_dates(as_of, expiry)
            .expect("discount factor");

        let contracts = call.num_contracts();
        let mult = call.contract_specs.multiplier;

        // Put-Call Parity: C - P = (F - K) × DF × mult × contracts
        let parity_diff = (forward - strike) * df * mult * contracts;
        let actual_diff = call_pv - put_pv;

        assert!(
            (actual_diff - parity_diff).abs() < 1.0,
            "Put-call parity violated: expected diff {}, got {}",
            parity_diff,
            actual_diff
        );
    }

    #[test]
    fn test_deep_itm_call_approximates_intrinsic() {
        let (market, as_of) = setup_market();

        // Deep ITM call (strike << forward)
        let option = VolatilityIndexOption::builder()
            .id(InstrumentId::new("VIX-CALL-DITM"))
            .notional(Money::new(1_000.0, Currency::USD))
            .strike(10.0) // Way below forward of ~20
            .option_type(OptionType::Call)
            .expiry(Date::from_calendar_date(2025, Month::April, 1).expect("valid date"))
            .day_count(DayCount::Act365F)
            .discount_curve_id(CurveId::new("USD-OIS"))
            .vol_index_curve_id(CurveId::new("VIX"))
            .vol_of_vol_surface_id(CurveId::new("VIX-VOLVOL"))
            .build()
            .expect("valid option");

        let npv = option.npv_raw(&market, as_of).expect("npv");
        let intrinsic = option.intrinsic_value(&market, as_of).expect("intrinsic");

        // Deep ITM option should be close to intrinsic
        assert!(
            (npv - intrinsic).abs() / intrinsic < 0.2,
            "Deep ITM call should be close to intrinsic: NPV={}, intrinsic={}",
            npv,
            intrinsic
        );
    }

    #[test]
    fn test_delta_call_positive() {
        let (market, as_of) = setup_market();

        let call = VolatilityIndexOption::builder()
            .id(InstrumentId::new("VIX-CALL"))
            .notional(Money::new(2_000.0, Currency::USD))
            .strike(20.0)
            .option_type(OptionType::Call)
            .expiry(Date::from_calendar_date(2025, Month::April, 1).expect("valid date"))
            .day_count(DayCount::Act365F)
            .discount_curve_id(CurveId::new("USD-OIS"))
            .vol_index_curve_id(CurveId::new("VIX"))
            .vol_of_vol_surface_id(CurveId::new("VIX-VOLVOL"))
            .build()
            .expect("valid call");

        let delta = call.delta(&market, as_of).expect("delta");
        assert!(delta > 0.0, "Call delta should be positive");
    }

    #[test]
    fn test_delta_put_negative() {
        let (market, as_of) = setup_market();

        let put = VolatilityIndexOption::builder()
            .id(InstrumentId::new("VIX-PUT"))
            .notional(Money::new(2_000.0, Currency::USD))
            .strike(20.0)
            .option_type(OptionType::Put)
            .expiry(Date::from_calendar_date(2025, Month::April, 1).expect("valid date"))
            .day_count(DayCount::Act365F)
            .discount_curve_id(CurveId::new("USD-OIS"))
            .vol_index_curve_id(CurveId::new("VIX"))
            .vol_of_vol_surface_id(CurveId::new("VIX-VOLVOL"))
            .build()
            .expect("valid put");

        let delta = put.delta(&market, as_of).expect("delta");
        assert!(delta < 0.0, "Put delta should be negative");
    }

    #[test]
    fn test_vega_positive() {
        let (market, as_of) = setup_market();

        let option = VolatilityIndexOption::builder()
            .id(InstrumentId::new("VIX-CALL"))
            .notional(Money::new(2_000.0, Currency::USD))
            .strike(20.0)
            .option_type(OptionType::Call)
            .expiry(Date::from_calendar_date(2025, Month::April, 1).expect("valid date"))
            .day_count(DayCount::Act365F)
            .discount_curve_id(CurveId::new("USD-OIS"))
            .vol_index_curve_id(CurveId::new("VIX"))
            .vol_of_vol_surface_id(CurveId::new("VIX-VOLVOL"))
            .build()
            .expect("valid option");

        let vega = option.vega(&market, as_of).expect("vega");
        assert!(
            vega > 0.0,
            "Vega should be positive for both calls and puts"
        );
    }

    #[test]
    fn test_serde_round_trip() {
        let option = VolatilityIndexOption::example();
        let json = serde_json::to_string(&option).expect("json serialization");
        let recovered: VolatilityIndexOption =
            serde_json::from_str(&json).expect("json deserialization");
        assert_eq!(option.id, recovered.id);
        assert!((option.strike - recovered.strike).abs() < 1e-10);
    }
}
