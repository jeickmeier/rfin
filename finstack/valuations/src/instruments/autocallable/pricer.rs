//! Autocallable Monte Carlo pricer.

#[cfg(feature = "mc")]
use crate::instruments::autocallable::types::{Autocallable, FinalPayoffType};
#[cfg(feature = "mc")]
use crate::instruments::common::mc::process::gbm::{GbmParams, GbmProcess};
#[cfg(feature = "mc")]
use crate::instruments::common::models::monte_carlo::payoff::autocallable::{
    AutocallablePayoff, FinalPayoffType as McFinalPayoffType,
};
#[cfg(feature = "mc")]
use crate::instruments::common::models::monte_carlo::pricer::path_dependent::{
    PathDependentPricer, PathDependentPricerConfig,
};
#[cfg(feature = "mc")]
use crate::instruments::common::traits::Instrument;
#[cfg(feature = "mc")]
use crate::pricer::{InstrumentType, ModelKey, Pricer, PricerKey, PricingError, PricingResult};
#[cfg(feature = "mc")]
use crate::results::ValuationResult;
#[cfg(feature = "mc")]
use finstack_core::dates::{Date, DayCountCtx};
#[cfg(feature = "mc")]
use finstack_core::market_data::MarketContext;
#[cfg(feature = "mc")]
use finstack_core::money::Money;
#[cfg(feature = "mc")]
use finstack_core::Result;

/// Autocallable Monte Carlo pricer.
#[cfg(feature = "mc")]
pub struct AutocallableMcPricer {
    config: PathDependentPricerConfig,
}

#[cfg(feature = "mc")]
impl AutocallableMcPricer {
    /// Create a new autocallable MC pricer with default config.
    pub fn new() -> Self {
        Self {
            config: PathDependentPricerConfig::default(),
        }
    }

    fn convert_final_payoff_type(ft: FinalPayoffType) -> McFinalPayoffType {
        match ft {
            FinalPayoffType::CapitalProtection { floor } => {
                McFinalPayoffType::CapitalProtection { floor }
            }
            FinalPayoffType::Participation { rate } => McFinalPayoffType::Participation { rate },
            FinalPayoffType::KnockInPut { strike } => McFinalPayoffType::KnockInPut { strike },
        }
    }

