//! Lookback option pricers (Monte Carlo and analytical).

// Common imports for all pricers
use crate::instruments::common_impl::traits::Instrument;
use crate::instruments::exotics::lookback_option::types::{LookbackOption, LookbackType};
use crate::pricer::{
    InstrumentType, ModelKey, Pricer, PricerKey, PricingError, PricingErrorContext, PricingResult,
};
use crate::results::ValuationResult;
use finstack_core::dates::{Date, DayCountCtx};
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;

// MC-specific imports
#[cfg(feature = "mc")]
use finstack_monte_carlo::payoff::lookback::{
    FloatingStrikeLookbackCall, FloatingStrikeLookbackPut, Lookback, LookbackDirection,
};
#[cfg(feature = "mc")]
use finstack_monte_carlo::pricer::path_dependent::{
    PathDependentPricer, PathDependentPricerConfig,
};
#[cfg(feature = "mc")]
use finstack_monte_carlo::process::gbm::{GbmParams, GbmProcess};

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
    ) -> finstack_core::Result<finstack_core::money::Money> {
        if as_of >= inst.expiry {
            let payoff = expired_lookback_payoff(inst, lookback_spot(curves, &inst.spot_id)?)?;
            return Ok(finstack_core::money::Money::new(
                payoff * inst.notional.amount(),
                inst.notional.currency(),
            ));
        }

        let t = inst
            .day_count
            .year_fraction(as_of, inst.expiry, DayCountCtx::default())?;
        if t <= 0.0 {
            let payoff = expired_lookback_payoff(inst, lookback_spot(curves, &inst.spot_id)?)?;
            return Ok(finstack_core::money::Money::new(
                payoff * inst.notional.amount(),
                inst.notional.currency(),
            ));
        }

        let disc_curve = curves.get_discount(inst.discount_curve_id.as_str())?;
        let discount_factor = disc_curve.df_between_dates(as_of, inst.expiry)?;
        // Keep drift consistent with date-based discounting for MC simulation.
        let r = if t > 0.0 && discount_factor > 0.0 {
            -discount_factor.ln() / t
        } else {
            0.0
        };

        let spot_scalar = curves.get_price(&inst.spot_id)?;
        let spot = match spot_scalar {
            finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
            finstack_core::market_data::scalars::MarketScalar::Price(m) => m.amount(),
        };

        let q = crate::instruments::common_impl::helpers::resolve_optional_dividend_yield(
            curves,
            inst.div_yield_id.as_ref(),
        )?;

        let vol_surface = curves.get_surface(inst.vol_surface_id.as_str())?;
        let strike_val = inst.strike.unwrap_or(spot);
        let sigma = vol_surface.value_clamped(t, strike_val);

        let gbm_params = GbmParams::new(r, q, sigma);
        let process = GbmProcess::new(gbm_params);

        let steps_per_year = self.config.steps_per_year;
        let num_steps = ((t * steps_per_year).round() as usize).max(self.config.min_steps);
        let maturity_step = num_steps - 1;

        let currency = inst.notional.currency();

        // Derive deterministic seed from instrument ID and scenario
        #[cfg(feature = "mc")]
        use finstack_monte_carlo::seed;

        let seed = if let Some(ref scenario) = inst.pricing_overrides.metrics.mc_seed_scenario {
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
            (LookbackType::FloatingStrike, crate::instruments::OptionType::Call) => {
                // Floating Strike Call: Payoff = S_T - S_min
                // Seed initial minimum from observed_min if seasoned
                let initial_min = inst
                    .observed_min
                    .as_ref()
                    .map(|m| m.amount())
                    .unwrap_or(f64::INFINITY);
                let payoff = FloatingStrikeLookbackCall::with_initial_min(
                    inst.notional.amount(),
                    maturity_step,
                    initial_min,
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
            (LookbackType::FloatingStrike, crate::instruments::OptionType::Put) => {
                // Floating Strike Put: Payoff = S_max - S_T
                // Seed initial maximum from observed_max if seasoned
                let initial_max = inst
                    .observed_max
                    .as_ref()
                    .map(|m| m.amount())
                    .unwrap_or(f64::NEG_INFINITY);
                let payoff = FloatingStrikeLookbackPut::with_initial_max(
                    inst.notional.amount(),
                    maturity_step,
                    initial_max,
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
            (LookbackType::FixedStrike, crate::instruments::OptionType::Call) => {
                // Fixed Strike Call: Payoff = max(S_max - K, 0)
                // Seed initial maximum from observed_max if seasoned
                let initial_max = inst
                    .observed_max
                    .as_ref()
                    .map(|m| m.amount())
                    .unwrap_or(f64::NEG_INFINITY);
                let strike = inst.strike.as_ref().ok_or_else(|| {
                    finstack_core::Error::Validation(
                        "FixedStrike lookback requires a strike".into(),
                    )
                })?;
                let payoff = Lookback::with_initial_extremum(
                    LookbackDirection::Call,
                    *strike,
                    inst.notional.amount(),
                    maturity_step,
                    initial_max,
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
                // Fixed Strike Put: Payoff = max(K - S_min, 0)
                // Seed initial minimum from observed_min if seasoned
                let initial_min = inst
                    .observed_min
                    .as_ref()
                    .map(|m| m.amount())
                    .unwrap_or(f64::INFINITY);
                let strike = inst.strike.as_ref().ok_or_else(|| {
                    finstack_core::Error::Validation(
                        "FixedStrike lookback requires a strike".into(),
                    )
                })?;
                let payoff = Lookback::with_initial_extremum(
                    LookbackDirection::Put,
                    *strike,
                    inst.notional.amount(),
                    maturity_step,
                    initial_min,
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
        instrument: &dyn crate::instruments::common_impl::traits::Instrument,
        market: &MarketContext,
        as_of: Date,
    ) -> PricingResult<ValuationResult> {
        let lookback = instrument
            .as_any()
            .downcast_ref::<LookbackOption>()
            .ok_or_else(|| {
                PricingError::type_mismatch(InstrumentType::LookbackOption, instrument.key())
            })?;

        let pv = self.price_internal(lookback, market, as_of).map_err(|e| {
            PricingError::model_failure_with_context(e.to_string(), PricingErrorContext::default())
        })?;

        Ok(ValuationResult::stamped(lookback.id(), as_of, pv))
    }
}

/// Present value using Monte Carlo.
#[cfg(feature = "mc")]
pub(crate) fn compute_pv(
    inst: &LookbackOption,
    curves: &MarketContext,
    as_of: Date,
) -> finstack_core::Result<Money> {
    let pricer = LookbackOptionMcPricer::new();
    pricer.price_internal(inst, curves, as_of)
}

// ========================= ANALYTICAL PRICER =========================

use crate::instruments::common_impl::models::closed_form::lookback::{
    fixed_strike_lookback_call, fixed_strike_lookback_put, floating_strike_lookback_call,
    floating_strike_lookback_put,
};

fn lookback_spot(
    curves: &MarketContext,
    spot_id: &finstack_core::types::PriceId,
) -> finstack_core::Result<f64> {
    let spot_scalar = curves.get_price(spot_id)?;
    Ok(match spot_scalar {
        finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
        finstack_core::market_data::scalars::MarketScalar::Price(m) => m.amount(),
    })
}

fn expired_lookback_payoff(inst: &LookbackOption, spot: f64) -> finstack_core::Result<f64> {
    let payoff = match inst.lookback_type {
        LookbackType::FixedStrike => {
            let strike = inst.strike.ok_or_else(|| {
                finstack_core::Error::Validation(
                    "FixedStrike lookback requires a strike".to_string(),
                )
            })?;
            match inst.option_type {
                crate::instruments::OptionType::Call => {
                    let observed_max = inst
                        .observed_max
                        .as_ref()
                        .map(|m| m.amount())
                        .unwrap_or(spot);
                    (observed_max.max(spot) - strike).max(0.0)
                }
                crate::instruments::OptionType::Put => {
                    let observed_min = inst
                        .observed_min
                        .as_ref()
                        .map(|m| m.amount())
                        .unwrap_or(spot);
                    (strike - observed_min.min(spot)).max(0.0)
                }
            }
        }
        LookbackType::FloatingStrike => match inst.option_type {
            crate::instruments::OptionType::Call => {
                let observed_min = inst
                    .observed_min
                    .as_ref()
                    .map(|m| m.amount())
                    .unwrap_or(spot);
                spot - observed_min.min(spot)
            }
            crate::instruments::OptionType::Put => {
                let observed_max = inst
                    .observed_max
                    .as_ref()
                    .map(|m| m.amount())
                    .unwrap_or(spot);
                observed_max.max(spot) - spot
            }
        },
    };
    Ok(payoff)
}

/// Helper to collect inputs for lookback option pricing.
fn collect_lookback_inputs(
    inst: &LookbackOption,
    curves: &MarketContext,
    as_of: Date,
) -> finstack_core::Result<(f64, f64, f64, f64, f64)> {
    let t = inst
        .day_count
        .year_fraction(as_of, inst.expiry, DayCountCtx::default())?;

    let disc_curve = curves.get_discount(inst.discount_curve_id.as_str())?;
    let df = disc_curve.df_between_dates(as_of, inst.expiry)?;
    // Keep analytical rate consistent with date-based discounting used by the curve.
    let r = if t > 0.0 && df > 0.0 {
        -df.ln() / t
    } else {
        0.0
    };

    let spot_scalar = curves.get_price(&inst.spot_id)?;
    let spot = match spot_scalar {
        finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
        finstack_core::market_data::scalars::MarketScalar::Price(m) => m.amount(),
    };

    let q = crate::instruments::common_impl::helpers::resolve_optional_dividend_yield(
        curves,
        inst.div_yield_id.as_ref(),
    )?;

    let vol_surface = curves.get_surface(inst.vol_surface_id.as_str())?;
    let strike_val = inst.strike.unwrap_or(spot);
    let sigma = vol_surface.value_clamped(t, strike_val);

    Ok((spot, r, q, sigma, t))
}

/// Lookback option analytical pricer (continuous monitoring).
pub struct LookbackOptionAnalyticalPricer;

impl LookbackOptionAnalyticalPricer {
    /// Create a new analytical lookback option pricer
    pub fn new() -> Self {
        Self
    }
}

impl Default for LookbackOptionAnalyticalPricer {
    fn default() -> Self {
        Self::new()
    }
}

impl Pricer for LookbackOptionAnalyticalPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(
            InstrumentType::LookbackOption,
            ModelKey::LookbackBSContinuous,
        )
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
        as_of: Date,
    ) -> PricingResult<ValuationResult> {
        let lookback = instrument
            .as_any()
            .downcast_ref::<LookbackOption>()
            .ok_or_else(|| {
                PricingError::type_mismatch(InstrumentType::LookbackOption, instrument.key())
            })?;

        if as_of >= lookback.expiry {
            let spot = lookback_spot(market, &lookback.spot_id).map_err(|e| {
                PricingError::model_failure_with_context(
                    e.to_string(),
                    PricingErrorContext::default(),
                )
            })?;
            let payoff = expired_lookback_payoff(lookback, spot).map_err(|e| {
                PricingError::model_failure_with_context(
                    e.to_string(),
                    PricingErrorContext::default(),
                )
            })?;
            return Ok(ValuationResult::stamped(
                lookback.id(),
                as_of,
                Money::new(
                    payoff * lookback.notional.amount(),
                    lookback.notional.currency(),
                ),
            ));
        }

        let (spot, r, q, sigma, t) =
            collect_lookback_inputs(lookback, market, as_of).map_err(|e| {
                PricingError::model_failure_with_context(
                    e.to_string(),
                    PricingErrorContext::default(),
                )
            })?;

        if t <= 0.0 {
            let payoff = expired_lookback_payoff(lookback, spot).map_err(|e| {
                PricingError::model_failure_with_context(
                    e.to_string(),
                    PricingErrorContext::default(),
                )
            })?;
            return Ok(ValuationResult::stamped(
                lookback.id(),
                as_of,
                Money::new(
                    payoff * lookback.notional.amount(),
                    lookback.notional.currency(),
                ),
            ));
        }

        // Determine current extremum based on option type
        let spot_extremum = match lookback.lookback_type {
            LookbackType::FixedStrike => match lookback.option_type {
                crate::instruments::OptionType::Call => {
                    // Fixed Strike Call: Payoff = max(S_max - K, 0)
                    // Need max(observed_max, current_spot)
                    let obs_max = lookback
                        .observed_max
                        .as_ref()
                        .map(|m| m.amount())
                        .unwrap_or(spot);
                    obs_max.max(spot)
                }
                crate::instruments::OptionType::Put => {
                    // Fixed Strike Put: Payoff = max(K - S_min, 0)
                    // Need min(observed_min, current_spot)
                    let obs_min = lookback
                        .observed_min
                        .as_ref()
                        .map(|m| m.amount())
                        .unwrap_or(spot);
                    obs_min.min(spot)
                }
            },
            LookbackType::FloatingStrike => match lookback.option_type {
                crate::instruments::OptionType::Call => {
                    // Floating Strike Call: Payoff = S_T - S_min
                    // Need min(observed_min, current_spot)
                    let obs_min = lookback
                        .observed_min
                        .as_ref()
                        .map(|m| m.amount())
                        .unwrap_or(spot);
                    obs_min.min(spot)
                }
                crate::instruments::OptionType::Put => {
                    // Floating Strike Put: Payoff = S_max - S_T
                    // Need max(observed_max, current_spot)
                    let obs_max = lookback
                        .observed_max
                        .as_ref()
                        .map(|m| m.amount())
                        .unwrap_or(spot);
                    obs_max.max(spot)
                }
            },
        };

        let price = match lookback.lookback_type {
            LookbackType::FixedStrike => {
                let strike = lookback.strike.as_ref().ok_or_else(|| {
                    PricingError::model_failure_with_context(
                        "FixedStrike lookback requires a strike",
                        PricingErrorContext::default(),
                    )
                })?;
                match lookback.option_type {
                    crate::instruments::OptionType::Call => {
                        fixed_strike_lookback_call(spot, *strike, t, r, q, sigma, spot_extremum)
                    }
                    crate::instruments::OptionType::Put => {
                        fixed_strike_lookback_put(spot, *strike, t, r, q, sigma, spot_extremum)
                    }
                }
            }
            LookbackType::FloatingStrike => match lookback.option_type {
                crate::instruments::OptionType::Call => {
                    floating_strike_lookback_call(spot, t, r, q, sigma, spot_extremum)
                }
                crate::instruments::OptionType::Put => {
                    floating_strike_lookback_put(spot, t, r, q, sigma, spot_extremum)
                }
            },
        };

        let currency = lookback.notional.currency();
        let pv = Money::new(price * lookback.notional.amount(), currency);
        Ok(ValuationResult::stamped(lookback.id(), as_of, pv))
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::instruments::common_impl::models::closed_form::lookback::{
        fixed_strike_lookback_call, floating_strike_lookback_put,
    };
    use crate::instruments::exotics::lookback_option::{LookbackOption, LookbackType};
    use crate::instruments::{Attributes, OptionType, PricingOverrides};
    use finstack_core::currency::Currency;
    use finstack_core::dates::DayCount;
    use finstack_core::market_data::scalars::MarketScalar;
    use finstack_core::market_data::surfaces::VolSurface;
    use finstack_core::market_data::term_structures::DiscountCurve;
    use finstack_core::types::{CurveId, InstrumentId};
    use time::Month;

    fn date(year: i32, month: u8, day: u8) -> Date {
        Date::from_calendar_date(year, Month::try_from(month).expect("valid month"), day)
            .expect("valid date")
    }

    fn market(as_of: Date, spot: f64, vol: f64, rate: f64, div_yield: f64) -> MarketContext {
        let curve = DiscountCurve::builder("USD-OIS")
            .base_date(as_of)
            .day_count(DayCount::Act365F)
            .knots([(0.0, 1.0), (10.0, (-rate * 10.0).exp())])
            .build()
            .expect("discount curve");
        let surface = VolSurface::builder("SPX-VOL")
            .expiries(&[0.25, 0.5, 1.0, 2.0])
            .strikes(&[80.0, 100.0, 120.0, 150.0])
            .row(&[vol, vol, vol, vol])
            .row(&[vol, vol, vol, vol])
            .row(&[vol, vol, vol, vol])
            .row(&[vol, vol, vol, vol])
            .build()
            .expect("vol surface");

        MarketContext::new()
            .insert(curve)
            .insert_surface(surface)
            .insert_price(
                "SPX-SPOT",
                MarketScalar::Price(Money::new(spot, Currency::USD)),
            )
            .insert_price("SPX-DIV", MarketScalar::Unitless(div_yield))
    }

    fn fixed_strike_call(expiry: Date, strike: f64, observed_max: Option<f64>) -> LookbackOption {
        LookbackOption::builder()
            .id(InstrumentId::new("LOOKBACK-FIXED-CALL"))
            .underlying_ticker("SPX".to_string())
            .strike_opt(Some(strike))
            .option_type(OptionType::Call)
            .lookback_type(LookbackType::FixedStrike)
            .expiry(expiry)
            .notional(Money::new(1.0, Currency::USD))
            .day_count(DayCount::Act365F)
            .discount_curve_id(CurveId::new("USD-OIS"))
            .spot_id("SPX-SPOT".into())
            .vol_surface_id(CurveId::new("SPX-VOL"))
            .div_yield_id_opt(Some(CurveId::new("SPX-DIV")))
            .pricing_overrides(PricingOverrides::default())
            .observed_max_opt(observed_max.map(|value| Money::new(value, Currency::USD)))
            .attributes(Attributes::new())
            .build()
            .expect("lookback option")
    }

    fn floating_strike_put(expiry: Date, observed_max: Option<f64>) -> LookbackOption {
        LookbackOption::builder()
            .id(InstrumentId::new("LOOKBACK-FLOAT-PUT"))
            .underlying_ticker("SPX".to_string())
            .strike_opt(None)
            .option_type(OptionType::Put)
            .lookback_type(LookbackType::FloatingStrike)
            .expiry(expiry)
            .notional(Money::new(1.0, Currency::USD))
            .day_count(DayCount::Act365F)
            .discount_curve_id(CurveId::new("USD-OIS"))
            .spot_id("SPX-SPOT".into())
            .vol_surface_id(CurveId::new("SPX-VOL"))
            .div_yield_id_opt(Some(CurveId::new("SPX-DIV")))
            .pricing_overrides(PricingOverrides::default())
            .observed_max_opt(observed_max.map(|value| Money::new(value, Currency::USD)))
            .attributes(Attributes::new())
            .build()
            .expect("lookback option")
    }

    #[test]
    fn analytical_pricer_matches_fixed_strike_lookback_call_benchmark() {
        let as_of = date(2025, 1, 1);
        let expiry = date(2026, 1, 1);
        let spot = 100.0;
        let strike = 100.0;
        let observed_max = 120.0;
        let rate = 0.05;
        let div_yield = 0.0;
        let vol = 0.20;

        let option = fixed_strike_call(expiry, strike, Some(observed_max));
        let market = market(as_of, spot, vol, rate, div_yield);
        let pv = option.value(&market, as_of).expect("lookback pv").amount();

        let t = option
            .day_count
            .year_fraction(as_of, expiry, DayCountCtx::default())
            .expect("year fraction");
        let expected = fixed_strike_lookback_call(
            spot,
            strike,
            t,
            rate,
            div_yield,
            vol,
            observed_max.max(spot),
        );

        assert!((pv - expected).abs() < 1e-12);
    }

    #[test]
    fn analytical_pricer_matches_floating_strike_lookback_put_benchmark() {
        let as_of = date(2025, 1, 1);
        let expiry = date(2026, 1, 1);
        let spot = 100.0;
        let observed_max = 130.0;
        let rate = 0.03;
        let div_yield = 0.01;
        let vol = 0.25;

        let option = floating_strike_put(expiry, Some(observed_max));
        let market = market(as_of, spot, vol, rate, div_yield);
        let pv = option.value(&market, as_of).expect("lookback pv").amount();

        let t = option
            .day_count
            .year_fraction(as_of, expiry, DayCountCtx::default())
            .expect("year fraction");
        let expected = floating_strike_lookback_put(
            spot,
            t,
            rate,
            div_yield,
            vol,
            observed_max.max(spot),
        );

        assert!((pv - expected).abs() < 1e-12);
    }
}
