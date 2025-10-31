//! Cliquet option Monte Carlo pricer.

use crate::instruments::cliquet_option::types::CliquetOption;
use crate::instruments::common::mc::payoff::cliquet::CliquetCallPayoff;
use crate::instruments::common::mc::pricer::path_dependent::{
    PathDependentPricer, PathDependentPricerConfig,
};
use crate::instruments::common::mc::process::gbm::{GbmParams, GbmProcess};
use crate::instruments::common::traits::Instrument;
use crate::pricer::{InstrumentType, ModelKey, Pricer, PricerKey, PricingError, PricingResult};
use crate::results::ValuationResult;
use finstack_core::dates::{Date, DayCountCtx};
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::Result;

/// Cliquet option Monte Carlo pricer.
pub struct CliquetOptionMcPricer {
    config: PathDependentPricerConfig,
}

impl CliquetOptionMcPricer {
    /// Create a new cliquet option MC pricer with default config.
    pub fn new() -> Self {
        Self {
            config: PathDependentPricerConfig::default(),
        }
    }

    /// Price a cliquet option using Monte Carlo.
    fn price_internal(
        &self,
        inst: &CliquetOption,
        curves: &MarketContext,
        as_of: Date,
    ) -> Result<finstack_core::money::Money> {
        let spot_scalar = curves.price(&inst.spot_id)?;
        let initial_spot = match spot_scalar {
            finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
            finstack_core::market_data::scalars::MarketScalar::Price(m) => m.amount(),
        };

        let final_date = inst.reset_dates.last().copied().unwrap_or(as_of);
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

        // Map reset dates to times
        let reset_times: Vec<f64> = inst
            .reset_dates
            .iter()
            .map(|&date| {
                inst.day_count
                    .year_fraction(as_of, date, DayCountCtx::default())
                    .unwrap_or(0.0)
            })
            .collect();

        let payoff = CliquetCallPayoff::new(
            reset_times,
            inst.local_cap,
            inst.global_cap,
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

impl Default for CliquetOptionMcPricer {
    fn default() -> Self {
        Self::new()
    }
}

impl Pricer for CliquetOptionMcPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::CliquetOption, ModelKey::MonteCarloGBM)
    }

    fn price_dyn(
        &self,
        instrument: &dyn crate::instruments::common::traits::Instrument,
        market: &MarketContext,
        as_of: Date,
    ) -> PricingResult<ValuationResult> {
        let cliquet = instrument
            .as_any()
            .downcast_ref::<CliquetOption>()
            .ok_or_else(|| {
                PricingError::type_mismatch(InstrumentType::CliquetOption, instrument.key())
            })?;

        let pv = self
            .price_internal(cliquet, market, as_of)
            .map_err(|e| PricingError::model_failure(e.to_string()))?;

        Ok(ValuationResult::stamped(cliquet.id(), as_of, pv))
    }
}

/// Present value using Monte Carlo.
pub fn npv(inst: &CliquetOption, curves: &MarketContext, as_of: Date) -> Result<Money> {
    let pricer = CliquetOptionMcPricer::new();
    pricer.price_internal(inst, curves, as_of)
}