    /// Price an autocallable using Monte Carlo.
    fn price_internal(
        &self,
        inst: &Autocallable,
        curves: &MarketContext,
        as_of: Date,
    ) -> Result<finstack_core::money::Money> {
        let spot_scalar = curves.price(&inst.spot_id)?;
        let initial_spot = match spot_scalar {
            finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
            finstack_core::market_data::scalars::MarketScalar::Price(m) => m.amount(),
        };

        let disc_curve = curves.get_discount_ref(inst.discount_curve_id.as_str())?;

        // Get time to final observation date using the discount curve's basis to
        // align DF/zero calculations with the time grid.
        let final_date = inst
            .observation_dates
            .last()
            .copied()
            .unwrap_or(inst.observation_dates[0]);
        let disc_dc = disc_curve.day_count();
        let t = disc_dc.year_fraction(as_of, final_date, DayCountCtx::default())?;
        if t <= 0.0 {
            return Ok(Money::new(0.0, inst.notional.currency()));
        }

        let r = disc_curve.zero(t);
        let t_as_of = disc_curve.day_count().year_fraction(
            disc_curve.base_date(),
            as_of,
            DayCountCtx::default(),
        )?;
        let df_as_of = disc_curve.df(t_as_of);
        let df_maturity = disc_curve.df(t_as_of + t);
        let discount_factor = if df_as_of > 0.0 {
            df_maturity / df_as_of
        } else {
            1.0
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

        // NOTE: Vol surface expiries are assumed to be expressed in the same day count
        // convention as the discount curve (both typically use ACT/365F for equity vol).
        // If the surface was built with a different convention, this lookup may be
        // slightly off. Consider adding explicit day_count to VolSurface in future.
        let vol_surface = curves.surface_ref(inst.vol_surface_id.as_str())?;
        let sigma = vol_surface.value_clamped(t, initial_spot);

        let gbm_params = GbmParams::new(r, q, sigma);
        let process = GbmProcess::new(gbm_params);

        let steps_per_year = self.config.steps_per_year;
        let num_steps = ((t * steps_per_year).round() as usize).max(self.config.min_steps);

        // Map observation dates to times
        let observation_times: Vec<f64> = inst
            .observation_dates
            .iter()
            .map(|&date| {
                disc_dc
                    .year_fraction(as_of, date, DayCountCtx::default())
                    .unwrap_or(0.0)
            })
            .collect();

        let mc_final_payoff = Self::convert_final_payoff_type(inst.final_payoff_type);

        // Calculate discount factor ratios for each observation date
        // Ratio = DF(T_obs) / DF(T_mat)
        // This corrects for the engine applying DF(T_mat) to early cashflows
        //
        // IMPORTANT: Use discount curve's day count (disc_dc) consistently for all
        // time calculations. Mixing inst.day_count with disc_dc would distort timing
        // and coupon PVs. The observation_times above already use disc_dc, so the
        // discount factor lookups must match to ensure consistent discounting.
        let df_ratios: Vec<f64> = observation_times
            .iter()
            .map(|&t_obs| {
                let df_obs = disc_curve.df(t_as_of + t_obs.max(0.0));
                if df_maturity > 0.0 {
                    df_obs / df_maturity
                } else {
                    1.0
                }
            })
            .collect();

        let payoff = AutocallablePayoff::new(
            observation_times.clone(),
            inst.autocall_barriers.clone(),
            inst.coupons.clone(),
            inst.final_barrier,
            mc_final_payoff,
            inst.participation_rate,
            inst.cap_level,
            inst.notional.amount(),
            inst.notional.currency(),
            initial_spot,
            df_ratios,
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

        // Create time grid that includes observation dates to ensure exact event timing
        #[cfg(feature = "mc")]
        use crate::instruments::common::mc::time_grid::TimeGrid;

        let mut grid_times = Vec::with_capacity(num_steps + observation_times.len() + 1);
        grid_times.push(0.0);

        // Add uniform steps
        let dt = t / num_steps as f64;
        for i in 1..=num_steps {
            grid_times.push(i as f64 * dt);
        }

        // Add observation times (ensure we visit exact dates)
        for &obs_t in &observation_times {
            if obs_t > 1e-10 && obs_t <= t {
                grid_times.push(obs_t);
            }
        }

        // Sort and dedup
        grid_times.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        grid_times.dedup_by(|a, b| (*a - *b).abs() < 1e-10);

        let time_grid = TimeGrid::from_times(grid_times)?;

        let mut config = self.config.clone();
        config.seed = seed;
        let pricer = PathDependentPricer::new(config);
        let result = pricer.price_with_grid(
            &process,
            initial_spot,
            time_grid,
            &payoff,
            inst.notional.currency(),
            discount_factor,
        )?;

        Ok(result.mean)
    }
}

#[cfg(feature = "mc")]
impl Default for AutocallableMcPricer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "mc")]
impl Pricer for AutocallableMcPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::Autocallable, ModelKey::MonteCarloGBM)
    }

    fn price_dyn(
        &self,
        instrument: &dyn crate::instruments::common::traits::Instrument,
        market: &MarketContext,
        as_of: Date,
    ) -> PricingResult<ValuationResult> {
        let autocallable = instrument
            .as_any()
            .downcast_ref::<Autocallable>()
            .ok_or_else(|| {
                PricingError::type_mismatch(InstrumentType::Autocallable, instrument.key())
            })?;

        let pv = self
            .price_internal(autocallable, market, as_of)
            .map_err(|e| PricingError::model_failure(e.to_string()))?;

        Ok(ValuationResult::stamped(autocallable.id(), as_of, pv))
    }
}

/// Present value using Monte Carlo.
#[cfg(feature = "mc")]
pub fn npv(inst: &Autocallable, curves: &MarketContext, as_of: Date) -> Result<Money> {
    let pricer = AutocallableMcPricer::new();
    pricer.price_internal(inst, curves, as_of)
}
