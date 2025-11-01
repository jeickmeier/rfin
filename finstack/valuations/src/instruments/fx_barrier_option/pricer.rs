//! FX barrier option pricers (Monte Carlo and analytical).

// Common imports for all pricers
use crate::instruments::barrier_option::types::BarrierType;
use crate::instruments::common::traits::Instrument;
use crate::instruments::fx_barrier_option::types::FxBarrierOption;
use crate::pricer::{InstrumentType, ModelKey, Pricer, PricerKey, PricingError, PricingResult};
use crate::results::ValuationResult;
use finstack_core::dates::{Date, DayCountCtx};
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::Result;

// MC-specific imports
#[cfg(feature = "mc")]
use crate::instruments::common::mc::payoff::barrier::BarrierType as McBarrierType;
#[cfg(feature = "mc")]
use crate::instruments::common::mc::payoff::fx_barrier::FxBarrierCall;
#[cfg(feature = "mc")]
use crate::instruments::common::mc::pricer::path_dependent::{
    PathDependentPricer, PathDependentPricerConfig,
};
#[cfg(feature = "mc")]
use crate::instruments::common::mc::process::gbm::{GbmParams, GbmProcess};

/// FX barrier option Monte Carlo pricer.
#[cfg(feature = "mc")]
pub struct FxBarrierOptionMcPricer {
    config: PathDependentPricerConfig,
}

#[cfg(feature = "mc")]
impl FxBarrierOptionMcPricer {
    /// Create a new FX barrier option MC pricer with default config.
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

    /// Price an FX barrier option using Monte Carlo.
    fn price_internal(
        &self,
        inst: &FxBarrierOption,
        curves: &MarketContext,
        as_of: Date,
    ) -> Result<finstack_core::money::Money> {
        let t = inst
            .day_count
            .year_fraction(as_of, inst.expiry, DayCountCtx::default())?;
        if t <= 0.0 {
            return Ok(finstack_core::money::Money::new(
                0.0,
                inst.domestic_currency,
            ));
        }

        let disc_curve = curves.get_discount_ref(inst.disc_id.as_str())?;
        let r_dom = disc_curve.zero(t);
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

        let spot_scalar = curves.price(&inst.fx_spot_id)?;
        let fx_spot = match spot_scalar {
            finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
            finstack_core::market_data::scalars::MarketScalar::Price(m) => m.amount(),
        };

        let vol_surface = curves.surface_ref(inst.fx_vol_id.as_str())?;
        let sigma = vol_surface.value_clamped(t, inst.strike.amount());

        // For FX, drift is typically r_dom - r_for, simplified here
        let q = 0.0; // Foreign rate handled via quanto adjustment if needed
        let gbm_params = GbmParams::new(r_dom, q, sigma);
        let process = GbmProcess::new(gbm_params);

        let num_steps = (t * 252.0) as usize;
        let dt = t / num_steps as f64;
        let maturity_step = num_steps - 1;

        // Compute quanto adjustment if correlation provided
        let quanto_adjustment = if inst.correlation != 0.0 {
            // Simplified: would need FX volatility
            inst.correlation * sigma * 0.12 // Placeholder
        } else {
            0.0
        };

        let mc_barrier_type = Self::convert_barrier_type(inst.barrier_type);
        let payoff = FxBarrierCall::new(
            inst.strike.amount(),
            inst.barrier.amount(),
            mc_barrier_type,
            inst.notional,
            maturity_step,
            sigma,
            dt,
            inst.use_gobet_miri,
            inst.domestic_currency,
            inst.foreign_currency,
            quanto_adjustment,
        );

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
        let result = pricer.price(
            &process,
            fx_spot,
            t,
            num_steps,
            &payoff,
            inst.domestic_currency,
            discount_factor,
        )?;

        Ok(result.mean)
    }
}

#[cfg(feature = "mc")]
impl Default for FxBarrierOptionMcPricer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "mc")]
impl Pricer for FxBarrierOptionMcPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::FxBarrierOption, ModelKey::MonteCarloGBM)
    }

    fn price_dyn(
        &self,
        instrument: &dyn crate::instruments::common::traits::Instrument,
        market: &MarketContext,
        as_of: Date,
    ) -> PricingResult<ValuationResult> {
        let fx_barrier = instrument
            .as_any()
            .downcast_ref::<FxBarrierOption>()
            .ok_or_else(|| {
                PricingError::type_mismatch(InstrumentType::FxBarrierOption, instrument.key())
            })?;

        let pv = self
            .price_internal(fx_barrier, market, as_of)
            .map_err(|e| PricingError::model_failure(e.to_string()))?;

        Ok(ValuationResult::stamped(fx_barrier.id(), as_of, pv))
    }
}

