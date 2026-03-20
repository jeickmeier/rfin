//! Barrier option pricers (Monte Carlo and analytical).

// Common imports for all pricers
use crate::instruments::common_impl::traits::Instrument;
use crate::instruments::exotics::barrier_option::types::BarrierOption;
use crate::pricer::{
    InstrumentType, ModelKey, Pricer, PricerKey, PricingError, PricingErrorContext, PricingResult,
};
use crate::results::ValuationResult;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;

// DayCountCtx is only used by MC pricer
#[cfg(feature = "mc")]
use finstack_core::dates::DayCountCtx;

// MC-specific imports
#[cfg(feature = "mc")]
use finstack_monte_carlo::payoff::barrier::BarrierOptionPayoff;
#[cfg(feature = "mc")]
use finstack_monte_carlo::payoff::barrier::{BarrierType as McBarrierType, OptionKind};
#[cfg(feature = "mc")]
use finstack_monte_carlo::pricer::path_dependent::{
    PathDependentPricer, PathDependentPricerConfig,
};
#[cfg(feature = "mc")]
use finstack_monte_carlo::process::gbm::{GbmParams, GbmProcess};

/// Barrier option Monte Carlo pricer.
#[cfg(feature = "mc")]
pub struct BarrierOptionMcPricer {
    config: PathDependentPricerConfig,
}

#[cfg(feature = "mc")]
impl BarrierOptionMcPricer {
    /// Create a new barrier option MC pricer with default config.
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

    fn convert_option_kind(option_type: crate::instruments::OptionType) -> OptionKind {
        match option_type {
            crate::instruments::OptionType::Call => OptionKind::Call,
            crate::instruments::OptionType::Put => OptionKind::Put,
        }
    }

    /// Price a barrier option using Monte Carlo.
    ///
    /// # Day Count Convention Handling
    ///
    /// Uses separate day count bases for different purposes:
    /// - **Discounting**: Uses the discount curve's own day count for DF and zero rate calculations
    /// - **Volatility lookup**: Uses the instrument's day count (assumed to match vol surface calibration)
    /// - **Monte Carlo time grid**: Uses the vol surface time basis for proper barrier monitoring
    fn price_internal(
        &self,
        inst: &BarrierOption,
        curves: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        // Get discount curve
        let disc_curve = curves.get_discount(inst.discount_curve_id.as_str())?;

        // Time to maturity for vol surface (using instrument's day count, which should
        // match how the vol surface was calibrated)
        let t_vol = inst
            .day_count
            .year_fraction(as_of, inst.expiry, DayCountCtx::default())?;

        if t_vol <= 0.0 {
            return price_expired_barrier(inst, curves);
        }

        let discount_factor = disc_curve.df_between_dates(as_of, inst.expiry)?;
        // Keep drift consistent with date-based discounting for MC simulation.
        let r = if t_vol > 0.0 && discount_factor > 0.0 {
            -discount_factor.ln() / t_vol
        } else {
            0.0
        };

        // Get spot
        let spot_scalar = curves.get_price(&inst.spot_id)?;
        let spot = match spot_scalar {
            finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
            finstack_core::market_data::scalars::MarketScalar::Price(m) => m.amount(),
        };

        // Get dividend yield
        let q = crate::instruments::common_impl::helpers::resolve_optional_dividend_yield(
            curves,
            inst.div_yield_id.as_ref(),
        )?;

        // Get volatility using vol surface time basis
        let vol_surface = curves.get_surface(inst.vol_surface_id.as_str())?;
        let sigma = vol_surface.value_clamped(t_vol, inst.strike);

        // Create GBM process
        let gbm_params = GbmParams::new(r, q, sigma);
        let process = GbmProcess::new(gbm_params);

        // Create time grid with minimum-capped steps (using vol surface time basis for proper
        // barrier monitoring - this ensures time steps align with volatility assumptions)
        let steps_per_year = self.config.steps_per_year;
        let num_steps = ((t_vol * steps_per_year).round() as usize).max(self.config.min_steps);
        let maturity_step = num_steps - 1;
        let time_grid = finstack_monte_carlo::time_grid::TimeGrid::uniform(t_vol, num_steps)?;

        // Create payoff (using vol surface time for barrier adjustment calculations)
        let mc_barrier_type = Self::convert_barrier_type(inst.barrier_type);
        let payoff = BarrierOptionPayoff::new(
            inst.strike,
            inst.barrier.amount(),
            mc_barrier_type,
            Self::convert_option_kind(inst.option_type),
            inst.rebate.map(|m| m.amount()),
            inst.notional.amount(),
            maturity_step,
            sigma,
            &time_grid,
            inst.use_gobet_miri,
        );

        // Derive deterministic seed from instrument ID and scenario
        #[cfg(feature = "mc")]
        use finstack_monte_carlo::seed;

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

        // Create config with derived seed
        let mut config = self.config.clone();
        config.seed = seed;

        // Price using path-dependent pricer (using vol surface time basis for simulation)
        let pricer = PathDependentPricer::new(config);
        let result = pricer.price(
            &process,
            spot,
            t_vol,
            num_steps,
            &payoff,
            inst.notional.currency(),
            discount_factor,
        )?;

        Ok(result.mean)
    }

