//! Barrier option Monte Carlo pricer.

use crate::instruments::barrier_option::types::{BarrierOption, BarrierType};
use crate::instruments::common::mc::pricer::path_dependent::{
    PathDependentPricer, PathDependentPricerConfig,
};
use crate::instruments::common::mc::payoff::barrier::BarrierCall;
use crate::instruments::common::mc::payoff::barrier::BarrierType as McBarrierType;
use crate::instruments::common::mc::process::gbm::{GbmParams, GbmProcess};
use crate::instruments::common::traits::Instrument;
use crate::pricer::{InstrumentType, ModelKey, Pricer, PricerKey, PricingError, PricingResult};
use crate::results::ValuationResult;
use finstack_core::dates::{Date, DayCountCtx};
use finstack_core::market_data::MarketContext;
use finstack_core::Result;
use finstack_core::money::Money;

/// Barrier option Monte Carlo pricer.
pub struct BarrierOptionMcPricer {
    config: PathDependentPricerConfig,
}

impl BarrierOptionMcPricer {
    /// Create a new barrier option MC pricer with default config.
    pub fn new() -> Self {
        Self {
            config: PathDependentPricerConfig::default(),
        }
    }

    fn convert_barrier_type(bt: BarrierType) -> McBarrierType {
        match bt {
            BarrierType::UpAndOut => McBarrierType::UpAndOut,
            BarrierType::UpAndIn => McBarrierType::UpAndIn,
            BarrierType::DownAndOut => McBarrierType::DownAndOut,
            BarrierType::DownAndIn => McBarrierType::DownAndIn,
        }
    }

    /// Price a barrier option using Monte Carlo.
    fn price_internal(
        &self,
        inst: &BarrierOption,
        curves: &MarketContext,
        as_of: Date,
    ) -> Result<finstack_core::money::Money> {
        // Get time to maturity
        let t = inst
            .day_count
            .year_fraction(as_of, inst.expiry, DayCountCtx::default())?;
        if t <= 0.0 {
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
                intrinsic * inst.notional,
                inst.strike.currency(),
            ));
        }

        // Get discount curve
        let disc_curve = curves.get_discount_ref(inst.disc_id.as_str())?;
        let r = disc_curve.zero(t);
        let t_as_of = disc_curve.day_count().year_fraction(
            disc_curve.base_date(),
            as_of,
            DayCountCtx::default(),
        )?;
        let df_as_of = disc_curve.df(t_as_of);
        let df_maturity = disc_curve.df(t_as_of + t);
        let discount_factor = if df_as_of > 0.0 { df_maturity / df_as_of } else { 1.0 };

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

        // Get volatility
        let vol_surface = curves.surface_ref(inst.vol_id.as_str())?;
        let sigma = vol_surface.value_clamped(t, inst.strike.amount());

        // Create GBM process
        let gbm_params = GbmParams::new(r, q, sigma);
        let process = GbmProcess::new(gbm_params);

        // Create time grid
        let num_steps = (t * 252.0) as usize; // Daily steps
        let maturity_step = num_steps - 1;

        // Create payoff
        let mc_barrier_type = Self::convert_barrier_type(inst.barrier_type);
        let payoff = BarrierCall::new(
            inst.strike.amount(),
            inst.barrier.amount(),
            mc_barrier_type,
            inst.notional,
            maturity_step,
            sigma,
            t,
            inst.use_gobet_miri,
        );

        // Price using path-dependent pricer
        let pricer = PathDependentPricer::new(self.config.clone());
        let result = pricer.price(
            &process,
            spot,
            t,
            num_steps,
            &payoff,
            inst.strike.currency(),
            discount_factor,
        )?;

        Ok(result.mean)
    }
}

impl Default for BarrierOptionMcPricer {
    fn default() -> Self {
        Self::new()
    }
}

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
            .ok_or_else(|| PricingError::type_mismatch(InstrumentType::BarrierOption, instrument.key()))?;

        let pv = self
            .price_internal(barrier, market, as_of)
            .map_err(|e| PricingError::model_failure(e.to_string()))?;

        Ok(ValuationResult::stamped(barrier.id(), as_of, pv))
    }
}

/// Present value using Monte Carlo.
pub fn npv(inst: &BarrierOption, curves: &MarketContext, as_of: Date) -> Result<Money> {
    let pricer = BarrierOptionMcPricer::new();
    pricer.price_internal(inst, curves, as_of)
}