/// Present value using Monte Carlo.
#[cfg(feature = "mc")]
pub fn npv(inst: &FxBarrierOption, curves: &MarketContext, as_of: Date) -> Result<Money> {
    let pricer = FxBarrierOptionMcPricer::new();
    pricer.price_internal(inst, curves, as_of)
}

// ========================= ANALYTICAL PRICER =========================

use crate::instruments::common::analytical::barrier::{
    barrier_call_continuous, barrier_put_continuous, BarrierType as AnalyticalBarrierType,
};

/// Helper to collect inputs for FX barrier option pricing.
fn collect_fx_barrier_inputs(
    inst: &FxBarrierOption,
    curves: &MarketContext,
    as_of: Date,
) -> finstack_core::Result<(f64, f64, f64, f64)> {
    let t = inst.day_count.year_fraction(as_of, inst.expiry, DayCountCtx::default())?;
    
    let disc_curve = curves.get_discount_ref(inst.disc_id.as_str())?;
    let r_dom = disc_curve.zero(t);
    
    let spot_scalar = curves.price(&inst.fx_spot_id)?;
    let fx_spot = match spot_scalar {
        finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
        finstack_core::market_data::scalars::MarketScalar::Price(m) => m.amount(),
    };
    
    let vol_surface = curves.surface_ref(inst.fx_vol_id.as_str())?;
    let sigma = vol_surface.value_clamped(t, inst.strike.amount());
    
    Ok((fx_spot, r_dom, sigma, t))
}

/// FX Barrier option analytical pricer (continuous monitoring).
pub struct FxBarrierOptionAnalyticalPricer;

impl FxBarrierOptionAnalyticalPricer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for FxBarrierOptionAnalyticalPricer {
    fn default() -> Self {
        Self::new()
    }
}

impl Pricer for FxBarrierOptionAnalyticalPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::FxBarrierOption, ModelKey::FxBarrierBSContinuous)
    }
    
    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
        as_of: Date,
    ) -> PricingResult<ValuationResult> {
        let fx_barrier = instrument
            .as_any()
            .downcast_ref::<FxBarrierOption>()
            .ok_or_else(|| {
                PricingError::type_mismatch(InstrumentType::FxBarrierOption, instrument.key())
            })?;
        
        let (fx_spot, r_dom, sigma, t) = collect_fx_barrier_inputs(fx_barrier, market, as_of)
            .map_err(|e| PricingError::model_failure(e.to_string()))?;
        
        if t <= 0.0 {
            return Ok(ValuationResult::stamped(
                fx_barrier.id(),
                as_of,
                Money::new(0.0, fx_barrier.domestic_currency),
            ));
        }
        
        // For FX, q = r_for (foreign rate, simplified to r_dom for now)
        let r_for = r_dom; // Simplified: would fetch from separate curve in production
        
        // Map barrier type
        use crate::instruments::barrier_option::types::BarrierType;
        let analytical_barrier_type = match fx_barrier.barrier_type {
            BarrierType::UpAndIn => AnalyticalBarrierType::UpIn,
            BarrierType::UpAndOut => AnalyticalBarrierType::UpOut,
            BarrierType::DownAndIn => AnalyticalBarrierType::DownIn,
            BarrierType::DownAndOut => AnalyticalBarrierType::DownOut,
        };
        
        let price = match fx_barrier.option_type {
            crate::instruments::OptionType::Call => barrier_call_continuous(
                fx_spot,
                fx_barrier.strike.amount(),
                fx_barrier.barrier.amount(),
                t,
                r_dom,
                r_for, // q = r_for for FX
                sigma,
                analytical_barrier_type,
            ),
            crate::instruments::OptionType::Put => barrier_put_continuous(
                fx_spot,
                fx_barrier.strike.amount(),
                fx_barrier.barrier.amount(),
                t,
                r_dom,
                r_for,
                sigma,
                analytical_barrier_type,
            ),
        };
        
        let pv = Money::new(price * fx_barrier.notional, fx_barrier.domestic_currency);
        Ok(ValuationResult::stamped(fx_barrier.id(), as_of, pv))
    }
}