    /// Price with LRM Greeks (delta, vega) convenience for barrier options.
    ///
    /// Returns `(pv, Option<(delta, vega)>)` where the Greeks are from the
    /// Likelihood Ratio Method (LRM). Greeks are `None` if the option is expired.
    ///
    /// # Day Count Convention Handling
    ///
    /// Uses separate day count bases for different purposes:
    /// - **Discounting**: Uses the discount curve's own day count for DF and zero rate calculations
    /// - **Volatility lookup and MC simulation**: Uses the instrument's day count (assumed to match vol surface calibration)
    #[allow(dead_code)] // May be used by external bindings or tests
    pub(crate) fn price_with_lrm_greeks_internal(
        &self,
        inst: &BarrierOption,
        curves: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<(finstack_core::money::Money, Option<(f64, f64)>)> {
        // Get discount curve first to access its day count
        let disc_curve = curves.get_discount(inst.discount_curve_id.as_str())?;

        // Time to maturity for vol surface (using instrument's day count)
        let t_vol = inst
            .day_count
            .year_fraction(as_of, inst.expiry, DayCountCtx::default())?;
        if t_vol <= 0.0 {
            let pv = price_expired_barrier(inst, curves)?;
            return Ok((pv, None));
        }

        let discount_factor = disc_curve.df_between_dates(as_of, inst.expiry)?;
        // Keep drift consistent with date-based discounting for MC simulation.
        let r = if t_vol > 0.0 && discount_factor > 0.0 {
            -discount_factor.ln() / t_vol
        } else {
            0.0
        };

        // Spot and dividend yield
        let spot_scalar = curves.get_price(&inst.spot_id)?;
        let spot = match spot_scalar {
            finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
            finstack_core::market_data::scalars::MarketScalar::Price(m) => m.amount(),
        };
        let q = crate::instruments::common_impl::helpers::resolve_optional_dividend_yield(
            curves,
            inst.div_yield_id.as_ref(),
        )?;

        // Volatility and process (using vol surface time basis)
        let vol_surface = curves.get_surface(inst.vol_surface_id.as_str())?;
        let sigma = vol_surface.value_clamped(t_vol, inst.strike);
        let gbm_params = GbmParams::new(r, q, sigma);
        let process = GbmProcess::new(gbm_params);

        // Steps and payoff (using vol surface time basis)
        let steps_per_year = self.config.steps_per_year;
        let num_steps = ((t_vol * steps_per_year).round() as usize).max(self.config.min_steps);
        let maturity_step = num_steps - 1;
        let time_grid = finstack_monte_carlo::time_grid::TimeGrid::uniform(t_vol, num_steps)?;
        let mc_barrier_type = Self::convert_barrier_type(inst.barrier_type);
        let payoff = BarrierOptionPayoff::new(
            inst.strike,
            inst.barrier.amount(),
            mc_barrier_type,
            Self::convert_option_kind(inst.option_type),
            inst.rebate.map(|m| m.amount()),
            inst.notional.amount(),
            maturity_step,
            sigma,
            &time_grid,
            inst.use_gobet_miri,
        );

        // Seed
        #[cfg(feature = "mc")]
        use finstack_monte_carlo::seed;
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
        let mut cfg = self.config.clone();
        cfg.seed = seed;

        let pricer = PathDependentPricer::new(cfg);
        let (est, greeks) = pricer.price_with_lrm_greeks(
            &process,
            spot,
            t_vol,
            num_steps,
            &payoff,
            inst.notional.currency(),
            discount_factor,
            r,
            q,
            sigma,
        )?;

        Ok((est.mean, greeks))
    }
}

#[cfg(feature = "mc")]
impl Default for BarrierOptionMcPricer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "mc")]
impl Pricer for BarrierOptionMcPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::BarrierOption, ModelKey::MonteCarloGBM)
    }

    fn price_dyn(
        &self,
        instrument: &dyn crate::instruments::common_impl::traits::Instrument,
        market: &MarketContext,
        as_of: Date,
    ) -> PricingResult<ValuationResult> {
        let barrier = instrument
            .as_any()
            .downcast_ref::<BarrierOption>()
            .ok_or_else(|| {
                PricingError::type_mismatch(InstrumentType::BarrierOption, instrument.key())
            })?;

        let pv = self.price_internal(barrier, market, as_of).map_err(|e| {
            PricingError::model_failure_with_context(e.to_string(), PricingErrorContext::default())
        })?;

        Ok(ValuationResult::stamped(barrier.id(), as_of, pv))
    }
}

