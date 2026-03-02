//! Lookback option pricers (Monte Carlo and analytical).

// Common imports for all pricers
use crate::instruments::common_impl::traits::Instrument;
use crate::instruments::exotics::lookback_option::types::{LookbackOption, LookbackType};
use crate::pricer::{
    InstrumentType, ModelKey, Pricer, PricerKey, PricingError, PricingErrorContext, PricingResult,
};
use crate::results::ValuationResult;
use finstack_core::dates::{Date, DayCountCtx};
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;

// MC-specific imports
#[cfg(feature = "mc")]
use crate::instruments::common_impl::models::monte_carlo::payoff::lookback::{
    FloatingStrikeLookbackCall, FloatingStrikeLookbackPut, Lookback, LookbackDirection,
};
#[cfg(feature = "mc")]
use crate::instruments::common_impl::models::monte_carlo::pricer::path_dependent::{
    PathDependentPricer, PathDependentPricerConfig,
};
#[cfg(feature = "mc")]
use crate::instruments::common_impl::models::monte_carlo::process::gbm::{GbmParams, GbmProcess};

/// Lookback option Monte Carlo pricer.
#[cfg(feature = "mc")]
pub struct LookbackOptionMcPricer {
    config: PathDependentPricerConfig,
}

#[cfg(feature = "mc")]
impl LookbackOptionMcPricer {
    /// Create a new lookback option MC pricer with default config.
    pub fn new() -> Self {
        Self {
            config: PathDependentPricerConfig::default(),
        }
    }

    /// Price a lookback option using Monte Carlo.
    fn price_internal(
        &self,
        inst: &LookbackOption,
        curves: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        let t = inst
            .day_count
            .year_fraction(as_of, inst.expiry, DayCountCtx::default())?;
        if t <= 0.0 {
            return Ok(finstack_core::money::Money::new(
                0.0,
                inst.notional.currency(),
            ));
        }

        let disc_curve = curves.get_discount(inst.discount_curve_id.as_str())?;
        let discount_factor = disc_curve.df_between_dates(as_of, inst.expiry)?;
        // Keep drift consistent with date-based discounting for MC simulation.
        let r = if t > 0.0 && discount_factor > 0.0 {
            -discount_factor.ln() / t
        } else {
            0.0
        };

        let spot_scalar = curves.price(&inst.spot_id)?;
        let spot = match spot_scalar {
            finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
            finstack_core::market_data::scalars::MarketScalar::Price(m) => m.amount(),
        };

        let q = if let Some(div_id) = &inst.div_yield_id {
            match curves.price(div_id.as_str()) {
                Ok(ms) => match ms {
                    finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
                    finstack_core::market_data::scalars::MarketScalar::Price(_) => 0.0,
                },
                Err(_) => 0.0,
            }
        } else {
            0.0
        };

        let vol_surface = curves.surface(inst.vol_surface_id.as_str())?;
        let strike_val = inst.strike.unwrap_or(spot);
        let sigma = vol_surface.value_clamped(t, strike_val);

        let gbm_params = GbmParams::new(r, q, sigma);
        let process = GbmProcess::new(gbm_params);

        let steps_per_year = self.config.steps_per_year;
        let num_steps = ((t * steps_per_year).round() as usize).max(self.config.min_steps);
        let maturity_step = num_steps - 1;

        let currency = inst.notional.currency();

        // Derive deterministic seed from instrument ID and scenario
        #[cfg(feature = "mc")]
        use crate::instruments::common_impl::models::monte_carlo::seed;

        let seed = if let Some(ref scenario) = inst.pricing_overrides.scenario.mc_seed_scenario {
            #[cfg(feature = "mc")]
            {
                seed::derive_seed(&inst.id, scenario)
            }
            #[cfg(not(feature = "mc"))]
            42
        } else {
            #[cfg(feature = "mc")]
            {
                seed::derive_seed(&inst.id, "base")
            }
            #[cfg(not(feature = "mc"))]
            self.config.seed
        };

        let mut config = self.config.clone();
        config.seed = seed;
        let pricer = PathDependentPricer::new(config);
        let result = match (inst.lookback_type, inst.option_type) {
            (LookbackType::FloatingStrike, crate::instruments::OptionType::Call) => {
                // Floating Strike Call: Payoff = S_T - S_min
                // Seed initial minimum from observed_min if seasoned
                let initial_min = inst
                    .observed_min
                    .as_ref()
                    .map(|m| m.amount())
                    .unwrap_or(f64::INFINITY);
                let payoff = FloatingStrikeLookbackCall::with_initial_min(
                    inst.notional.amount(),
                    maturity_step,
                    initial_min,
                );
                pricer.price(
                    &process,
                    spot,
                    t,
                    num_steps,
                    &payoff,
                    currency,
                    discount_factor,
                )?
            }
            (LookbackType::FloatingStrike, crate::instruments::OptionType::Put) => {
                // Floating Strike Put: Payoff = S_max - S_T
                // Seed initial maximum from observed_max if seasoned
                let initial_max = inst
                    .observed_max
                    .as_ref()
                    .map(|m| m.amount())
                    .unwrap_or(f64::NEG_INFINITY);
                let payoff = FloatingStrikeLookbackPut::with_initial_max(
                    inst.notional.amount(),
                    maturity_step,
                    initial_max,
                );
                pricer.price(
                    &process,
                    spot,
                    t,
                    num_steps,
                    &payoff,
                    currency,
                    discount_factor,
                )?
            }
            (LookbackType::FixedStrike, crate::instruments::OptionType::Call) => {
                // Fixed Strike Call: Payoff = max(S_max - K, 0)
                // Seed initial maximum from observed_max if seasoned
                let initial_max = inst
                    .observed_max
                    .as_ref()
                    .map(|m| m.amount())
                    .unwrap_or(f64::NEG_INFINITY);
                let strike = inst.strike.as_ref().ok_or_else(|| {
                    finstack_core::Error::Validation(
                        "FixedStrike lookback requires a strike".into(),
                    )
                })?;
                let payoff = Lookback::with_initial_extremum(
                    LookbackDirection::Call,
                    *strike,
                    inst.notional.amount(),
                    maturity_step,
                    initial_max,
                );
                pricer.price(
                    &process,
                    spot,
                    t,
                    num_steps,
                    &payoff,
                    currency,
                    discount_factor,
                )?
            }
            (LookbackType::FixedStrike, crate::instruments::OptionType::Put) => {
                // Fixed Strike Put: Payoff = max(K - S_min, 0)
                // Seed initial minimum from observed_min if seasoned
                let initial_min = inst
                    .observed_min
                    .as_ref()
                    .map(|m| m.amount())
                    .unwrap_or(f64::INFINITY);
                let strike = inst.strike.as_ref().ok_or_else(|| {
                    finstack_core::Error::Validation(
                        "FixedStrike lookback requires a strike".into(),
                    )
                })?;
                let payoff = Lookback::with_initial_extremum(
                    LookbackDirection::Put,
                    *strike,
                    inst.notional.amount(),
                    maturity_step,
                    initial_min,
                );
                pricer.price(
                    &process,
                    spot,
                    t,
                    num_steps,
                    &payoff,
                    currency,
                    discount_factor,
                )?
            }
        };

        Ok(result.mean)
    }
}

