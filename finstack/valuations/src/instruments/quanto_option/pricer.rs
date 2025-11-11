//! Quanto option pricers (Monte Carlo and analytical).

// Common imports for all pricers
use crate::instruments::common::traits::Instrument;
use crate::instruments::quanto_option::types::QuantoOption;
use crate::pricer::{InstrumentType, ModelKey, Pricer, PricerKey, PricingError, PricingResult};
use crate::results::ValuationResult;
use finstack_core::dates::{Date, DayCountCtx};
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;

// MC-specific imports
#[cfg(feature = "mc")]
use crate::instruments::common::mc::process::gbm::{GbmParams, GbmProcess};
#[cfg(feature = "mc")]
use crate::instruments::common::models::monte_carlo::payoff::quanto::{
    QuantoCallPayoff, QuantoPutPayoff,
};
#[cfg(feature = "mc")]
use crate::instruments::common::models::monte_carlo::pricer::european::{
    EuropeanPricer, EuropeanPricerConfig,
};

/// Quanto option Monte Carlo pricer.
#[cfg(feature = "mc")]
pub struct QuantoOptionMcPricer {
    config: EuropeanPricerConfig,
}

#[cfg(feature = "mc")]
impl QuantoOptionMcPricer {
    /// Create a new quanto option MC pricer with default config.
    pub fn new() -> Self {
        Self {
            config: EuropeanPricerConfig::default(),
        }
    }

    /// Price a quanto option using Monte Carlo.
    fn price_internal(
        &self,
        inst: &QuantoOption,
        curves: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        let t = inst
            .day_count
            .year_fraction(as_of, inst.expiry, DayCountCtx::default())?;
        if t <= 0.0 {
            return Ok(finstack_core::money::Money::new(
                0.0,
                inst.domestic_currency,
            ));
        }

        let disc_curve = curves.get_discount_ref(inst.discount_curve_id.as_str())?;
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

        // Get foreign rate (could be different curve)
        let r_for = r_dom; // Simplified: assume same for now

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

        let vol_surface = curves.surface_ref(inst.vol_surface_id.as_str())?;
        let sigma_equity = vol_surface.value_clamped(t, inst.equity_strike.amount());

        // Get FX volatility
        let sigma_fx = if let Some(fx_vol_id) = &inst.fx_vol_id {
            let fx_vol_surface = curves.surface_ref(fx_vol_id.as_str())?;
            fx_vol_surface.value_clamped(t, 1.0) // Use spot FX rate of 1.0 as reference
        } else {
            0.12 // Default FX vol if not provided
        };

        // Compute quanto adjustment
        let quanto_adjustment = QuantoCallPayoff::compute_quanto_adjustment(
            r_for,
            q,
            inst.correlation,
            sigma_equity,
            sigma_fx,
        );

        // Create GBM process with quanto-adjusted drift
        let adjusted_drift = r_for - q - quanto_adjustment + r_for;
        let gbm_params = GbmParams::new(adjusted_drift, q, sigma_equity);
        let process = GbmProcess::new(gbm_params);

        let steps_per_year = 252.0;
        let num_steps = ((t * steps_per_year).round() as usize).max(8);

        let result = match inst.option_type {
            crate::instruments::OptionType::Call => {
                let payoff = QuantoCallPayoff::new(
                    inst.equity_strike.amount(),
                    inst.notional.amount(),
                    inst.domestic_currency,
                    inst.foreign_currency,
                    quanto_adjustment,
                );
                let pricer = EuropeanPricer::new(self.config.clone());
                pricer.price(
                    &process,
                    spot,
                    t,
                    num_steps,
                    &payoff,
                    inst.domestic_currency,
                    discount_factor,
                )?
            }
            crate::instruments::OptionType::Put => {
                let payoff = QuantoPutPayoff::new(
                    inst.equity_strike.amount(),
                    inst.notional.amount(),
                    inst.domestic_currency,
                    inst.foreign_currency,
                    quanto_adjustment,
                );
                let pricer = EuropeanPricer::new(self.config.clone());
                pricer.price(
                    &process,
                    spot,
                    t,
                    num_steps,
                    &payoff,
                    inst.domestic_currency,
                    discount_factor,
                )?
            }
        };

        Ok(result.mean)
    }
}