/// Present value using Monte Carlo.
#[cfg(feature = "mc")]
pub(crate) fn compute_pv(
    inst: &BarrierOption,
    curves: &MarketContext,
    as_of: Date,
) -> finstack_core::Result<Money> {
    let pricer = BarrierOptionMcPricer::new();
    pricer.price_internal(inst, curves, as_of)
}

/// Present value with LRM Greeks via Monte Carlo (barrier option).
///
/// Returns `(pv, Option<(delta, vega)>)` where the Greeks are from the
/// Likelihood Ratio Method. Greeks are `None` if the option is expired.
#[allow(dead_code)] // May be used by external bindings or tests
#[cfg(feature = "mc")]
pub fn npv_with_lrm_greeks(
    inst: &BarrierOption,
    curves: &MarketContext,
    as_of: Date,
) -> finstack_core::Result<(Money, Option<(f64, f64)>)> {
    let pricer = BarrierOptionMcPricer::new();
    pricer.price_with_lrm_greeks_internal(inst, curves, as_of)
}

// ========================= EXPIRED BARRIER HELPER =========================

/// Price an expired barrier option using explicit observed barrier state.
///
/// Terminal spot alone is insufficient to determine whether a barrier was
/// breached intralife and then later reversed, so expired contracts require
/// the caller to provide `observed_barrier_breached`.
/// The intrinsic value is `max(S - K, 0)` for calls and `max(K - S, 0)` for puts,
/// scaled by notional.
fn price_expired_barrier(
    inst: &BarrierOption,
    curves: &MarketContext,
) -> finstack_core::Result<Money> {
    use crate::instruments::exotics::barrier_option::types::BarrierType;

    let spot_scalar = curves.get_price(&inst.spot_id)?;
    let spot = match spot_scalar {
        finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
        finstack_core::market_data::scalars::MarketScalar::Price(m) => m.amount(),
    };

    let ccy = inst.notional.currency();
    let notional = inst.notional.amount();
    let is_knock_out = matches!(
        inst.barrier_type,
        BarrierType::UpAndOut | BarrierType::DownAndOut
    );

    let barrier_breached = inst.observed_barrier_breached.ok_or_else(|| {
        finstack_core::Error::Validation(
            "Expired barrier option requires `observed_barrier_breached` to determine realized payoff"
                .to_string(),
        )
    })?;

    let intrinsic = match inst.option_type {
        crate::instruments::OptionType::Call => (spot - inst.strike).max(0.0) * notional,
        crate::instruments::OptionType::Put => (inst.strike - spot).max(0.0) * notional,
    };
    let rebate = inst.rebate.map(|m| m.amount()).unwrap_or(0.0);

    let pv = if is_knock_out {
        if barrier_breached {
            rebate
        } else {
            intrinsic
        }
    } else {
        // Knock-in
        if barrier_breached {
            intrinsic
        } else {
            rebate
        }
    };

    Ok(Money::new(pv, ccy))
}

