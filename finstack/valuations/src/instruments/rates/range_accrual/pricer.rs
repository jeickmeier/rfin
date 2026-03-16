//! Range accrual Monte Carlo and Analytical pricers.
//!
//! This module provides two pricing methods:
//!
//! 1. **Static Replication (Default)**: Uses digital call spread replication to price
//!    the range accrual analytically. Captures volatility skew/smile naturally.
//!
//! 2. **Monte Carlo**: Path-dependent simulation for complex cases or when explicitly
//!    requested via `mc_seed_scenario` override.
//!
//! Both methods support:
//! - Absolute or relative bounds (via `BoundsType`)
//! - Quanto drift adjustment (requires `quanto_correlation` and `fx_vol_surface_id`)
//! - Historical fixings for mid-life valuations (via `past_fixings_in_range`)

#[cfg(feature = "mc")]
use crate::instruments::common_impl::traits::Instrument;
#[cfg(feature = "mc")]
use crate::instruments::rates::range_accrual::monte_carlo::RangeAccrualPayoff;
#[cfg(feature = "mc")]
use crate::instruments::rates::range_accrual::types::RangeAccrual;
#[cfg(feature = "mc")]
use crate::pricer::{
    InstrumentType, ModelKey, Pricer, PricerKey, PricingError, PricingErrorContext, PricingResult,
};
#[cfg(feature = "mc")]
use crate::results::ValuationResult;
#[cfg(feature = "mc")]
use finstack_core::dates::{Date, DayCountCtx};
#[cfg(feature = "mc")]
use finstack_core::market_data::context::MarketContext;
#[cfg(feature = "mc")]
use finstack_core::money::Money;
#[cfg(feature = "mc")]
use finstack_core::Result;
#[cfg(feature = "mc")]
use finstack_monte_carlo::pricer::path_dependent::{
    PathDependentPricer, PathDependentPricerConfig,
};
#[cfg(feature = "mc")]
use finstack_monte_carlo::process::gbm::{GbmParams, GbmProcess};

/// Helper to get FX spot for quanto vol lookup.
/// Falls back to 1.0 if fx_spot_id is not provided (ATM approximation).
#[cfg(feature = "mc")]
fn get_fx_spot(inst: &RangeAccrual, curves: &MarketContext) -> f64 {
    let fx_spot_id = inst.quanto.as_ref().and_then(|q| q.fx_spot_id.as_deref());

    if let Some(id) = fx_spot_id {
        match curves.get_price(id) {
            Ok(ms) => match ms {
                finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
                finstack_core::market_data::scalars::MarketScalar::Price(m) => m.amount(),
            },
            Err(_) => 1.0, // Fallback to ATM approximation
        }
    } else {
        1.0 // ATM approximation when no FX spot provided
    }
}

/// Range accrual Monte Carlo pricer.
#[cfg(feature = "mc")]
pub struct RangeAccrualMcPricer {
    config: PathDependentPricerConfig,
}

#[cfg(feature = "mc")]
impl RangeAccrualMcPricer {
    /// Create a new range accrual MC pricer with default config.
    pub fn new() -> Self {
        Self {
            config: PathDependentPricerConfig::default(),
        }
    }