#[cfg(feature = "mc")]
impl Default for LookbackOptionMcPricer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "mc")]
impl Pricer for LookbackOptionMcPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::LookbackOption, ModelKey::MonteCarloGBM)
    }

    fn price_dyn(
        &self,
        instrument: &dyn crate::instruments::common_impl::traits::Instrument,
        market: &MarketContext,
        as_of: Date,
    ) -> PricingResult<ValuationResult> {
        let lookback = instrument
            .as_any()
            .downcast_ref::<LookbackOption>()
            .ok_or_else(|| {
                PricingError::type_mismatch(InstrumentType::LookbackOption, instrument.key())
            })?;

        let pv = self.price_internal(lookback, market, as_of).map_err(|e| {
            PricingError::model_failure_with_context(e.to_string(), PricingErrorContext::default())
        })?;

        Ok(ValuationResult::stamped(lookback.id(), as_of, pv))
    }
}

/// Present value using Monte Carlo.
#[cfg(feature = "mc")]
pub(crate) fn compute_pv(
    inst: &LookbackOption,
    curves: &MarketContext,
    as_of: Date,
) -> finstack_core::Result<Money> {
    let pricer = LookbackOptionMcPricer::new();
    pricer.price_internal(inst, curves, as_of)
}

// ========================= ANALYTICAL PRICER =========================

use crate::instruments::common_impl::models::closed_form::lookback::{
    fixed_strike_lookback_call, fixed_strike_lookback_put, floating_strike_lookback_call,
    floating_strike_lookback_put,
};

/// Helper to collect inputs for lookback option pricing.
fn collect_lookback_inputs(
    inst: &LookbackOption,
    curves: &MarketContext,
    as_of: Date,
) -> finstack_core::Result<(f64, f64, f64, f64, f64)> {
    let t = inst
        .day_count
        .year_fraction(as_of, inst.expiry, DayCountCtx::default())?;

    let disc_curve = curves.get_discount(inst.discount_curve_id.as_str())?;
    let df = disc_curve.df_between_dates(as_of, inst.expiry)?;
    // Keep analytical rate consistent with date-based discounting used by the curve.
    let r = if t > 0.0 && df > 0.0 {
        -df.ln() / t
    } else {
        0.0
    };

    let spot_scalar = curves.price(&inst.spot_id)?;
    let spot = match spot_scalar {
        finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
        finstack_core::market_data::scalars::MarketScalar::Price(m) => m.amount(),
    };

    let q = if let Some(div_id) = &inst.div_yield_id {
        match curves.price(div_id.as_str()) {
            Ok(ms) => match ms {
                finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
                finstack_core::market_data::scalars::MarketScalar::Price(_) => 0.0,
            },
            Err(_) => 0.0,
        }
    } else {
        0.0
    };

    let vol_surface = curves.surface(inst.vol_surface_id.as_str())?;
    let strike_val = inst.strike.unwrap_or(spot);
    let sigma = vol_surface.value_clamped(t, strike_val);

    Ok((spot, r, q, sigma, t))
}