// ========================= ANALYTICAL PRICER =========================

use crate::instruments::common_impl::models::closed_form::barrier::{
    barrier_call_continuous_df, barrier_put_continuous_df, barrier_rebate_continuous_df,
    BarrierType as AnalyticalBarrierType,
};
/// Broadie-Glasserman-Kou / Gobet-Miri discrete barrier adjustment constant.
/// β = -ζ(1/2) / √(2π) ≈ 0.5826
const BG_BETA: f64 = 0.5826;

/// Barrier option analytical pricer (continuous monitoring).
///
/// # Monitoring Convention
///
/// **Important**: This pricer uses **continuous monitoring** Reiner-Rubinstein formulas.
/// Real-world barriers are typically monitored discretely (e.g., daily closes).
/// Continuous barrier formulas **systematically underestimate** knock-out option values
/// and overestimate knock-in option values compared to discrete monitoring.
///
/// For discrete monitoring pricing, use the Monte Carlo pricer
/// ([`BarrierOptionMcPricer`]) which applies the Broadie-Glasserman-Kou / Gobet-Miri
/// correction when `use_gobet_miri = true`.
///
/// `BarrierOption::value()` dispatches to this analytical pricer only when
/// `use_gobet_miri = false`. When `use_gobet_miri = true`, `value()` routes
/// to the MC pricer (`npv_mc()`) for discrete-monitoring-corrected prices.
pub struct BarrierOptionAnalyticalPricer;

impl BarrierOptionAnalyticalPricer {
    /// Create a new analytical barrier option pricer
    pub fn new() -> Self {
        Self
    }
}

impl Default for BarrierOptionAnalyticalPricer {
    fn default() -> Self {
        Self::new()
    }
}

impl Pricer for BarrierOptionAnalyticalPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::BarrierOption, ModelKey::BarrierBSContinuous)
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
        as_of: Date,
    ) -> PricingResult<ValuationResult> {
        let barrier_opt = instrument
            .as_any()
            .downcast_ref::<BarrierOption>()
            .ok_or_else(|| {
                PricingError::type_mismatch(InstrumentType::BarrierOption, instrument.key())
            })?;

        if barrier_opt.use_gobet_miri {
            tracing::warn!(
                "Analytical barrier pricer uses continuous monitoring; discrete monitoring flag \
                 is ignored. Use Monte Carlo pricer for discrete barrier monitoring."
            );
        }

        // Use DF-first input collection to keep vol lookup on the instrument clock
        // while preserving discounting on the discount curve clock.
        let bs_inputs = crate::instruments::common_impl::helpers::collect_black_scholes_inputs_df(
            &barrier_opt.spot_id,
            &barrier_opt.discount_curve_id,
            barrier_opt.div_yield_id.as_ref(),
            &barrier_opt.vol_surface_id,
            barrier_opt.strike,
            barrier_opt.expiry,
            barrier_opt.day_count,
            market,
            as_of,
        )
        .map_err(|e| {
            PricingError::model_failure_with_context(e.to_string(), PricingErrorContext::default())
        })?;
        let spot = bs_inputs.spot;
        let q = bs_inputs.q;
        let sigma = bs_inputs.sigma;
        let t = bs_inputs.t;
        let df = bs_inputs.df;

        if t <= 0.0 {
            let pv = price_expired_barrier(barrier_opt, market).map_err(|e| {
                PricingError::model_failure_with_context(
                    e.to_string(),
                    PricingErrorContext::default(),
                )
            })?;
            return Ok(ValuationResult::stamped(barrier_opt.id(), as_of, pv));
        }

        // Map barrier type
        use crate::instruments::exotics::barrier_option::types::BarrierType;
        let analytical_barrier_type = match barrier_opt.barrier_type {
            BarrierType::UpAndIn => AnalyticalBarrierType::UpIn,
            BarrierType::UpAndOut => AnalyticalBarrierType::UpOut,
            BarrierType::DownAndIn => AnalyticalBarrierType::DownIn,
            BarrierType::DownAndOut => AnalyticalBarrierType::DownOut,
        };

        // Apply Broadie-Glasserman discrete monitoring correction when monitoring_frequency is set
        let is_down = matches!(
            barrier_opt.barrier_type,
            BarrierType::DownAndIn | BarrierType::DownAndOut
        );
        let effective_barrier = if let Some(dt) = barrier_opt.monitoring_frequency {
            let shift = BG_BETA * sigma * dt.sqrt();
            if is_down {
                barrier_opt.barrier.amount() * (-shift).exp()
            } else {
                barrier_opt.barrier.amount() * shift.exp()
            }
        } else {
            barrier_opt.barrier.amount()
        };

        let price = match barrier_opt.option_type {
            crate::instruments::OptionType::Call => barrier_call_continuous_df(
                spot,
                barrier_opt.strike,
                effective_barrier,
                t,
                df,
                q,
                sigma,
                analytical_barrier_type,
            ),
            crate::instruments::OptionType::Put => barrier_put_continuous_df(
                spot,
                barrier_opt.strike,
                effective_barrier,
                t,
                df,
                q,
                sigma,
                analytical_barrier_type,
            ),
        };

        let rebate_val = if let Some(rebate) = barrier_opt.rebate {
            barrier_rebate_continuous_df(
                spot,
                effective_barrier,
                rebate.amount(),
                t,
                df,
                q,
                sigma,
                analytical_barrier_type,
            )
        } else {
            0.0
        };

        let pv = Money::new(
            (price + rebate_val) * barrier_opt.notional.amount(),
            barrier_opt.notional.currency(),
        );
        Ok(ValuationResult::stamped(barrier_opt.id(), as_of, pv))
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::instruments::common_impl::models::closed_form::barrier::{
        barrier_rebate_continuous, down_out_call,
    };
    use crate::instruments::exotics::barrier_option::types::{BarrierOption, BarrierType};
    use crate::instruments::{Attributes, OptionType, PricingOverrides};
    use finstack_core::currency::Currency;
    use finstack_core::dates::{DayCount, DayCountCtx};
    use finstack_core::market_data::scalars::MarketScalar;
    use finstack_core::market_data::surfaces::VolSurface;
    use finstack_core::market_data::term_structures::DiscountCurve;
    use finstack_core::types::InstrumentId;
    use time::Month;

    fn date(year: i32, month: u8, day: u8) -> Date {
        Date::from_calendar_date(year, Month::try_from(month).expect("valid month"), day)
            .expect("valid date")
    }

    fn market(as_of: Date, spot: f64, vol: f64, rate: f64, div_yield: f64) -> MarketContext {
        let discount = DiscountCurve::builder("USD_DISC")
            .base_date(as_of)
            .day_count(DayCount::Act365F)
            .knots([(0.0, 1.0), (5.0, (-rate * 5.0).exp())])
            .build()
            .expect("discount curve");
        let surface = VolSurface::builder("SPX_VOL")
            .expiries(&[0.25, 0.5, 1.0, 2.0])
            .strikes(&[80.0, 90.0, 100.0, 110.0, 120.0])
            .row(&[vol, vol, vol, vol, vol])
            .row(&[vol, vol, vol, vol, vol])
            .row(&[vol, vol, vol, vol, vol])
            .row(&[vol, vol, vol, vol, vol])
            .build()
            .expect("vol surface");

        MarketContext::new()
            .insert(discount)
            .insert_surface(surface)
            .insert_price(
                "SPX",
                MarketScalar::Price(Money::new(spot, Currency::USD)),
            )
            .insert_price("SPX_DIV", MarketScalar::Unitless(div_yield))
    }

    fn down_and_out_call(expiry: Date, strike: f64, barrier: f64) -> BarrierOption {
        BarrierOption {
            id: InstrumentId::new("BARRIER-BENCH"),
            underlying_ticker: "SPX".to_string(),
            strike,
            barrier: Money::new(barrier, Currency::USD),
            rebate: None,
            option_type: OptionType::Call,
            barrier_type: BarrierType::DownAndOut,
            expiry,
            observed_barrier_breached: None,
            notional: Money::new(1.0, Currency::USD),
            day_count: DayCount::Act365F,
            use_gobet_miri: false,
            discount_curve_id: "USD_DISC".into(),
            spot_id: "SPX".into(),
            vol_surface_id: "SPX_VOL".into(),
            div_yield_id: Some("SPX_DIV".into()),
            pricing_overrides: PricingOverrides::default(),
            monitoring_frequency: None,
            attributes: Attributes::new(),
        }
    }

    #[test]
    fn analytical_pricer_matches_reiner_rubinstein_down_and_out_call() {
        let as_of = date(2024, 1, 1);
        let expiry = date(2024, 7, 1);
        let spot = 100.0;
        let strike = 100.0;
        let barrier = 80.0;
        let vol = 0.20;
        let rate = 0.05;
        let div_yield = 0.0;

        let option = down_and_out_call(expiry, strike, barrier);
        let market = market(as_of, spot, vol, rate, div_yield);
        let pv = option.value(&market, as_of).expect("barrier pv").amount();

        let t = option
            .day_count
            .year_fraction(as_of, expiry, DayCountCtx::default())
            .expect("year fraction");
        let expected = down_out_call(spot, strike, barrier, t, rate, div_yield, vol);

        assert!((pv - expected).abs() < 1e-12);
    }

    #[test]
    fn analytical_pricer_adds_reiner_rubinstein_rebate_value() {
        let as_of = date(2024, 1, 1);
        let expiry = date(2025, 1, 1);
        let spot = 100.0;
        let strike = 100.0;
        let barrier = 120.0;
        let vol = 0.18;
        let rate = 0.04;
        let div_yield = 0.01;
        let rebate = 2.5;

        let market = market(as_of, spot, vol, rate, div_yield);
        let base = BarrierOption {
            barrier_type: BarrierType::UpAndOut,
            option_type: OptionType::Call,
            barrier: Money::new(barrier, Currency::USD),
            ..down_and_out_call(expiry, strike, barrier)
        };
        let with_rebate = BarrierOption {
            rebate: Some(Money::new(rebate, Currency::USD)),
            ..base.clone()
        };

        let base_pv = base.value(&market, as_of).expect("base pv").amount();
        let rebate_pv = with_rebate.value(&market, as_of).expect("rebate pv").amount();

        let t = with_rebate
            .day_count
            .year_fraction(as_of, expiry, DayCountCtx::default())
            .expect("year fraction");
        let expected_rebate = barrier_rebate_continuous(
            spot,
            barrier,
            rebate,
            t,
            rate,
            div_yield,
            vol,
            AnalyticalBarrierType::UpOut,
        );

        assert!(((rebate_pv - base_pv) - expected_rebate).abs() < 1e-12);
    }
}