    /// Price a range accrual using Monte Carlo.
    fn price_internal(
        &self,
        inst: &RangeAccrual,
        curves: &MarketContext,
        as_of: Date,
    ) -> Result<finstack_core::money::Money> {
        inst.validate()?;
        let spot_scalar = curves.get_price(&inst.spot_id)?;
        let initial_spot = match spot_scalar {
            finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
            finstack_core::market_data::scalars::MarketScalar::Price(m) => m.amount(),
        };

        // Compute effective bounds based on BoundsType
        let effective_lower = inst.effective_lower_bound(initial_spot);
        let effective_upper = inst.effective_upper_bound(initial_spot);

        let final_date = inst
            .payment_date
            .unwrap_or(inst.observation_dates.last().copied().unwrap_or(as_of));
        let t = inst
            .day_count
            .year_fraction(as_of, final_date, DayCountCtx::default())?;

        // Count future observations only
        let future_obs_count = inst
            .observation_dates
            .iter()
            .filter(|&&date| {
                inst.day_count
                    .year_fraction(as_of, date, DayCountCtx::default())
                    .unwrap_or(0.0)
                    > 0.0
            })
            .count();

        // If no future observations, return value based on past fixings only
        if future_obs_count == 0 || t <= 0.0 {
            return compute_past_only_value(inst);
        }

        let disc_curve = curves.get_discount(inst.discount_curve_id.as_str())?;
        let r = disc_curve.zero(t);
        let discount_factor = disc_curve.df_between_dates(as_of, final_date)?;

        let mut q = if let Some(div_id) = &inst.div_yield_id {
            match curves.get_price(div_id.as_str()) {
                Ok(ms) => match ms {
                    finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
                    finstack_core::market_data::scalars::MarketScalar::Price(_) => 0.0,
                },
                Err(_) => 0.0,
            }
        } else {
            0.0
        };

        let vol_surface = curves.get_surface(inst.vol_surface_id.as_str())?;
        let sigma = vol_surface.value_clamped(t, initial_spot);

        // Quanto Adjustment using FX spot for vol lookup
        if let Some(quanto) = &inst.quanto {
            let fx_vol_surface = curves.get_surface(quanto.fx_vol_surface_id.as_str())?;
            let fx_spot = get_fx_spot(inst, curves);
            let sigma_fx = fx_vol_surface.value_clamped(t, fx_spot);

            // Drift adjustment: q_param = q_real + rho * sigma_S * sigma_FX
            q += quanto.correlation * sigma * sigma_fx;
        }

        let gbm_params = GbmParams::new(r, q, sigma);
        let process = GbmProcess::new(gbm_params);

        let steps_per_year = self.config.steps_per_year;
        let num_steps = ((t * steps_per_year).round() as usize).max(self.config.min_steps);

        // Map only future observation dates to times (filter out past observations)
        let observation_times: Vec<f64> = inst
            .observation_dates
            .iter()
            .filter_map(|&date| {
                let t_obs = inst
                    .day_count
                    .year_fraction(as_of, date, DayCountCtx::default())
                    .unwrap_or(0.0);
                if t_obs > 0.0 {
                    Some(t_obs)
                } else {
                    None
                }
            })
            .collect();

        // Create payoff with effective bounds and historical fixing info
        let payoff = RangeAccrualPayoff::new_with_history(
            observation_times,
            effective_lower,
            effective_upper,
            inst.coupon_rate,
            inst.notional.amount(),
            inst.notional.currency(),
            inst.past_fixings_in_range.unwrap_or(0),
            inst.total_past_observations.unwrap_or(0),
        );

        // Derive deterministic seed from instrument ID and scenario
        use finstack_monte_carlo::seed;

        let seed = if let Some(ref scenario) = inst.pricing_overrides.scenario.mc_seed_scenario {
            seed::derive_seed(&inst.id, scenario)
        } else {
            seed::derive_seed(&inst.id, "base")
        };

        let mut config = self.config.clone();
        config.seed = seed;
        let pricer = PathDependentPricer::new(config);
        let result = pricer.price(
            &process,
            initial_spot,
            t,
            num_steps,
            &payoff,
            inst.notional.currency(),
            discount_factor,
        )?;

        Ok(result.mean)
    }
}

/// Compute value when only past fixings exist (no future observations).
#[cfg(feature = "mc")]
fn compute_past_only_value(inst: &RangeAccrual) -> Result<Money> {
    match (inst.past_fixings_in_range, inst.total_past_observations) {
        (Some(in_range), Some(total)) if total > 0 => {
            let accrual_fraction = in_range as f64 / total as f64;
            let fv = inst.notional.amount() * inst.coupon_rate * accrual_fraction;
            // No discounting needed - payment date is in the past or at as_of
            Ok(Money::new(fv, inst.notional.currency()))
        }
        _ => Ok(Money::new(0.0, inst.notional.currency())),
    }
}

