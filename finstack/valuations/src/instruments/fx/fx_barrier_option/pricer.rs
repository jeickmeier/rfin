//! FX barrier option pricers (Monte Carlo and analytical).

// Common imports for all pricers
use crate::instruments::common_impl::traits::Instrument;
use crate::instruments::fx::fx_barrier_option::types::FxBarrierOption;
use crate::pricer::{
    InstrumentType, ModelKey, Pricer, PricerKey, PricingError, PricingErrorContext, PricingResult,
};
use crate::results::ValuationResult;
use finstack_core::dates::{Date, DayCountCtx};
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;

// MC-specific imports
#[cfg(feature = "mc")]
use crate::instruments::common_impl::mc::process::gbm::{GbmParams, GbmProcess};
#[cfg(feature = "mc")]
use crate::instruments::common_impl::models::monte_carlo::payoff::barrier::BarrierType as McBarrierType;
#[cfg(feature = "mc")]
use crate::instruments::common_impl::models::monte_carlo::payoff::fx_barrier::FxBarrierCall;
#[cfg(feature = "mc")]
use crate::instruments::common_impl::models::monte_carlo::pricer::path_dependent::{
    PathDependentPricer, PathDependentPricerConfig,
};

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

    /// Price an FX barrier option using Monte Carlo.
    fn price_internal(
        &self,
        inst: &FxBarrierOption,
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

        // Domestic curve (discounting)
        let disc_curve = curves.get_discount(inst.domestic_discount_curve_id.as_str())?;
        let r_dom = disc_curve.zero(t);
        let discount_factor = disc_curve.df_between_dates(as_of, inst.expiry)?;

        // Foreign curve (risk-free rate for drift)
        let for_curve = curves.get_discount(inst.foreign_discount_curve_id.as_str())?;
        let r_for = for_curve.zero(t);

        let spot_scalar = curves.price(&inst.fx_spot_id)?;
        let fx_spot = match spot_scalar {
            finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
            finstack_core::market_data::scalars::MarketScalar::Price(m) => m.amount(),
        };

        let vol_surface = curves.surface(inst.fx_vol_id.as_str())?;
        let sigma = vol_surface.value_clamped(t, inst.strike.amount());

        // For FX, drift is r_dom - r_for.
        // In GBM process param 'q' is subtracted from r to get drift (r-q).
        // So q should be r_for.
        let q = r_for;
        let gbm_params = GbmParams::new(r_dom, q, sigma);
        let process = GbmProcess::new(gbm_params);

        let steps_per_year = self.config.steps_per_year;
        let num_steps = ((t * steps_per_year).round() as usize).max(self.config.min_steps);
        let dt = t / num_steps as f64;
        let maturity_step = num_steps - 1;

        // Quanto adjustment removed as standard FX barriers don't need it.
        // If Quanto is needed, it should be explicit.
        let quanto_adjustment = 0.0;

        let mc_barrier_type = Self::convert_barrier_type(inst.barrier_type);
        let payoff = FxBarrierCall::new(
            inst.strike.amount(),
            inst.barrier.amount(),
            mc_barrier_type,
            inst.notional.amount(),
            maturity_step,
            sigma,
            dt,
            inst.use_gobet_miri,
            inst.domestic_currency,
            inst.foreign_currency,
            quanto_adjustment,
            inst.rebate.map(|m| m.amount()),
        );

        // Derive deterministic seed from instrument ID and scenario
        #[cfg(feature = "mc")]
        use crate::instruments::common_impl::models::monte_carlo::seed;

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
        instrument: &dyn crate::instruments::common_impl::traits::Instrument,
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
            .map_err(|e| {
                PricingError::model_failure_with_context(
                    e.to_string(),
                    PricingErrorContext::default(),
                )
            })?;

        Ok(ValuationResult::stamped(fx_barrier.id(), as_of, pv))
    }
}

/// Present value using Monte Carlo.
#[cfg(feature = "mc")]
pub(crate) fn compute_pv(
    inst: &FxBarrierOption,
    curves: &MarketContext,
    as_of: Date,
) -> finstack_core::Result<Money> {
    let pricer = FxBarrierOptionMcPricer::new();
    pricer.price_internal(inst, curves, as_of)
}

// ========================= ANALYTICAL PRICER =========================

use crate::instruments::common_impl::models::closed_form::barrier::{
    barrier_call_continuous, barrier_put_continuous, barrier_rebate_continuous,
    BarrierType as AnalyticalBarrierType,
};

