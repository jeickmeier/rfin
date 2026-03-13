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
use finstack_core::money::fx::FxQuery;
use finstack_core::money::Money;

// MC-specific imports
#[cfg(feature = "mc")]
use crate::instruments::common_impl::models::monte_carlo::payoff::barrier::BarrierType as McBarrierType;
#[cfg(feature = "mc")]
use crate::instruments::common_impl::models::monte_carlo::payoff::fx_barrier::FxBarrierCall;
#[cfg(feature = "mc")]
use crate::instruments::common_impl::models::monte_carlo::pricer::path_dependent::{
    PathDependentPricer, PathDependentPricerConfig,
};
#[cfg(feature = "mc")]
use crate::instruments::common_impl::models::monte_carlo::process::gbm::{GbmParams, GbmProcess};

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
        validate_fx_barrier_currencies(inst)?;

        let fx_spot = resolve_fx_spot(inst, curves, as_of)?;

        let t = inst
            .day_count
            .year_fraction(as_of, inst.expiry, DayCountCtx::default())?;
        if t <= 0.0 {
            let per_unit = expired_barrier_value_per_unit(inst, fx_spot)?;
            return Ok(finstack_core::money::Money::new(
                per_unit * inst.notional.amount(),
                inst.quote_currency,
            ));
        }

        // Domestic curve (discounting)
        let disc_curve = curves.get_discount(inst.domestic_discount_curve_id.as_str())?;
        let discount_factor = disc_curve.df_between_dates(as_of, inst.expiry)?;
        let r_dom = if t > 0.0 && discount_factor > 0.0 {
            -discount_factor.ln() / t
        } else {
            0.0
        };

        // Foreign curve (risk-free rate for drift)
        let for_curve = curves.get_discount(inst.foreign_discount_curve_id.as_str())?;
        let df_for = for_curve.df_between_dates(as_of, inst.expiry)?;
        let r_for = if t > 0.0 && df_for > 0.0 {
            -df_for.ln() / t
        } else {
            0.0
        };

        let vol_surface = curves.get_surface(inst.vol_surface_id.as_str())?;
        let sigma = vol_surface.value_clamped(t, inst.strike);

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
            inst.strike,
            inst.barrier,
            mc_barrier_type,
            inst.notional.amount(),
            maturity_step,
            sigma,
            dt,
            inst.use_gobet_miri,
            inst.base_currency,
            inst.quote_currency,
            quanto_adjustment,
            inst.rebate,
        );

        // Derive deterministic seed from instrument ID and scenario
        #[cfg(feature = "mc")]
        use crate::instruments::common_impl::models::monte_carlo::seed;

        let seed = if let Some(ref scenario) = inst.pricing_overrides.scenario.mc_seed_scenario {
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
            inst.quote_currency,
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

#[inline]
fn barrier_is_knock_in(
    bt: crate::instruments::exotics::barrier_option::types::BarrierType,
) -> bool {
    matches!(
        bt,
        crate::instruments::exotics::barrier_option::types::BarrierType::UpAndIn
            | crate::instruments::exotics::barrier_option::types::BarrierType::DownAndIn
    )
}

fn expired_barrier_value_per_unit(inst: &FxBarrierOption, spot: f64) -> finstack_core::Result<f64> {
    let strike = inst.strike;
    let is_knock_in = barrier_is_knock_in(inst.barrier_type);
    let barrier_hit = inst.observed_barrier_breached.ok_or_else(|| {
        finstack_core::Error::Validation(
            "Expired FX barrier option requires `observed_barrier_breached` to determine realized payoff"
                .to_string(),
        )
    })?;
    let activated = if is_knock_in {
        barrier_hit
    } else {
        !barrier_hit
    };

    let intrinsic = if activated {
        match inst.option_type {
            crate::instruments::OptionType::Call => (spot - strike).max(0.0),
            crate::instruments::OptionType::Put => (strike - spot).max(0.0),
        }
    } else {
        0.0
    };

    let rebate_due = if is_knock_in {
        !barrier_hit
    } else {
        barrier_hit
    };
    let rebate = if rebate_due {
        inst.rebate.unwrap_or(0.0)
    } else {
        0.0
    };

    Ok(intrinsic + rebate)
}

/// Validate currency semantics and numeric bounds for FX barrier option.
///
/// # Currency Conventions
///
/// For an FX barrier option on `foreign_currency/domestic_currency` (e.g., EUR/USD):
/// - Strike and barrier are dimensionless exchange rates (f64)
/// - Notional is in foreign currency (base currency) - the amount of foreign currency
///   being bought/sold
fn validate_fx_barrier_currencies(inst: &FxBarrierOption) -> finstack_core::Result<()> {
    // Notional should be in foreign currency
    if inst.notional.currency() != inst.base_currency {
        return Err(finstack_core::Error::CurrencyMismatch {
            expected: inst.base_currency,
            actual: inst.notional.currency(),
        });
    }

    let strike = inst.strike;
    if !strike.is_finite() || strike <= 0.0 {
        return Err(finstack_core::Error::Validation(format!(
            "FxBarrierOption strike must be finite and > 0, got {}",
            strike
        )));
    }
    let barrier = inst.barrier;
    if !barrier.is_finite() || barrier <= 0.0 {
        return Err(finstack_core::Error::Validation(format!(
            "FxBarrierOption barrier must be finite and > 0, got {}",
            barrier
        )));
    }
    let notional = inst.notional.amount();
    if !notional.is_finite() || notional <= 0.0 {
        return Err(finstack_core::Error::Validation(format!(
            "FxBarrierOption notional must be finite and > 0, got {}",
            notional
        )));
    }

    Ok(())
}

fn resolve_fx_spot(
    inst: &FxBarrierOption,
    curves: &MarketContext,
    as_of: Date,
) -> finstack_core::Result<f64> {
    if let Some(spot_id) = inst.fx_spot_id.as_ref() {
        let spot_scalar = curves.get_price(spot_id)?;
        let fx_spot = match spot_scalar {
            finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
            finstack_core::market_data::scalars::MarketScalar::Price(m) => m.amount(),
        };
        if !fx_spot.is_finite() || fx_spot <= 0.0 {
            return Err(finstack_core::Error::Validation(format!(
                "FxBarrierOption spot must be finite and > 0, got {}",
                fx_spot
            )));
        }
        return Ok(fx_spot);
    }

    let fx_matrix = curves.fx().ok_or_else(|| {
        finstack_core::Error::from(finstack_core::InputError::NotFound {
            id: "fx_matrix".to_string(),
        })
    })?;
    let fx_spot = fx_matrix
        .rate(FxQuery::new(inst.base_currency, inst.quote_currency, as_of))?
        .rate;
    if !fx_spot.is_finite() || fx_spot <= 0.0 {
        return Err(finstack_core::Error::Validation(format!(
            "FxBarrierOption spot must be finite and > 0, got {}",
            fx_spot
        )));
    }
    Ok(fx_spot)
}

fn collect_fx_barrier_expiry_state(
    inst: &FxBarrierOption,
    curves: &MarketContext,
    as_of: Date,
) -> finstack_core::Result<(f64, f64)> {
    validate_fx_barrier_currencies(inst)?;
    let t = inst
        .day_count
        .year_fraction(as_of, inst.expiry, DayCountCtx::default())?;
    let fx_spot = resolve_fx_spot(inst, curves, as_of)?;
    Ok((fx_spot, t))
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

    let fx_spot = resolve_fx_spot(inst, curves, as_of)?;

    let vol_surface = curves.get_surface(inst.vol_surface_id.as_str())?;
    let sigma = vol_surface.value_clamped(t, inst.strike);
    if !sigma.is_finite() || sigma < 0.0 {
        return Err(finstack_core::Error::Validation(format!(
            "FxBarrierOption volatility must be finite and non-negative, got {}",
            sigma
        )));
    }

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
            fx_barrier.strike,
            fx_barrier.barrier,
            t,
            r_dom,
            r_for,
            sigma,
            analytical_barrier_type,
        ),
        crate::instruments::OptionType::Put => barrier_put_continuous(
            fx_spot,
            fx_barrier.strike,
            fx_barrier.barrier,
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
            fx_barrier.barrier,
            rebate,
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

        if fx_barrier.use_gobet_miri {
            tracing::warn!(
                "Analytical barrier pricer uses continuous monitoring; discrete monitoring flag \
                 is ignored. Use Monte Carlo pricer for discrete barrier monitoring."
            );
        }

        let (fx_spot, t) =
            collect_fx_barrier_expiry_state(fx_barrier, market, as_of).map_err(|e| {
                PricingError::model_failure_with_context(
                    e.to_string(),
                    PricingErrorContext::default(),
                )
            })?;

        if t <= 0.0 {
            let per_unit = expired_barrier_value_per_unit(fx_barrier, fx_spot).map_err(|e| {
                PricingError::model_failure_with_context(
                    e.to_string(),
                    PricingErrorContext::default(),
                )
            })?;
            return Ok(ValuationResult::stamped(
                fx_barrier.id(),
                as_of,
                Money::new(
                    per_unit * fx_barrier.notional.amount(),
                    fx_barrier.quote_currency,
                ),
            ));
        }

        let (_, r_dom, r_for, sigma, _) = collect_fx_barrier_inputs(fx_barrier, market, as_of)
            .map_err(|e| {
                PricingError::model_failure_with_context(
                    e.to_string(),
                    PricingErrorContext::default(),
                )
            })?;

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
            fx_barrier.quote_currency,
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
        if fx_barrier.use_gobet_miri {
            tracing::warn!(
                "Analytical barrier pricer uses continuous monitoring; discrete monitoring flag \
                 is ignored. Use Monte Carlo pricer for discrete barrier monitoring."
            );
        }

        let (fx_spot, t) =
            collect_fx_barrier_expiry_state(fx_barrier, market, as_of).map_err(|e| {
                PricingError::model_failure_with_context(
                    e.to_string(),
                    PricingErrorContext::default(),
                )
            })?;

        if t <= 0.0 {
            let per_unit = expired_barrier_value_per_unit(fx_barrier, fx_spot).map_err(|e| {
                PricingError::model_failure_with_context(
                    e.to_string(),
                    PricingErrorContext::default(),
                )
            })?;
            return Ok(Money::new(
                per_unit * fx_barrier.notional.amount(),
                fx_barrier.quote_currency,
            ));
        }

        let (_, r_dom, r_for, sigma, _) = collect_fx_barrier_inputs(fx_barrier, market, as_of)
            .map_err(|e| {
                PricingError::model_failure_with_context(
                    e.to_string(),
                    PricingErrorContext::default(),
                )
            })?;

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
            fx_barrier.barrier,
            fx_barrier.strike,
            r_dom,
            r_for,
            t,
            &self.quotes,
            is_call,
            analytical_barrier_type,
        );

        Ok(Money::new(
            vv_price * fx_barrier.notional.amount(),
            fx_barrier.quote_currency,
        ))
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::instruments::exotics::barrier_option::types::BarrierType;
    use crate::instruments::Instrument;
    use crate::instruments::OptionType;
    use finstack_core::currency::Currency;
    use finstack_core::dates::Date;
    use finstack_core::market_data::context::MarketContext;
    use finstack_core::market_data::scalars::MarketScalar;
    use finstack_core::market_data::surfaces::VolSurface;
    use finstack_core::market_data::term_structures::DiscountCurve;
    use finstack_core::money::Money;
    use time::Month;

    #[test]
    fn expired_up_and_in_call_returns_intrinsic_when_hit() {
        let mut inst = FxBarrierOption::example();
        inst.option_type = OptionType::Call;
        inst.barrier_type = BarrierType::UpAndIn;
        inst.strike = 1.10;
        inst.barrier = 1.20;
        inst.rebate = None;
        inst.observed_barrier_breached = Some(true);

        let per_unit = expired_barrier_value_per_unit(&inst, 1.25).expect("expired value");
        assert!((per_unit - 0.15).abs() < 1e-12);
    }

    #[test]
    fn expired_down_and_out_put_returns_intrinsic_when_not_hit() {
        let mut inst = FxBarrierOption::example();
        inst.option_type = OptionType::Put;
        inst.barrier_type = BarrierType::DownAndOut;
        inst.strike = 1.10;
        inst.barrier = 0.90;
        inst.rebate = None;
        inst.observed_barrier_breached = Some(false);

        // Barrier not hit at expiry => down-and-out stays active => intrinsic applies.
        let per_unit = expired_barrier_value_per_unit(&inst, 1.00).expect("expired value");
        assert!((per_unit - 0.10).abs() < 1e-12);
    }

    #[test]
    fn expired_up_and_out_with_hit_pays_rebate_only() {
        let mut inst = FxBarrierOption::example();
        inst.option_type = OptionType::Call;
        inst.barrier_type = BarrierType::UpAndOut;
        inst.strike = 1.10;
        inst.barrier = 1.20;
        inst.rebate = Some(0.02);
        inst.observed_barrier_breached = Some(true);

        // Barrier hit at expiry => knocked out. With rebate, no intrinsic and rebate paid.
        let per_unit = expired_barrier_value_per_unit(&inst, 1.25).expect("expired value");
        assert!((per_unit - 0.02).abs() < 1e-12);
    }

    #[test]
    fn expired_up_and_in_with_no_hit_pays_rebate_only() {
        let mut inst = FxBarrierOption::example();
        inst.option_type = OptionType::Call;
        inst.barrier_type = BarrierType::UpAndIn;
        inst.strike = 1.10;
        inst.barrier = 1.20;
        inst.rebate = Some(0.02);
        inst.observed_barrier_breached = Some(false);

        let per_unit = expired_barrier_value_per_unit(&inst, 1.25).expect("expired value");
        assert!((per_unit - 0.02).abs() < 1e-12);
    }

    #[test]
    fn expired_fx_barrier_requires_observed_state() {
        let mut inst = FxBarrierOption::example();
        inst.observed_barrier_breached = None;

        let err = expired_barrier_value_per_unit(&inst, 1.25).expect_err("missing observed state");
        assert!(
            err.to_string().contains("observed_barrier_breached"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn validation_allows_barrier_equal_to_strike() {
        let mut inst = FxBarrierOption::example();
        inst.strike = 1.10;
        inst.barrier = 1.10;

        validate_fx_barrier_currencies(&inst).expect("equal strike/barrier should remain valid");
    }

    #[test]
    fn expired_analytical_value_only_requires_observed_state_and_spot() {
        let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");

        let mut option = FxBarrierOption::example();
        option.expiry = as_of;
        option.use_gobet_miri = false;
        option.option_type = OptionType::Call;
        option.barrier_type = BarrierType::UpAndIn;
        option.rebate = Some(0.02);
        option.observed_barrier_breached = Some(false);

        let market = MarketContext::new().insert_price("EURUSD-SPOT", MarketScalar::Unitless(1.25));

        let pv = option
            .value(&market, as_of)
            .expect("expired analytical value");
        assert!(
            (pv.amount() - 20_000.0).abs() < 1e-8,
            "expired FX barrier should settle from observed state and spot only, got {}",
            pv.amount()
        );
    }

    #[test]
    fn analytical_pricer_handles_zero_vol_knock_in_rebate_end_to_end() {
        let as_of = Date::from_calendar_date(2024, Month::January, 1).expect("valid date");
        let expiry = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");

        let option = FxBarrierOption::builder()
            .id("FXBAR-ZERO-VOL-UPIN".into())
            .strike(1.10)
            .barrier(1.20)
            .rebate(0.02)
            .option_type(OptionType::Call)
            .barrier_type(BarrierType::UpAndIn)
            .expiry(expiry)
            .notional(Money::new(1_000_000.0, Currency::EUR))
            .base_currency(Currency::EUR)
            .quote_currency(Currency::USD)
            .day_count(finstack_core::dates::DayCount::Act365F)
            .use_gobet_miri(false)
            .domestic_discount_curve_id("USD-OIS".into())
            .foreign_discount_curve_id("EUR-OIS".into())
            .fx_spot_id_opt(Some("EURUSD-SPOT".into()))
            .vol_surface_id("EURUSD-VOL".into())
            .pricing_overrides(crate::instruments::PricingOverrides::default())
            .attributes(crate::instruments::Attributes::new())
            .build()
            .expect("fx barrier option");

        let market = MarketContext::new()
            .insert(
                DiscountCurve::builder("USD-OIS")
                    .base_date(as_of)
                    .knots([(0.0, 1.0), (1.0, 1.0)])
                    .build()
                    .expect("dom curve"),
            )
            .insert(
                DiscountCurve::builder("EUR-OIS")
                    .base_date(as_of)
                    .knots([(0.0, 1.0), (1.0, 1.0)])
                    .build()
                    .expect("for curve"),
            )
            .insert_surface(
                VolSurface::builder("EURUSD-VOL")
                    .expiries(&[0.25, 0.5, 1.0])
                    .strikes(&[1.0, 1.1, 1.2])
                    .row(&[0.0, 0.0, 0.0])
                    .row(&[0.0, 0.0, 0.0])
                    .row(&[0.0, 0.0, 0.0])
                    .build()
                    .expect("vol surface"),
            )
            .insert_price("EURUSD-SPOT", MarketScalar::Unitless(1.10));

        let pv = option.value(&market, as_of).expect("fx barrier pv");
        assert!(
            (pv.amount() - 20_000.0).abs() < 1e-8,
            "zero-vol no-hit knock-in rebate should settle at rebate * notional, got {}",
            pv.amount()
        );
        assert_eq!(pv.currency(), Currency::USD);
    }
}