#[cfg(feature = "mc")]
impl Default for RangeAccrualMcPricer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "mc")]
impl Pricer for RangeAccrualMcPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::RangeAccrual, ModelKey::MonteCarloGBM)
    }

    fn price_dyn(
        &self,
        instrument: &dyn crate::instruments::common_impl::traits::Instrument,
        market: &MarketContext,
        as_of: Date,
    ) -> PricingResult<ValuationResult> {
        let range_accrual = instrument
            .as_any()
            .downcast_ref::<RangeAccrual>()
            .ok_or_else(|| {
                PricingError::type_mismatch(InstrumentType::RangeAccrual, instrument.key())
            })?;

        let pv = self
            .price_internal(range_accrual, market, as_of)
            .map_err(|e| {
                PricingError::model_failure_with_context(
                    e.to_string(),
                    PricingErrorContext::default(),
                )
            })?;

        Ok(ValuationResult::stamped(range_accrual.id(), as_of, pv))
    }
}

/// Present value using Monte Carlo.
#[cfg(feature = "mc")]
pub(crate) fn compute_pv(
    inst: &RangeAccrual,
    curves: &MarketContext,
    as_of: Date,
) -> Result<Money> {
    // If explicit model choice is not MC, prefer Analytic Static Replication
    // (Currently assuming Analytic is the "Standard" for simple range accruals)
    // We can add a flag in PricingOverrides if the user wants to force MC.
    // For now, we route to Analytic by default as it is more accurate for skew.

    // Check if forced MC (future feature? or infer from overrides?)
    // If 'mc_seed_scenario' is set, user likely expects MC.
    if inst.pricing_overrides.scenario.mc_seed_scenario.is_some() {
        let pricer = RangeAccrualMcPricer::new();
        pricer.price_internal(inst, curves, as_of)
    } else {
        npv_analytic(inst, curves, as_of)
    }
}

