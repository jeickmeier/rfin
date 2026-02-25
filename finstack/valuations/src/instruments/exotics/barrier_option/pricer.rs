//! Barrier option pricers (Monte Carlo and analytical).

// Common imports for all pricers
use crate::instruments::common_impl::traits::Instrument;
use crate::instruments::exotics::barrier_option::types::BarrierOption;
use crate::pricer::{
    InstrumentType, ModelKey, Pricer, PricerKey, PricingError, PricingErrorContext, PricingResult,
};
use crate::results::ValuationResult;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;

// DayCountCtx is only used by MC pricer
#[cfg(feature = "mc")]
use finstack_core::dates::DayCountCtx;

// MC-specific imports
#[cfg(feature = "mc")]
use crate::instruments::common_impl::mc::process::gbm::{GbmParams, GbmProcess};
#[cfg(feature = "mc")]
use crate::instruments::common_impl::models::monte_carlo::payoff::barrier::BarrierOptionPayoff;
#[cfg(feature = "mc")]
use crate::instruments::common_impl::models::monte_carlo::payoff::barrier::BarrierType as McBarrierType;
#[cfg(feature = "mc")]
use crate::instruments::common_impl::models::monte_carlo::pricer::path_dependent::{
    PathDependentPricer, PathDependentPricerConfig,
};

/// Barrier option Monte Carlo pricer.
#[cfg(feature = "mc")]
pub struct BarrierOptionMcPricer {
    config: PathDependentPricerConfig,
}

#[cfg(feature = "mc")]
impl BarrierOptionMcPricer {
    /// Create a new barrier option MC pricer with default config.
    pub fn new() -> Self {
        Self {
            config: PathDependentPricerConfig::default(),
        }
    }

    fn convert_barrier_type(
        bt: crate::instruments::exotics::barrier_option::types::BarrierType,
    ) -> McBarrierType {
        match bt {
            crate::instruments::exotics::barrier_option::types::BarrierType::UpAndOut => {
                McBarrierType::UpAndOut
            }
            crate::instruments::exotics::barrier_option::types::BarrierType::UpAndIn => {
                McBarrierType::UpAndIn
            }
            crate::instruments::exotics::barrier_option::types::BarrierType::DownAndOut => {
                McBarrierType::DownAndOut
            }
            crate::instruments::exotics::barrier_option::types::BarrierType::DownAndIn => {
                McBarrierType::DownAndIn
            }
        }
    }