/// Lookback option analytical pricer (continuous monitoring).
pub struct LookbackOptionAnalyticalPricer;

impl LookbackOptionAnalyticalPricer {
    /// Create a new analytical lookback option pricer
    pub fn new() -> Self {
        Self
    }
}

impl Default for LookbackOptionAnalyticalPricer {
    fn default() -> Self {
        Self::new()
    }
}

impl Pricer for LookbackOptionAnalyticalPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(
            InstrumentType::LookbackOption,
            ModelKey::LookbackBSContinuous,
        )
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
        as_of: Date,
    ) -> PricingResult<ValuationResult> {
        let lookback = instrument
            .as_any()
            .downcast_ref::<LookbackOption>()
            .ok_or_else(|| {
                PricingError::type_mismatch(InstrumentType::LookbackOption, instrument.key())
            })?;

        let (spot, r, q, sigma, t) =
            collect_lookback_inputs(lookback, market, as_of).map_err(|e| {
                PricingError::model_failure_with_context(
                    e.to_string(),
                    PricingErrorContext::default(),
                )
            })?;

        if t <= 0.0 {
            return Ok(ValuationResult::stamped(
                lookback.id(),
                as_of,
                Money::new(0.0, lookback.notional.currency()),
            ));
        }

        // Determine current extremum based on option type
        let spot_extremum = match lookback.lookback_type {
            LookbackType::FixedStrike => match lookback.option_type {
                crate::instruments::OptionType::Call => {
                    // Fixed Strike Call: Payoff = max(S_max - K, 0)
                    // Need max(observed_max, current_spot)
                    let obs_max = lookback
                        .observed_max
                        .as_ref()
                        .map(|m| m.amount())
                        .unwrap_or(spot);
                    obs_max.max(spot)
                }
                crate::instruments::OptionType::Put => {
                    // Fixed Strike Put: Payoff = max(K - S_min, 0)
                    // Need min(observed_min, current_spot)
                    let obs_min = lookback
                        .observed_min
                        .as_ref()
                        .map(|m| m.amount())
                        .unwrap_or(spot);
                    obs_min.min(spot)
                }
            },
            LookbackType::FloatingStrike => match lookback.option_type {
                crate::instruments::OptionType::Call => {
                    // Floating Strike Call: Payoff = S_T - S_min
                    // Need min(observed_min, current_spot)
                    let obs_min = lookback
                        .observed_min
                        .as_ref()
                        .map(|m| m.amount())
                        .unwrap_or(spot);
                    obs_min.min(spot)
                }
                crate::instruments::OptionType::Put => {
                    // Floating Strike Put: Payoff = S_max - S_T
                    // Need max(observed_max, current_spot)
                    let obs_max = lookback
                        .observed_max
                        .as_ref()
                        .map(|m| m.amount())
                        .unwrap_or(spot);
                    obs_max.max(spot)
                }
            },
        };

        let price = match lookback.lookback_type {
            LookbackType::FixedStrike => {
                let strike = lookback.strike.as_ref().ok_or_else(|| {
                    PricingError::model_failure_with_context(
                        "FixedStrike lookback requires a strike",
                        PricingErrorContext::default(),
                    )
                })?;
                match lookback.option_type {
                    crate::instruments::OptionType::Call => {
                        fixed_strike_lookback_call(spot, *strike, t, r, q, sigma, spot_extremum)
                    }
                    crate::instruments::OptionType::Put => {
                        fixed_strike_lookback_put(spot, *strike, t, r, q, sigma, spot_extremum)
                    }
                }
            }
            LookbackType::FloatingStrike => match lookback.option_type {
                crate::instruments::OptionType::Call => {
                    floating_strike_lookback_call(spot, t, r, q, sigma, spot_extremum)
                }
                crate::instruments::OptionType::Put => {
                    floating_strike_lookback_put(spot, t, r, q, sigma, spot_extremum)
                }
            },
        };

        let currency = lookback.notional.currency();
        let pv = Money::new(price * lookback.notional.amount(), currency);
        Ok(ValuationResult::stamped(lookback.id(), as_of, pv))
    }
}
