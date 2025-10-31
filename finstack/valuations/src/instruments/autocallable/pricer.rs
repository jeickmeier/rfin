//! Autocallable Monte Carlo pricer.

use crate::instruments::autocallable::types::{Autocallable, FinalPayoffType};
use crate::instruments::common::mc::payoff::autocallable::{
    AutocallablePayoff, FinalPayoffType as McFinalPayoffType,
};
use crate::instruments::common::mc::pricer::path_dependent::{
    PathDependentPricer, PathDependentPricerConfig,
};
use crate::instruments::common::mc::process::gbm::{GbmParams, GbmProcess};
use crate::instruments::common::traits::Instrument;
use crate::pricer::{InstrumentType, ModelKey, Pricer, PricerKey, PricingError, PricingResult};
use crate::results::ValuationResult;
use finstack_core::dates::{Date, DayCountCtx};
use finstack_core::market_data::MarketContext;
use finstack_core::Result;
use finstack_core::money::Money;

/// Autocallable Monte Carlo pricer.
pub struct AutocallableMcPricer {
    config: PathDependentPricerConfig,
}

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

        // Get time to final observation date
        let final_date = inst.observation_dates.last().copied().unwrap_or(inst.observation_dates[0]);
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
        let discount_factor = if df_as_of > 0.0 { df_maturity / df_as_of } else { 1.0 };

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

        let mc_final_payoff = Self::convert_final_payoff_type(inst.final_payoff_type);

        let payoff = AutocallablePayoff::new(
            observation_times,
            inst.autocall_barriers.clone(),
            inst.coupons.clone(),
            inst.final_barrier,
            mc_final_payoff,
            inst.participation_rate,
            inst.cap_level,
            inst.notional.amount(),
            inst.notional.currency(),
            initial_spot,
        );

        let pricer = PathDependentPricer::new(self.config.clone());
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

impl Default for AutocallableMcPricer {
    fn default() -> Self {
        Self::new()
    }
}

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
            .ok_or_else(|| PricingError::type_mismatch(InstrumentType::Autocallable, instrument.key()))?;

        let pv = self
            .price_internal(autocallable, market, as_of)
            .map_err(|e| PricingError::model_failure(e.to_string()))?;

        Ok(ValuationResult::stamped(autocallable.id(), as_of, pv))
    }
}

/// Present value using Monte Carlo.
pub fn npv(inst: &Autocallable, curves: &MarketContext, as_of: Date) -> Result<Money> {
    let pricer = AutocallableMcPricer::new();
    pricer.price_internal(inst, curves, as_of)
}