    /// Price a barrier option using Monte Carlo.
    ///
    /// # Day Count Convention Handling
    ///
    /// Uses separate day count bases for different purposes:
    /// - **Discounting**: Uses the discount curve's own day count for DF and zero rate calculations
    /// - **Volatility lookup**: Uses the instrument's day count (assumed to match vol surface calibration)
    /// - **Monte Carlo time grid**: Uses the vol surface time basis for proper barrier monitoring
    fn price_internal(
        &self,
        inst: &BarrierOption,
        curves: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        // Get discount curve
        let disc_curve = curves.get_discount(inst.discount_curve_id.as_str())?;

        // Time to maturity for vol surface (using instrument's day count, which should
        // match how the vol surface was calibrated)
        let t_vol = inst
            .day_count
            .year_fraction(as_of, inst.expiry, DayCountCtx::default())?;

        if t_vol <= 0.0 {
            // Expired barrier option: without barrier-crossing history, we cannot
            // determine whether the barrier was hit during the option's life.
            //
            // - Knock-out: if barrier was hit → 0 (+ rebate); if not → intrinsic
            // - Knock-in: if barrier was hit → intrinsic; if not → 0 (+ rebate)
            //
            // Since barrier-crossing state is unknown, we conservatively return 0.
            // For accurate expired barrier pricing, provide barrier-crossing history
            // or price before expiry. The rebate is also not paid since we cannot
            // determine the triggering condition.
            return Ok(finstack_core::money::Money::new(
                0.0,
                inst.notional.currency(),
            ));
        }

        let discount_factor = disc_curve.df_between_dates(as_of, inst.expiry)?;
        // Keep drift consistent with date-based discounting for MC simulation.
        let r = if t_vol > 0.0 && discount_factor > 0.0 {
            -discount_factor.ln() / t_vol
        } else {
            0.0
        };

        // Get spot
        let spot_scalar = curves.price(&inst.spot_id)?;
        let spot = match spot_scalar {
            finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
            finstack_core::market_data::scalars::MarketScalar::Price(m) => m.amount(),
        };

        // Get dividend yield
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

        // Get volatility using vol surface time basis
        let vol_surface = curves.surface(inst.vol_surface_id.as_str())?;
        let sigma = vol_surface.value_clamped(t_vol, inst.strike);

        // Create GBM process
        let gbm_params = GbmParams::new(r, q, sigma);
        let process = GbmProcess::new(gbm_params);

        // Create time grid with minimum-capped steps (using vol surface time basis for proper
        // barrier monitoring - this ensures time steps align with volatility assumptions)
        let steps_per_year = self.config.steps_per_year;
        let num_steps = ((t_vol * steps_per_year).round() as usize).max(self.config.min_steps);
        let maturity_step = num_steps - 1;

        // Create payoff (using vol surface time for barrier adjustment calculations)
        let mc_barrier_type = Self::convert_barrier_type(inst.barrier_type);
        let payoff = BarrierOptionPayoff::new(
            inst.strike,
            inst.barrier.amount(),
            mc_barrier_type,
            inst.option_type,
            inst.rebate.map(|m| m.amount()),
            inst.notional.amount(),
            maturity_step,
            sigma,
            t_vol,
            inst.use_gobet_miri,
        );

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

        // Create config with derived seed
        let mut config = self.config.clone();
        config.seed = seed;

        // Price using path-dependent pricer (using vol surface time basis for simulation)
        let pricer = PathDependentPricer::new(config);
        let result = pricer.price(
            &process,
            spot,
            t_vol,
            num_steps,
            &payoff,
            inst.notional.currency(),
            discount_factor,
        )?;

        Ok(result.mean)
    }