/// Present value using Static Replication (Analytic).
///
/// Replicates the range accrual as a sum of digital options (binary call spreads).
/// Captures volatility skew/smile and term structure naturally from the surface.
///
/// This method:
/// - Uses effective bounds based on `BoundsType` (absolute or relative to initial spot)
/// - Applies quanto drift adjustment using FX spot for vol lookup when available
/// - Includes historical fixings in the accrual calculation for mid-life valuations
#[cfg(feature = "mc")]
pub fn npv_analytic(inst: &RangeAccrual, curves: &MarketContext, as_of: Date) -> Result<Money> {
    use crate::instruments::common_impl::models::volatility::black::d1_d2_black76;
    use finstack_core::math::special_functions::norm_cdf;

    inst.validate()?;
    let spot_scalar = curves.get_price(&inst.spot_id)?;
    let initial_spot = match spot_scalar {
        finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
        finstack_core::market_data::scalars::MarketScalar::Price(m) => m.amount(),
    };

    // Compute effective bounds based on BoundsType
    let effective_lower = inst.effective_lower_bound(initial_spot);
    let effective_upper = inst.effective_upper_bound(initial_spot);

    let final_date = inst
        .payment_date
        .unwrap_or(inst.observation_dates.last().copied().unwrap_or(as_of));

    let disc_curve = curves.get_discount(inst.discount_curve_id.as_str())?;
    let discount_factor = disc_curve.df_between_dates(as_of, final_date)?;

    let q_yield = if let Some(div_id) = &inst.div_yield_id {
        match curves.get_price(div_id.as_str()) {
            Ok(ms) => match ms {
                finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
                finstack_core::market_data::scalars::MarketScalar::Price(_) => 0.0,
            },
            Err(_) => 0.0,
        }
    } else {
        0.0
    };

    let vol_surface = curves.get_surface(inst.vol_surface_id.as_str())?;

    // Get FX spot for quanto vol lookup (uses actual spot if available, else 1.0)
    let fx_spot = get_fx_spot(inst, curves);

    // Count observations and track past/future split
    let n_total_obs = inst.observation_dates.len();
    if n_total_obs == 0 {
        return Ok(Money::new(0.0, inst.notional.currency()));
    }

    // Count future observations
    let mut future_obs_count = 0usize;
    let mut total_expected_in_range = 0.0;

    for &date in &inst.observation_dates {
        let t_obs = inst
            .day_count
            .year_fraction(as_of, date, DayCountCtx::default())?;

        if t_obs <= 0.0 {
            // Past observation - skip (handled via past_fixings_in_range)
            continue;
        }

        future_obs_count += 1;
        let r_obs = disc_curve.zero(t_obs);

        // Quanto drift adjustment specific to this horizon
        let mut drift_adj = 0.0;
        if let Some(quanto) = &inst.quanto {
            let fx_vol_surface = curves.get_surface(quanto.fx_vol_surface_id.as_str())?;
            // Vol of Asset (S) for drift adj: use ATM at current spot
            let sig_s = vol_surface.value_clamped(t_obs, initial_spot);
            // Vol of FX for drift adj: use ATM at FX spot
            let sig_fx = fx_vol_surface.value_clamped(t_obs, fx_spot);
            drift_adj = quanto.correlation * sig_s * sig_fx;
        }

        // Forward Price F = S * exp((r - q - drift_adj) * t)
        let forward = initial_spot * ((r_obs - q_yield - drift_adj) * t_obs).exp();

        // Digital Call Probability P(S_t > K) via finite-width call spread.
        //
        // Using a 25bp spread replaces the analytically thin N(d₂) digital with
        // a hedgeable call spread that correctly captures the volatility skew
        // contribution to the digital price.  The formula is:
        //
        //   P(S > K) ≈ [Call(K - h/2) - Call(K + h/2)] / h
        //
        // where h = DIGITAL_SPREAD_WIDTH = 0.0025 (25 basis points) and
        // Call(k) is the undiscounted Black-76 call price at strike k.
        //
        // For a flat smile this recovers N(d₂) exactly as h → 0.  With a
        // downward skew (higher vol at lower strikes), P(S > K) is larger
        // than the flat-smile N(d₂), matching market digital prices.
        //
        // Lower node is clamped to DIGITAL_SPREAD_FLOOR to ensure K - h/2 > 0.
        const DIGITAL_SPREAD_WIDTH: f64 = 0.0025; // 25 bp
        const DIGITAL_SPREAD_FLOOR: f64 = 1e-6; // prevent negative strikes

        // Undiscounted Black-76 call price: F·N(d1) - K·N(d2)
        let black_call = |k: f64| -> f64 {
            let vol = vol_surface.value_clamped(t_obs, k);
            let std_dev = vol * t_obs.sqrt();
            if std_dev < 1e-6 {
                return (forward - k).max(0.0);
            }
            let (d1, d2) = d1_d2_black76(forward, k, vol, t_obs);
            forward * norm_cdf(d1) - k * norm_cdf(d2)
        };

        // Digital call probability using finite-width call spread.
        // The spread half-width is clipped so K - h/2 ≥ DIGITAL_SPREAD_FLOOR.
        let calc_prob_above = |strike: f64| -> finstack_core::Result<f64> {
            let half_h = DIGITAL_SPREAD_WIDTH / 2.0;
            let k_lo = (strike - half_h).max(DIGITAL_SPREAD_FLOOR);
            let k_hi = strike + half_h;
            // Effective spread width (may be narrower near zero)
            let spread = k_hi - k_lo;
            if spread < 1e-12 {
                // Degenerate: fall back to a binary step on the forward
                return Ok(if forward > strike { 1.0 } else { 0.0 });
            }
            let prob = (black_call(k_lo) - black_call(k_hi)) / spread;
            Ok(prob.clamp(0.0, 1.0))
        };

        let p_lower = calc_prob_above(effective_lower)?;
        let p_upper = calc_prob_above(effective_upper)?;

        // Prob in range [L, U] = P(S > L) - P(S > U)
        let p_in_range = (p_lower - p_upper).clamp(0.0, 1.0);
        total_expected_in_range += p_in_range;
    }

    // Include historical fixings in the total
    // Total observations = past observations + future observations
    let past_in_range = inst.past_fixings_in_range.unwrap_or(0) as f64;
    let total_past_obs = inst.total_past_observations.unwrap_or(0);

    // Total observations across full life of instrument
    let total_obs_count = total_past_obs + future_obs_count;
    if total_obs_count == 0 {
        return Ok(Money::new(0.0, inst.notional.currency()));
    }

    // Expected total days in range = known past + expected future
    let expected_total_in_range = past_in_range + total_expected_in_range;

    // Accrual fraction = expected days in range / total days
    let expected_fraction = expected_total_in_range / (total_obs_count as f64);

    // Future value and present value
    let fv = inst.notional.amount() * inst.coupon_rate * expected_fraction;
    let pv = fv * discount_factor;

    Ok(Money::new(pv, inst.notional.currency()))
}
