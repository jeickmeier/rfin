//! Range accrual Monte Carlo and Analytical pricers.

#[cfg(feature = "mc")]
use crate::instruments::common::mc::process::gbm::{GbmParams, GbmProcess};
#[cfg(feature = "mc")]
use crate::instruments::common::models::monte_carlo::payoff::range_accrual::RangeAccrualPayoff;
#[cfg(feature = "mc")]
use crate::instruments::common::models::monte_carlo::pricer::path_dependent::{
    PathDependentPricer, PathDependentPricerConfig,
};
#[cfg(feature = "mc")]
use crate::instruments::common::traits::Instrument;
#[cfg(feature = "mc")]
use crate::instruments::range_accrual::types::RangeAccrual;
#[cfg(feature = "mc")]
use crate::pricer::{InstrumentType, ModelKey, Pricer, PricerKey, PricingError, PricingResult};
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
        let spot_scalar = curves.price(&inst.spot_id)?;
        let initial_spot = match spot_scalar {
            finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
            finstack_core::market_data::scalars::MarketScalar::Price(m) => m.amount(),
        };

        let final_date = inst
            .payment_date
            .unwrap_or(inst.observation_dates.last().copied().unwrap_or(as_of));
        let t = inst
            .day_count
            .year_fraction(as_of, final_date, DayCountCtx::default())?;
        if t <= 0.0 {
            return Ok(Money::new(0.0, inst.notional.currency()));
        }

        let disc_curve = curves.get_discount_ref(inst.discount_curve_id.as_str())?;
        let r = disc_curve.zero(t);
        let discount_factor = disc_curve
            .try_df_between_dates(as_of, final_date)
            .unwrap_or(1.0);

        let mut q = if let Some(div_id) = &inst.div_yield_id {
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

        let vol_surface = curves.surface_ref(inst.vol_surface_id.as_str())?;
        let sigma = vol_surface.value_clamped(t, initial_spot); // value_clamped handles bounds

        // Quanto Adjustment
        if let Some(rho) = inst.quanto_correlation {
            if let Some(ref fx_vol_id) = inst.fx_vol_surface_id {
                let fx_vol_surface = curves.surface_ref(fx_vol_id.as_str())?;
                // Assume FX vol at strike 1.0 (or spot) roughly.
                // If we knew FX spot, we'd use it. Without it, 1.0 is a common proxy for normalized FX surfaces or ATM.
                let sigma_fx = fx_vol_surface.value_clamped(t, 1.0);

                // Drift adjustment: r_d - r_f - q - rho * sigma_S * sigma_FX
                // The 'q' parameter in GbmParams is subtracted from r.
                // Drift = r - q_param.
                // Desired Drift = r - q_real - rho * sigma_S * sigma_FX
                // => r - q_param = r - q_real - rho * sigma_S * sigma_FX
                // => q_param = q_real + rho * sigma_S * sigma_FX

                q += rho * sigma * sigma_fx;
            }
        }

        let gbm_params = GbmParams::new(r, q, sigma);
        let process = GbmProcess::new(gbm_params);

        let steps_per_year = self.config.steps_per_year;
        let num_steps = ((t * steps_per_year).round() as usize).max(self.config.min_steps);

        // Map observation dates to times
        let observation_times: Vec<f64> = inst
            .observation_dates
            .iter()
            .map(|&date| {
                inst.day_count
                    .year_fraction(as_of, date, DayCountCtx::default())
                    .unwrap_or(0.0)
            })
            .collect();

        let payoff = RangeAccrualPayoff::new(
            observation_times,
            inst.lower_bound,
            inst.upper_bound,
            inst.coupon_rate,
            inst.notional.amount(),
            inst.notional.currency(),
        );

        // Derive deterministic seed from instrument ID and scenario
        #[cfg(feature = "mc")]
        use crate::instruments::common::models::monte_carlo::seed;

        let seed = if let Some(ref scenario) = inst.pricing_overrides.mc_seed_scenario {
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
        instrument: &dyn crate::instruments::common::traits::Instrument,
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
            .map_err(|e| PricingError::model_failure(e.to_string()))?;

        Ok(ValuationResult::stamped(range_accrual.id(), as_of, pv))
    }
}