    /// Price with LRM Greeks (delta, vega) convenience for barrier options.
    ///
    /// Returns `(pv, Option<(delta, vega)>)` where the Greeks are from the
    /// Likelihood Ratio Method (LRM). Greeks are `None` if the option is expired.
    ///
    /// # Day Count Convention Handling
    ///
    /// Uses separate day count bases for different purposes:
    /// - **Discounting**: Uses the discount curve's own day count for DF and zero rate calculations
    /// - **Volatility lookup and MC simulation**: Uses the instrument's day count (assumed to match vol surface calibration)
    #[allow(dead_code)] // May be used by external bindings or tests
    pub(crate) fn price_with_lrm_greeks_internal(
        &self,
        inst: &BarrierOption,
        curves: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<(finstack_core::money::Money, Option<(f64, f64)>)> {
        // Get discount curve first to access its day count
        let disc_curve = curves.get_discount(inst.discount_curve_id.as_str())?;

        // Time to maturity for vol surface (using instrument's day count)
        let t_vol = inst
            .day_count
            .year_fraction(as_of, inst.expiry, DayCountCtx::default())?;
        if t_vol <= 0.0 {
            return Ok((
                finstack_core::money::Money::new(0.0, inst.notional.currency()),
                None,
            ));
        }

        let discount_factor = disc_curve.df_between_dates(as_of, inst.expiry)?;
        // Keep drift consistent with date-based discounting for MC simulation.
        let r = if t_vol > 0.0 && discount_factor > 0.0 {
            -discount_factor.ln() / t_vol
        } else {
            0.0
        };

        // Spot and dividend yield
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

        // Volatility and process (using vol surface time basis)
        let vol_surface = curves.surface(inst.vol_surface_id.as_str())?;
        let sigma = vol_surface.value_clamped(t_vol, inst.strike);
        let gbm_params = GbmParams::new(r, q, sigma);
        let process = GbmProcess::new(gbm_params);

        // Steps and payoff (using vol surface time basis)
        let steps_per_year = self.config.steps_per_year;
        let num_steps = ((t_vol * steps_per_year).round() as usize).max(self.config.min_steps);
        let maturity_step = num_steps - 1;
        let mc_barrier_type = Self::convert_barrier_type(inst.barrier_type);
        let payoff = BarrierOptionPayoff::new(
            inst.strike,
            inst.barrier.amount(),
            mc_barrier_type,
            inst.option_type,
            inst.rebate.map(|m| m.amount()),
            inst.notional.amount(),
            maturity_step,
            sigma,
            t_vol,
            inst.use_gobet_miri,
        );

        // Seed
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
        let mut cfg = self.config.clone();
        cfg.seed = seed;

        let pricer = PathDependentPricer::new(cfg);
        let (est, greeks) = pricer.price_with_lrm_greeks(
            &process,
            spot,
            t_vol,
            num_steps,
            &payoff,
            inst.notional.currency(),
            discount_factor,
            r,
            q,
            sigma,
        )?;

        Ok((est.mean, greeks))
    }
}

#[cfg(feature = "mc")]
impl Default for BarrierOptionMcPricer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "mc")]
impl Pricer for BarrierOptionMcPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::BarrierOption, ModelKey::MonteCarloGBM)
    }

    fn price_dyn(
        &self,
        instrument: &dyn crate::instruments::common_impl::traits::Instrument,
        market: &MarketContext,
        as_of: Date,
    ) -> PricingResult<ValuationResult> {
        let barrier = instrument
            .as_any()
            .downcast_ref::<BarrierOption>()
            .ok_or_else(|| {
                PricingError::type_mismatch(InstrumentType::BarrierOption, instrument.key())
            })?;

        let pv = self.price_internal(barrier, market, as_of).map_err(|e| {
            PricingError::model_failure_with_context(e.to_string(), PricingErrorContext::default())
        })?;

        Ok(ValuationResult::stamped(barrier.id(), as_of, pv))
    }
}

/// Present value using Monte Carlo.
#[cfg(feature = "mc")]
pub(crate) fn compute_pv(
    inst: &BarrierOption,
    curves: &MarketContext,
    as_of: Date,
) -> finstack_core::Result<Money> {
    let pricer = BarrierOptionMcPricer::new();
    pricer.price_internal(inst, curves, as_of)
}

/// Present value with LRM Greeks via Monte Carlo (barrier option).
///
/// Returns `(pv, Option<(delta, vega)>)` where the Greeks are from the
/// Likelihood Ratio Method. Greeks are `None` if the option is expired.
#[allow(dead_code)] // May be used by external bindings or tests
#[cfg(feature = "mc")]
pub fn npv_with_lrm_greeks(
    inst: &BarrierOption,
    curves: &MarketContext,
    as_of: Date,
) -> finstack_core::Result<(Money, Option<(f64, f64)>)> {
    let pricer = BarrierOptionMcPricer::new();
    pricer.price_with_lrm_greeks_internal(inst, curves, as_of)
}

// ========================= ANALYTICAL PRICER =========================

use crate::instruments::common_impl::helpers::collect_black_scholes_inputs;
use crate::instruments::common_impl::models::closed_form::barrier::{
    barrier_call_continuous, barrier_put_continuous, barrier_rebate_continuous,
    BarrierType as AnalyticalBarrierType,
};

