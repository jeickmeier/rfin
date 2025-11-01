//! Range accrual Monte Carlo pricer.

#[cfg(feature = "mc")]
use crate::instruments::common::models::monte_carlo::payoff::range_accrual::RangeAccrualPayoff;
#[cfg(feature = "mc")]
use crate::instruments::common::models::monte_carlo::pricer::path_dependent::{
    PathDependentPricer, PathDependentPricerConfig,
};
#[cfg(feature = "mc")]
use crate::instruments::common::mc::process::gbm::{GbmParams, GbmProcess};
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
use finstack_core::market_data::MarketContext;
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

        let final_date = inst.observation_dates.last().copied().unwrap_or(as_of);
        let t = inst
            .day_count
            .year_fraction(as_of, final_date, DayCountCtx::default())?;
        if t <= 0.0 {
            return Ok(Money::new(0.0, inst.notional.currency()));
        }

        let disc_curve = curves.get_discount_ref(inst.disc_id.as_str())?;
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

        let vol_surface = curves.surface_ref(inst.vol_id.as_str())?;
        let sigma = vol_surface.value_clamped(t, initial_spot);

        let gbm_params = GbmParams::new(r, q, sigma);
        let process = GbmProcess::new(gbm_params);

        let num_steps = (t * 252.0) as usize;

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
    let pricer = RangeAccrualMcPricer::new();
    pricer.price_internal(inst, curves, as_of)
}
