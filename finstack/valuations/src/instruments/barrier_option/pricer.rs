//! Barrier option pricers (Monte Carlo and analytical).

// Common imports for all pricers
use crate::instruments::barrier_option::types::BarrierOption;
use crate::instruments::common::traits::Instrument;
use crate::pricer::{InstrumentType, ModelKey, Pricer, PricerKey, PricingError, PricingResult};
use crate::results::ValuationResult;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;

// DayCountCtx is only used by MC pricer
#[cfg(feature = "mc")]
use finstack_core::dates::DayCountCtx;

// MC-specific imports
#[cfg(feature = "mc")]
use crate::instruments::common::mc::process::gbm::{GbmParams, GbmProcess};
#[cfg(feature = "mc")]
use crate::instruments::common::models::monte_carlo::payoff::barrier::BarrierOptionPayoff;
#[cfg(feature = "mc")]
use crate::instruments::common::models::monte_carlo::payoff::barrier::BarrierType as McBarrierType;
#[cfg(feature = "mc")]
use crate::instruments::common::models::monte_carlo::pricer::path_dependent::{
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
        bt: crate::instruments::barrier_option::types::BarrierType,
    ) -> McBarrierType {
        match bt {
            crate::instruments::barrier_option::types::BarrierType::UpAndOut => {
                McBarrierType::UpAndOut
            }
            crate::instruments::barrier_option::types::BarrierType::UpAndIn => {
                McBarrierType::UpAndIn
            }
            crate::instruments::barrier_option::types::BarrierType::DownAndOut => {
                McBarrierType::DownAndOut
            }
            crate::instruments::barrier_option::types::BarrierType::DownAndIn => {
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
        let disc_curve = curves.get_discount_ref(inst.discount_curve_id.as_str())?;

        // Time to maturity for discounting (using discount curve's basis)
        let t_disc =
            disc_curve
                .day_count()
                .year_fraction(as_of, inst.expiry, DayCountCtx::default())?;

        // Time to maturity for vol surface (using instrument's day count, which should
        // match how the vol surface was calibrated)
        let t_vol = inst
            .day_count
            .year_fraction(as_of, inst.expiry, DayCountCtx::default())?;

        if t_vol <= 0.0 {
            // Expired: return intrinsic value if barrier not hit (simplified)
            let spot_scalar = curves.price(&inst.spot_id)?;
            let spot = match spot_scalar {
                finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
                finstack_core::market_data::scalars::MarketScalar::Price(m) => m.amount(),
            };
            let intrinsic = match inst.option_type {
                crate::instruments::OptionType::Call => (spot - inst.strike.amount()).max(0.0),
                crate::instruments::OptionType::Put => (inst.strike.amount() - spot).max(0.0),
            };
            return Ok(finstack_core::money::Money::new(
                intrinsic * inst.notional.amount(),
                inst.strike.currency(),
            ));
        }

        // Risk-free rate using discount curve's time basis
        let r = disc_curve.zero(t_disc);
        let discount_factor = disc_curve.try_df_between_dates(as_of, inst.expiry)?;

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
        let vol_surface = curves.surface_ref(inst.vol_surface_id.as_str())?;
        let sigma = vol_surface.value_clamped(t_vol, inst.strike.amount());

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
            inst.strike.amount(),
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
            inst.strike.currency(),
            discount_factor,
        )?;

        Ok(result.mean)
    }

    /// Price with LRM Greeks (delta, vega) convenience for barrier options.
    ///
    /// # Day Count Convention Handling
    ///
    /// Uses separate day count bases for different purposes:
    /// - **Discounting**: Uses the discount curve's own day count for DF and zero rate calculations
    /// - **Volatility lookup and MC simulation**: Uses the instrument's day count (assumed to match vol surface calibration)
    pub fn price_with_lrm_greeks_internal(
        &self,
        inst: &BarrierOption,
        curves: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        // Get discount curve first to access its day count
        let disc_curve = curves.get_discount_ref(inst.discount_curve_id.as_str())?;

        // Time to maturity for vol surface (using instrument's day count)
        let t_vol = inst
            .day_count
            .year_fraction(as_of, inst.expiry, DayCountCtx::default())?;
        if t_vol <= 0.0 {
            return Ok(finstack_core::money::Money::new(
                0.0,
                inst.strike.currency(),
            ));
        }

        // Time to maturity for discounting (using discount curve's basis)
        let t_disc =
            disc_curve
                .day_count()
                .year_fraction(as_of, inst.expiry, DayCountCtx::default())?;

        // Risk-free rate using discount curve's time basis
        let r = disc_curve.zero(t_disc);
        let discount_factor = disc_curve.try_df_between_dates(as_of, inst.expiry)?;

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
        let vol_surface = curves.surface_ref(inst.vol_surface_id.as_str())?;
        let sigma = vol_surface.value_clamped(t_vol, inst.strike.amount());
        let gbm_params = GbmParams::new(r, q, sigma);
        let process = GbmProcess::new(gbm_params);

        // Steps and payoff (using vol surface time basis)
        let steps_per_year = self.config.steps_per_year;
        let num_steps = ((t_vol * steps_per_year).round() as usize).max(self.config.min_steps);
        let maturity_step = num_steps - 1;
        let mc_barrier_type = Self::convert_barrier_type(inst.barrier_type);
        let payoff = BarrierOptionPayoff::new(
            inst.strike.amount(),
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
        let mut cfg = self.config.clone();
        cfg.seed = seed;

        let pricer = PathDependentPricer::new(cfg);
        let (est, _greeks) = pricer.price_with_lrm_greeks(
            &process,
            spot,
            t_vol,
            num_steps,
            &payoff,
            inst.strike.currency(),
            discount_factor,
            r,
            q,
            sigma,
        )?;

        Ok(est.mean)
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
        instrument: &dyn crate::instruments::common::traits::Instrument,
        market: &MarketContext,
        as_of: Date,
    ) -> PricingResult<ValuationResult> {
        let barrier = instrument
            .as_any()
            .downcast_ref::<BarrierOption>()
            .ok_or_else(|| {
                PricingError::type_mismatch(InstrumentType::BarrierOption, instrument.key())
            })?;

        let pv = self
            .price_internal(barrier, market, as_of)
            .map_err(|e| PricingError::model_failure(e.to_string()))?;

        Ok(ValuationResult::stamped(barrier.id(), as_of, pv))
    }
}

/// Present value using Monte Carlo.
#[cfg(feature = "mc")]
pub fn npv(
    inst: &BarrierOption,
    curves: &MarketContext,
    as_of: Date,
) -> finstack_core::Result<Money> {
    let pricer = BarrierOptionMcPricer::new();
    pricer.price_internal(inst, curves, as_of)
}

/// Present value with LRM Greeks via Monte Carlo (barrier option).
#[cfg(feature = "mc")]
pub fn npv_with_lrm_greeks(
    inst: &BarrierOption,
    curves: &MarketContext,
    as_of: Date,
) -> finstack_core::Result<Money> {
    let pricer = BarrierOptionMcPricer::new();
    pricer.price_with_lrm_greeks_internal(inst, curves, as_of)
}

// ========================= ANALYTICAL PRICER =========================

use crate::instruments::common::helpers::collect_black_scholes_inputs;
use crate::instruments::common::models::closed_form::barrier::{
    barrier_call_continuous, barrier_put_continuous, barrier_rebate_continuous,
    BarrierType as AnalyticalBarrierType,
};

/// Barrier option analytical pricer (continuous monitoring).
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

        // Use standardized input collection
        let (spot, r, q, sigma, t) = collect_black_scholes_inputs(
            &barrier_opt.spot_id,
            &barrier_opt.discount_curve_id,
            barrier_opt.div_yield_id.as_ref(),
            &barrier_opt.vol_surface_id,
            barrier_opt.strike.amount(),
            barrier_opt.expiry,
            barrier_opt.day_count,
            market,
            as_of,
        )
        .map_err(|e| PricingError::model_failure(e.to_string()))?;

        if t <= 0.0 {
            return Ok(ValuationResult::stamped(
                barrier_opt.id(),
                as_of,
                Money::new(0.0, barrier_opt.strike.currency()),
            ));
        }

        // Map barrier type
        use crate::instruments::barrier_option::types::BarrierType;
        let analytical_barrier_type = match barrier_opt.barrier_type {
            BarrierType::UpAndIn => AnalyticalBarrierType::UpIn,
            BarrierType::UpAndOut => AnalyticalBarrierType::UpOut,
            BarrierType::DownAndIn => AnalyticalBarrierType::DownIn,
            BarrierType::DownAndOut => AnalyticalBarrierType::DownOut,
        };

        let price = match barrier_opt.option_type {
            crate::instruments::OptionType::Call => barrier_call_continuous(
                spot,
                barrier_opt.strike.amount(),
                barrier_opt.barrier.amount(),
                t,
                r,
                q,
                sigma,
                analytical_barrier_type,
            ),
            crate::instruments::OptionType::Put => barrier_put_continuous(
                spot,
                barrier_opt.strike.amount(),
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
            barrier_opt.strike.currency(),
        );
        Ok(ValuationResult::stamped(barrier_opt.id(), as_of, pv))
    }
}