/// Barrier option analytical pricer (continuous monitoring).
///
/// # Monitoring Convention
///
/// **Important**: This pricer uses **continuous monitoring** Reiner-Rubinstein formulas.
/// Real-world barriers are typically monitored discretely (e.g., daily closes).
/// Continuous barrier formulas **systematically underestimate** knock-out option values
/// and overestimate knock-in option values compared to discrete monitoring.
///
/// For discrete monitoring pricing, use the Monte Carlo pricer
/// ([`BarrierOptionMcPricer`]) which applies the Broadie-Glasserman-Kou / Gobet-Miri
/// correction when `use_gobet_miri = true`.
///
/// The `BarrierOption::value()` method dispatches to this analytical pricer for speed,
/// so be aware that it returns continuous-monitoring prices even when
/// `use_gobet_miri = true`. Use `npv_mc()` for discrete-monitoring-corrected prices.
pub struct BarrierOptionAnalyticalPricer;

impl BarrierOptionAnalyticalPricer {
    /// Create a new analytical barrier option pricer
    pub fn new() -> Self {
        Self
    }
}

impl Default for BarrierOptionAnalyticalPricer {
    fn default() -> Self {
        Self::new()
    }
}

impl Pricer for BarrierOptionAnalyticalPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::BarrierOption, ModelKey::BarrierBSContinuous)
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
        as_of: Date,
    ) -> PricingResult<ValuationResult> {
        let barrier_opt = instrument
            .as_any()
            .downcast_ref::<BarrierOption>()
            .ok_or_else(|| {
                PricingError::type_mismatch(InstrumentType::BarrierOption, instrument.key())
            })?;

        if barrier_opt.use_gobet_miri {
            tracing::warn!(
                "Analytical barrier pricer uses continuous monitoring; discrete monitoring flag \
                 is ignored. Use Monte Carlo pricer for discrete barrier monitoring."
            );
        }

        // Use standardized input collection
        let (spot, r, q, sigma, t) = collect_black_scholes_inputs(
            &barrier_opt.spot_id,
            &barrier_opt.discount_curve_id,
            barrier_opt.div_yield_id.as_ref(),
            &barrier_opt.vol_surface_id,
            barrier_opt.strike,
            barrier_opt.expiry,
            barrier_opt.day_count,
            market,
            as_of,
        )
        .map_err(|e| {
            PricingError::model_failure_with_context(e.to_string(), PricingErrorContext::default())
        })?;

        if t <= 0.0 {
            return Ok(ValuationResult::stamped(
                barrier_opt.id(),
                as_of,
                Money::new(0.0, barrier_opt.notional.currency()),
            ));
        }

        // Map barrier type
        use crate::instruments::exotics::barrier_option::types::BarrierType;
        let analytical_barrier_type = match barrier_opt.barrier_type {
            BarrierType::UpAndIn => AnalyticalBarrierType::UpIn,
            BarrierType::UpAndOut => AnalyticalBarrierType::UpOut,
            BarrierType::DownAndIn => AnalyticalBarrierType::DownIn,
            BarrierType::DownAndOut => AnalyticalBarrierType::DownOut,
        };

        let price = match barrier_opt.option_type {
            crate::instruments::OptionType::Call => barrier_call_continuous(
                spot,
                barrier_opt.strike,
                barrier_opt.barrier.amount(),
                t,
                r,
                q,
                sigma,
                analytical_barrier_type,
            ),
            crate::instruments::OptionType::Put => barrier_put_continuous(
                spot,
                barrier_opt.strike,
                barrier_opt.barrier.amount(),
                t,
                r,
                q,
                sigma,
                analytical_barrier_type,
            ),
        };

        let rebate_val = if let Some(rebate) = barrier_opt.rebate {
            barrier_rebate_continuous(
                spot,
                barrier_opt.barrier.amount(),
                rebate.amount(),
                t,
                r,
                q,
                sigma,
                analytical_barrier_type,
            )
        } else {
            0.0
        };

        let pv = Money::new(
            (price + rebate_val) * barrier_opt.notional.amount(),
            barrier_opt.notional.currency(),
        );
        Ok(ValuationResult::stamped(barrier_opt.id(), as_of, pv))
    }
}