/// Present value using Monte Carlo.
#[cfg(feature = "mc")]
pub fn npv(inst: &RangeAccrual, curves: &MarketContext, as_of: Date) -> Result<Money> {
    // If explicit model choice is not MC, prefer Analytic Static Replication
    // (Currently assuming Analytic is the "Standard" for simple range accruals)
    // We can add a flag in PricingOverrides if the user wants to force MC.
    // For now, we route to Analytic by default as it is more accurate for skew.

    // Check if forced MC (future feature? or infer from overrides?)
    // If 'mc_seed_scenario' is set, user likely expects MC.
    if inst.pricing_overrides.mc_seed_scenario.is_some() {
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
#[cfg(feature = "mc")]
pub fn npv_analytic(inst: &RangeAccrual, curves: &MarketContext, as_of: Date) -> Result<Money> {
    use finstack_core::math::special_functions::norm_cdf;

    let spot_scalar = curves.price(&inst.spot_id)?;
    let initial_spot = match spot_scalar {
        finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
        finstack_core::market_data::scalars::MarketScalar::Price(m) => m.amount(),
    };

    let final_date = inst
        .payment_date
        .unwrap_or(inst.observation_dates.last().copied().unwrap_or(as_of));

    let disc_curve = curves.get_discount_ref(inst.discount_curve_id.as_str())?;
    let discount_factor = disc_curve
        .try_df_between_dates(as_of, final_date)
        .unwrap_or(1.0);

    let q_yield = if let Some(div_id) = &inst.div_yield_id {
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

    // Term structure of rates: we should ideally look up rate for each observation.
    // Simplified: Use zero rate to observation date.

    let vol_surface = curves.surface_ref(inst.vol_surface_id.as_str())?;

    // Quanto Logic (adjust q_yield)
    // Note: For Static Replication, we adjust the Forward Price.
    // E[S_T] in Payment Measure = S_0 * exp((r - q - rho*sig*sig_fx)*T)
    // So we just add the drift term to q_yield effectively.
    if let Some(_rho) = inst.quanto_correlation {
        if let Some(ref fx_vol_id) = inst.fx_vol_surface_id {
            let _fx_vol_surface = curves.surface_ref(fx_vol_id.as_str())?;
            // Using ATM/1.0 vol approximation for the drift adjustment
            // Ideally this would be time-dependent, but for drift adjustment it's usually fine.
            // We'll look up at maturity or average? Let's use maturity for simplicity or look up per step.
            // We'll do it per step inside loop for better term structure support.
        }
    }

    let mut total_prob = 0.0;
    let n_obs = inst.observation_dates.len();
    if n_obs == 0 {
        return Ok(Money::new(0.0, inst.notional.currency()));
    }

    for &date in &inst.observation_dates {
        let t_obs = inst
            .day_count
            .year_fraction(as_of, date, DayCountCtx::default())?;
        if t_obs <= 0.0 {
            // Past observation. If we had history we'd check it.
            // Assuming valuation as of today implies we only care about future?
            // Or assuming past is "in range"?
            // Convention: Past fixings should be provided or we assume 1 (or 0).
            // Code usually prices "remaining value". If the user wants full value including accrued,
            // they need to handle past fixings separately.
            // For now, we assume t<=0 means known outcome, but we don't have history.
            // We'll skip (assume 0) or assume 1?
            // Let's skip contribution (0).
            continue;
        }

        let r_obs = disc_curve.zero(t_obs);

        // Quanto drift adjustment specific to this horizon
        let mut drift_adj = 0.0;
        if let Some(rho) = inst.quanto_correlation {
            if let Some(ref fx_vol_id) = inst.fx_vol_surface_id {
                let fx_vol_surface = curves.surface_ref(fx_vol_id.as_str())?;
                // Vol of Asset (S) for drift adj: use ATM
                let sig_s = vol_surface.value_clamped(t_obs, initial_spot);
                // Vol of FX for drift adj: use ATM (strike 1.0 proxy)
                let sig_fx = fx_vol_surface.value_clamped(t_obs, 1.0);
                drift_adj = rho * sig_s * sig_fx;
            }
        }

        // Forward Price F = S * exp((r - q - drift_adj) * t)
        // Note: drift_adj is subtracted from drift of S.
        // Risk neutral S drift is (r - q).
        // Payment measure S drift is (r - q - rho*sig*sig_fx).
        let forward = initial_spot * ((r_obs - q_yield - drift_adj) * t_obs).exp();

        // Digital Call Probability P(S > K) = N(d2)
        // d2 = (ln(F/K) - 0.5*sigma^2*t) / (sigma*sqrt(t))
        // We use sigma at strike K

        let calc_prob_above = |strike: f64| -> finstack_core::Result<f64> {
            let vol = vol_surface.value_clamped(t_obs, strike);
            let std_dev = vol * t_obs.sqrt();
            if std_dev < 1e-6 {
                if forward > strike {
                    Ok(1.0)
                } else {
                    Ok(0.0)
                }
            } else {
                let d2 = ((forward / strike).ln() - 0.5 * vol * vol * t_obs) / std_dev;
                Ok(norm_cdf(d2))
            }
        };

        let p_lower = calc_prob_above(inst.lower_bound)?;
        let p_upper = calc_prob_above(inst.upper_bound)?;

        // Prob in range [L, U] = P(S > L) - P(S > U)
        let p_in_range = p_lower - p_upper;

        // Clamp to [0, 1] for numerical noise
        let p_clamped = p_in_range.clamp(0.0, 1.0);

        total_prob += p_clamped;
    }

    // Average probability * Coupon * Notional * DF
    // Note: Range Accrual usually pays Coupon * (DaysRange / TotalDays).
    // So we sum probabilities (expected days) and divide by total days.
    let expected_fraction = total_prob / (n_obs as f64);
    let fv = inst.notional.amount() * inst.coupon_rate * expected_fraction;
    let pv = fv * discount_factor;

    Ok(Money::new(pv, inst.notional.currency()))
}