/// Validate currency semantics for FX barrier option.
///
/// # Currency Conventions
///
/// For an FX barrier option on `foreign_currency/domestic_currency` (e.g., EUR/USD):
/// - Strike and barrier are expressed in domestic currency (quote currency)
/// - Notional is in foreign currency (base currency) - the amount of foreign currency
///   being bought/sold
fn validate_fx_barrier_currencies(inst: &FxBarrierOption) -> finstack_core::Result<()> {
    // Strike should be in domestic currency
    if inst.strike.currency() != inst.domestic_currency {
        return Err(finstack_core::Error::CurrencyMismatch {
            expected: inst.domestic_currency,
            actual: inst.strike.currency(),
        });
    }

    // Barrier should be in domestic currency
    if inst.barrier.currency() != inst.domestic_currency {
        return Err(finstack_core::Error::CurrencyMismatch {
            expected: inst.domestic_currency,
            actual: inst.barrier.currency(),
        });
    }

    // Notional should be in foreign currency
    if inst.notional.currency() != inst.foreign_currency {
        return Err(finstack_core::Error::CurrencyMismatch {
            expected: inst.foreign_currency,
            actual: inst.notional.currency(),
        });
    }

    // Rebate, if present, should be in domestic currency
    if let Some(ref rebate) = inst.rebate {
        if rebate.currency() != inst.domestic_currency {
            return Err(finstack_core::Error::CurrencyMismatch {
                expected: inst.domestic_currency,
                actual: rebate.currency(),
            });
        }
    }

    Ok(())
}

/// Helper to collect inputs for FX barrier option pricing.
fn collect_fx_barrier_inputs(
    inst: &FxBarrierOption,
    curves: &MarketContext,
    as_of: Date,
) -> finstack_core::Result<(f64, f64, f64, f64, f64)> {
    // Validate currency semantics first
    validate_fx_barrier_currencies(inst)?;

    // Vol surface time using instrument day count (typically ACT/365F for FX options)
    let t = inst
        .day_count
        .year_fraction(as_of, inst.expiry, DayCountCtx::default())?;

    // Use each curve's own day count for discount factor lookup (consistent
    // with FxOptionCalculator::collect_inputs), then convert to effective
    // zero rates consistent with t_vol so that exp(-r * t) = df.
    let disc_curve = curves.get_discount(inst.domestic_discount_curve_id.as_str())?;
    let t_disc_dom =
        disc_curve
            .day_count()
            .year_fraction(as_of, inst.expiry, DayCountCtx::default())?;
    let df_d = disc_curve.df(t_disc_dom);
    let r_dom = if t > 0.0 { -df_d.ln() / t } else { 0.0 };

    let for_curve = curves.get_discount(inst.foreign_discount_curve_id.as_str())?;
    let t_disc_for =
        for_curve
            .day_count()
            .year_fraction(as_of, inst.expiry, DayCountCtx::default())?;
    let df_f = for_curve.df(t_disc_for);
    let r_for = if t > 0.0 { -df_f.ln() / t } else { 0.0 };

    let spot_scalar = curves.price(&inst.fx_spot_id)?;
    let fx_spot = match spot_scalar {
        finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
        finstack_core::market_data::scalars::MarketScalar::Price(m) => m.amount(),
    };

    let vol_surface = curves.surface(inst.fx_vol_id.as_str())?;
    let sigma = vol_surface.value_clamped(t, inst.strike.amount());

    Ok((fx_spot, r_dom, r_for, sigma, t))
}

/// FX Barrier option analytical pricer (continuous monitoring).
pub struct FxBarrierOptionAnalyticalPricer;

impl FxBarrierOptionAnalyticalPricer {
    /// Create a new analytical FX barrier option pricer
    pub fn new() -> Self {
        Self
    }
}

impl Default for FxBarrierOptionAnalyticalPricer {
    fn default() -> Self {
        Self::new()
    }
}

/// Map from the instrument's BarrierType to the analytical BarrierType.
fn map_barrier_type(
    bt: crate::instruments::exotics::barrier_option::types::BarrierType,
) -> AnalyticalBarrierType {
    use crate::instruments::exotics::barrier_option::types::BarrierType;
    match bt {
        BarrierType::UpAndIn => AnalyticalBarrierType::UpIn,
        BarrierType::UpAndOut => AnalyticalBarrierType::UpOut,
        BarrierType::DownAndIn => AnalyticalBarrierType::DownIn,
        BarrierType::DownAndOut => AnalyticalBarrierType::DownOut,
    }
}