#[cfg(feature = "mc")]
impl Default for QuantoOptionMcPricer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "mc")]
impl Pricer for QuantoOptionMcPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::QuantoOption, ModelKey::MonteCarloGBM)
    }

    fn price_dyn(
        &self,
        instrument: &dyn crate::instruments::common::traits::Instrument,
        market: &MarketContext,
        as_of: Date,
    ) -> PricingResult<ValuationResult> {
        let quanto = instrument
            .as_any()
            .downcast_ref::<QuantoOption>()
            .ok_or_else(|| {
                PricingError::type_mismatch(InstrumentType::QuantoOption, instrument.key())
            })?;

        let pv = self
            .price_internal(quanto, market, as_of)
            .map_err(|e| PricingError::model_failure(e.to_string()))?;

        Ok(ValuationResult::stamped(quanto.id(), as_of, pv))
    }
}

/// Present value using Monte Carlo.
#[cfg(feature = "mc")]
pub fn npv(inst: &QuantoOption, curves: &MarketContext, as_of: Date) -> finstack_core::Result<Money> {
    let pricer = QuantoOptionMcPricer::new();
    pricer.price_internal(inst, curves, as_of)
}

// ========================= ANALYTICAL PRICER =========================

use crate::instruments::common::models::closed_form::quanto::{quanto_call, quanto_put};

/// Helper to collect inputs for quanto option pricing.
fn collect_quanto_inputs(
    inst: &QuantoOption,
    curves: &MarketContext,
    as_of: Date,
) -> finstack_core::Result<(f64, f64, f64, f64, f64, f64, f64)> {
    let t = inst
        .day_count
        .year_fraction(as_of, inst.expiry, DayCountCtx::default())?;

    let disc_curve = curves.get_discount_ref(inst.discount_curve_id.as_str())?;
    let r_dom = disc_curve.zero(t);

    // For simplicity, use same rate curve for foreign rate
    // In production, would fetch from separate curve
    let r_for = r_dom;

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

    let vol_surface = curves.surface_ref(inst.vol_surface_id.as_str())?;
    let sigma_equity = vol_surface.value_clamped(t, inst.equity_strike.amount());

    // Get FX volatility
    let sigma_fx = if let Some(fx_vol_id) = &inst.fx_vol_id {
        let fx_vol_surface = curves.surface_ref(fx_vol_id.as_str())?;
        fx_vol_surface.value_clamped(t, 1.0)
    } else {
        0.12 // Default FX vol
    };

    Ok((spot, r_dom, r_for, q, sigma_equity, sigma_fx, t))
}

/// Quanto option analytical pricer.
pub struct QuantoOptionAnalyticalPricer;

impl QuantoOptionAnalyticalPricer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for QuantoOptionAnalyticalPricer {
    fn default() -> Self {
        Self::new()
    }
}

impl Pricer for QuantoOptionAnalyticalPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::QuantoOption, ModelKey::QuantoBS)
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
        as_of: Date,
    ) -> PricingResult<ValuationResult> {
        let quanto = instrument
            .as_any()
            .downcast_ref::<QuantoOption>()
            .ok_or_else(|| {
                PricingError::type_mismatch(InstrumentType::QuantoOption, instrument.key())
            })?;

        let (spot, r_dom, r_for, q, sigma_equity, sigma_fx, t) =
            collect_quanto_inputs(quanto, market, as_of)
                .map_err(|e| PricingError::model_failure(e.to_string()))?;

        if t <= 0.0 {
            return Ok(ValuationResult::stamped(
                quanto.id(),
                as_of,
                Money::new(0.0, quanto.domestic_currency),
            ));
        }

        let price = match quanto.option_type {
            crate::instruments::OptionType::Call => quanto_call(
                spot,
                quanto.equity_strike.amount(),
                t,
                r_dom,
                r_for,
                q,
                sigma_equity,
                sigma_fx,
                quanto.correlation,
            ),
            crate::instruments::OptionType::Put => quanto_put(
                spot,
                quanto.equity_strike.amount(),
                t,
                r_dom,
                r_for,
                q,
                sigma_equity,
                sigma_fx,
                quanto.correlation,
            ),
        };

        let pv = Money::new(price * quanto.notional.amount(), quanto.domestic_currency);
        Ok(ValuationResult::stamped(quanto.id(), as_of, pv))
    }
}
