//! Lookback option Monte Carlo pricer.

#[cfg(feature = "mc")]
use crate::instruments::common::mc::payoff::lookback::{
    FloatingStrikeLookbackCall, LookbackCall, LookbackPut,
};
#[cfg(feature = "mc")]
use crate::instruments::common::mc::pricer::path_dependent::{
    PathDependentPricer, PathDependentPricerConfig,
};
#[cfg(feature = "mc")]
use crate::instruments::common::mc::process::gbm::{GbmParams, GbmProcess};
#[cfg(feature = "mc")]
use crate::instruments::common::traits::Instrument;
#[cfg(feature = "mc")]
use crate::instruments::lookback_option::types::{LookbackOption, LookbackType};
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
    ) -> Result<finstack_core::money::Money> {
        let t = inst
            .day_count
            .year_fraction(as_of, inst.expiry, DayCountCtx::default())?;
        if t <= 0.0 {
            return Ok(finstack_core::money::Money::new(
                0.0,
                inst.strike
                    .map(|s| s.currency())
                    .unwrap_or(finstack_core::currency::Currency::USD),
            ));
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

        let vol_surface = curves.surface_ref(inst.vol_id.as_str())?;
        let strike_val = inst.strike.as_ref().map(|s| s.amount()).unwrap_or(spot);
        let sigma = vol_surface.value_clamped(t, strike_val);

        let gbm_params = GbmParams::new(r, q, sigma);
        let process = GbmProcess::new(gbm_params);

        let num_steps = (t * 252.0) as usize;
        let maturity_step = num_steps - 1;

        let currency = inst
            .strike
            .as_ref()
            .map(|s| s.currency())
            .unwrap_or(finstack_core::currency::Currency::USD);

        // Derive deterministic seed from instrument ID and scenario
        #[cfg(feature = "mc")]
        use crate::instruments::common::mc::seed;

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
        let result = match (inst.lookback_type, inst.option_type) {
            (LookbackType::FloatingStrike, _) => {
                let payoff = FloatingStrikeLookbackCall::new(inst.notional, maturity_step);
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
                let payoff = LookbackCall::new(
                    inst.strike.as_ref().unwrap().amount(),
                    inst.notional,
                    maturity_step,
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
                let payoff = LookbackPut::new(
                    inst.strike.as_ref().unwrap().amount(),
                    inst.notional,
                    maturity_step,
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
        instrument: &dyn crate::instruments::common::traits::Instrument,
        market: &MarketContext,
        as_of: Date,
    ) -> PricingResult<ValuationResult> {
        let lookback = instrument
            .as_any()
            .downcast_ref::<LookbackOption>()
            .ok_or_else(|| {
                PricingError::type_mismatch(InstrumentType::LookbackOption, instrument.key())
            })?;

        let pv = self
            .price_internal(lookback, market, as_of)
            .map_err(|e| PricingError::model_failure(e.to_string()))?;

        Ok(ValuationResult::stamped(lookback.id(), as_of, pv))
    }
}

/// Present value using Monte Carlo.
#[cfg(feature = "mc")]
pub fn npv(inst: &LookbackOption, curves: &MarketContext, as_of: Date) -> Result<Money> {
    let pricer = LookbackOptionMcPricer::new();
    pricer.price_internal(inst, curves, as_of)
}