/// Compute the BS barrier price + optional rebate (without notional scaling).
fn bs_barrier_price_per_unit(
    fx_barrier: &FxBarrierOption,
    fx_spot: f64,
    r_dom: f64,
    r_for: f64,
    sigma: f64,
    t: f64,
    analytical_barrier_type: AnalyticalBarrierType,
) -> f64 {
    let price = match fx_barrier.option_type {
        crate::instruments::OptionType::Call => barrier_call_continuous(
            fx_spot,
            fx_barrier.strike.amount(),
            fx_barrier.barrier.amount(),
            t,
            r_dom,
            r_for,
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

    let rebate_val = if let Some(rebate) = fx_barrier.rebate {
        barrier_rebate_continuous(
            fx_spot,
            fx_barrier.barrier.amount(),
            rebate.amount(),
            t,
            r_dom,
            r_for,
            sigma,
            analytical_barrier_type,
        )
    } else {
        0.0
    };

    price + rebate_val
}

impl Pricer for FxBarrierOptionAnalyticalPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(
            InstrumentType::FxBarrierOption,
            ModelKey::FxBarrierBSContinuous,
        )
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

        let (fx_spot, r_dom, r_for, sigma, t) =
            collect_fx_barrier_inputs(fx_barrier, market, as_of).map_err(|e| {
                PricingError::model_failure_with_context(
                    e.to_string(),
                    PricingErrorContext::default(),
                )
            })?;

        if t <= 0.0 {
            return Ok(ValuationResult::stamped(
                fx_barrier.id(),
                as_of,
                Money::new(0.0, fx_barrier.domestic_currency),
            ));
        }

        let analytical_barrier_type = map_barrier_type(fx_barrier.barrier_type);

        let price_per_unit = bs_barrier_price_per_unit(
            fx_barrier,
            fx_spot,
            r_dom,
            r_for,
            sigma,
            t,
            analytical_barrier_type,
        );

        let pv = Money::new(
            price_per_unit * fx_barrier.notional.amount(),
            fx_barrier.domestic_currency,
        );
        Ok(ValuationResult::stamped(fx_barrier.id(), as_of, pv))
    }
}

// ========================= VANNA-VOLGA PRICER =========================

use crate::instruments::fx::fx_barrier_option::vanna_volga::{
    vanna_volga_barrier_adjustment, VannaVolgaQuotes,
};

/// FX Barrier option Vanna-Volga pricer (continuous monitoring with smile correction).
///
/// Applies the Vanna-Volga method to adjust the analytical BS barrier price for
/// smile effects, using three market pillar volatilities (25Δ put, ATM, 25Δ call).
#[allow(dead_code)]
pub struct FxBarrierOptionVannaVolgaPricer {
    /// Market quotes for the three-point smile
    pub quotes: VannaVolgaQuotes,
}

#[allow(dead_code)]
impl FxBarrierOptionVannaVolgaPricer {
    /// Create a new Vanna-Volga pricer with the given market quotes.
    pub fn new(quotes: VannaVolgaQuotes) -> Self {
        Self { quotes }
    }

    /// Price an FX barrier option with Vanna-Volga smile adjustment.
    pub fn price_with_vv(
        &self,
        fx_barrier: &FxBarrierOption,
        market: &MarketContext,
        as_of: Date,
    ) -> PricingResult<Money> {
        let (fx_spot, r_dom, r_for, sigma, t) =
            collect_fx_barrier_inputs(fx_barrier, market, as_of).map_err(|e| {
                PricingError::model_failure_with_context(
                    e.to_string(),
                    PricingErrorContext::default(),
                )
            })?;

        if t <= 0.0 {
            return Ok(Money::new(0.0, fx_barrier.domestic_currency));
        }

        let analytical_barrier_type = map_barrier_type(fx_barrier.barrier_type);
        let is_call = matches!(fx_barrier.option_type, crate::instruments::OptionType::Call);

        // Compute BS barrier price at ATM vol
        let bs_price = bs_barrier_price_per_unit(
            fx_barrier,
            fx_spot,
            r_dom,
            r_for,
            sigma,
            t,
            analytical_barrier_type,
        );

        // Apply Vanna-Volga correction
        let vv_price = vanna_volga_barrier_adjustment(
            bs_price,
            fx_spot,
            fx_barrier.barrier.amount(),
            fx_barrier.strike.amount(),
            r_dom,
            r_for,
            t,
            &self.quotes,
            is_call,
            analytical_barrier_type,
        );

        Ok(Money::new(
            vv_price * fx_barrier.notional.amount(),
            fx_barrier.domestic_currency,
        ))
    }
}
