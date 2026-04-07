//! Equity option Black–Scholes pricing engine and greeks.
//!
//! Provides deterministic PV and greeks for `EquityOption` using the
//! Black–Scholes model with continuous dividend yield. Volatility is
//! sourced from a surface (clamped) unless overridden. This mirrors the
//! structure used by `fx_option` and keeps pricing logic separate from
//! instrument definitions.

use crate::instruments::common_impl::helpers::year_fraction;
use crate::instruments::common_impl::models::trees::binomial_tree::BinomialTree;
use crate::instruments::common_impl::models::{bs_greeks, bs_price, BsGreeks};
use crate::instruments::common_impl::parameters::{OptionMarketParams, OptionType};
use crate::instruments::equity::equity_option::types::EquityOption;
use crate::instruments::ExerciseStyle;
use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount};
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_core::Result;

/// Trading days per year for equity options (market standard for theta calculations)
const TRADING_DAYS_PER_YEAR: f64 = 252.0;

/// Present value using Black–Scholes; result currency is the instrument currency.
pub(crate) fn compute_pv(
    inst: &EquityOption,
    curves: &MarketContext,
    as_of: Date,
) -> Result<Money> {
    let (spot, r, q, sigma, t) = collect_inputs(inst, curves, as_of)?;
    let ccy = option_currency(inst);

    if t <= 0.0 {
        // Expired: intrinsic value scaled by notional amount
        let intrinsic = match inst.option_type {
            OptionType::Call => (spot - inst.strike).max(0.0),
            OptionType::Put => (inst.strike - spot).max(0.0),
        };
        return Ok(Money::new(intrinsic * inst.notional.amount(), ccy));
    }

    // Dispatch based on exercise style
    let unit_price = match inst.exercise_style {
        ExerciseStyle::European => {
            price_bs_unit(spot, inst.strike, r, q, sigma, t, inst.option_type)
        }
        ExerciseStyle::American => {
            // Use Leisen-Reimer tree for American options
            let steps = inst
                .pricing_overrides
                .model_config
                .tree_steps
                .unwrap_or(201);
            let tree = BinomialTree::leisen_reimer(steps);
            let params = OptionMarketParams {
                spot,
                strike: inst.strike,
                rate: r,
                dividend_yield: q,
                volatility: sigma,
                time_to_expiry: t,
                option_type: inst.option_type,
            };
            tree.price_american(&params)?
        }
        ExerciseStyle::Bermudan => {
            let schedule = inst.exercise_schedule.as_ref().ok_or_else(|| {
                finstack_core::Error::Validation(
                    "Bermudan equity option requires exercise_schedule".to_string(),
                )
            })?;
            let steps = inst
                .pricing_overrides
                .model_config
                .tree_steps
                .unwrap_or(201);
            let tree = BinomialTree::leisen_reimer(steps);
            let params = OptionMarketParams {
                spot,
                strike: inst.strike,
                rate: r,
                dividend_yield: q,
                volatility: sigma,
                time_to_expiry: t,
                option_type: inst.option_type,
            };
            let exercise_times: Vec<f64> = schedule
                .iter()
                .filter_map(|d| {
                    let yf = DayCount::Act365F
                        .year_fraction(as_of, *d, Default::default())
                        .ok()?;
                    if yf > 0.0 && yf <= t {
                        Some(yf)
                    } else {
                        None
                    }
                })
                .collect();
            tree.price_bermudan(&params, &exercise_times)?
        }
    };

    Ok(Money::new(unit_price * inst.notional.amount(), ccy))
}

fn option_currency(inst: &EquityOption) -> Currency {
    inst.notional.currency()
}

/// Collected market inputs for equity option pricing.
///
/// Separates time-to-expiry calculations by day count convention:
/// - `t_rate`: Time using the discount curve's day count (for rate lookups)
/// - `t_vol`: Time using ACT/365F (equity vol market standard)
#[derive(Debug, Clone, Copy)]
pub struct EquityOptionInputs {
    /// Spot price of the underlying
    pub spot: f64,
    /// Risk-free rate (from discount curve)
    /// Effective risk-free rate consistent with `t_vol`
    pub r: f64,
    /// Dividend yield
    pub q: f64,
    /// Implied volatility
    pub sigma: f64,
    /// Time to expiry for rate calculations (curve day count)
    #[allow(dead_code)] // part of public API result struct
    pub t_rate: f64,
    /// Time to expiry for vol calculations (ACT/365F standard)
    pub t_vol: f64,
}

/// Collect standard inputs (spot, risk-free, dividend yield, vol, time to expiry).
///
/// **Day Count Convention Handling:**
/// - Rate calculations use the discount curve's own day count
/// - Vol surface lookups use ACT/365F (equity market standard)
///
/// This separation ensures consistent pricing when discount curves use different
/// conventions (e.g., OIS curves with ACT/360) than the vol surface.
pub fn collect_inputs(
    inst: &EquityOption,
    curves: &MarketContext,
    as_of: Date,
) -> Result<(f64, f64, f64, f64, f64)> {
    let inputs = collect_inputs_extended(inst, curves, as_of)?;
    // Return t_vol as the primary time for the simplified interface
    Ok((inputs.spot, inputs.r, inputs.q, inputs.sigma, inputs.t_vol))
}

/// Collect inputs with separate rate and vol time fractions.
///
/// Returns `EquityOptionInputs` with properly separated day count handling:
/// - `t_rate`: Uses the discount curve's day count for rate lookups
/// - `t_vol`: Uses ACT/365F for volatility surface lookups (equity market standard)
///
/// # Discrete Dividend Handling
///
/// When `discrete_dividends` is non-empty and contains future dividends (ex-date > as_of
/// and ex-date <= expiry), the escrowed dividend model is applied:
/// - Spot is adjusted: `S* = S - Σ D_i × e^{-r × t_i}`
/// - Dividend yield `q` is set to 0.0 (dividends are already priced into S*)
///
/// This is the QuantLib-standard approach for discrete dividends in Black-Scholes.
pub fn collect_inputs_extended(
    inst: &EquityOption,
    curves: &MarketContext,
    as_of: Date,
) -> Result<EquityOptionInputs> {
    // Discount curve lookup - use curve's own day count for discount factor time
    let disc_curve = curves.get_discount(inst.discount_curve_id.as_str())?;
    let curve_dc = disc_curve.day_count();
    let t_rate = year_fraction(curve_dc, as_of, inst.expiry)?;
    let df = disc_curve.df(t_rate);

    // Vol time uses ACT/365F (equity market standard for vol surfaces)
    // This is consistent with how equity volatility is quoted in the market
    let t_vol = year_fraction(DayCount::Act365F, as_of, inst.expiry)?;
    let r = if t_vol > 0.0 { -df.ln() / t_vol } else { 0.0 };

    // Spot from scalar id (unitless or price)
    let spot_scalar = curves.get_price(&inst.spot_id)?;
    let raw_spot = match spot_scalar {
        finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
        finstack_core::market_data::scalars::MarketScalar::Price(m) => m.amount(),
    };

    // Check for discrete dividends — if present, adjust spot and zero out q
    let future_divs: Vec<(f64, f64)> = if !inst.discrete_dividends.is_empty() {
        inst.discrete_dividends
            .iter()
            .filter(|(ex_date, _)| *ex_date > as_of && *ex_date <= inst.expiry)
            .map(|(ex_date, amount)| {
                let t_div = year_fraction(DayCount::Act365F, as_of, *ex_date)?;
                Ok((t_div, *amount))
            })
            .collect::<finstack_core::Result<Vec<_>>>()?
            .into_iter()
            .filter(|(t_div, _)| *t_div > 0.0)
            .collect()
    } else {
        Vec::new()
    };

    let (spot, q) = if !future_divs.is_empty() {
        // Escrowed dividend model: adjust spot, set q=0
        let s_adj = adjust_spot_for_discrete_dividends(raw_spot, r, &future_divs);
        (s_adj, 0.0)
    } else {
        // Continuous dividend yield from scalar id if provided
        //
        // When a dividend yield ID is explicitly provided, we require the lookup to succeed
        // and return a unitless scalar. Silent fallback to 0.0 would mask market data
        // configuration errors.
        let q = if let Some(div_id) = &inst.div_yield_id {
            let ms = curves.get_price(div_id.as_str()).map_err(|e| {
                finstack_core::Error::Validation(format!(
                    "Failed to fetch dividend yield '{}': {}",
                    div_id, e
                ))
            })?;
            match ms {
                finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
                finstack_core::market_data::scalars::MarketScalar::Price(m) => {
                    return Err(finstack_core::Error::Validation(format!(
                        "Dividend yield '{}' should be a unitless scalar, got Price({})",
                        div_id,
                        m.currency()
                    )));
                }
            }
        } else {
            0.0
        };
        (raw_spot, q)
    };

    // Volatility from override or surface (using t_vol for surface lookup)
    let sigma = if let Some(impl_vol) = inst.pricing_overrides.market_quotes.implied_volatility {
        impl_vol
    } else {
        let vol_surface = curves.get_surface(inst.vol_surface_id.as_str())?;
        vol_surface.value_clamped(t_vol, inst.strike)
    };

    Ok(EquityOptionInputs {
        spot,
        r,
        q,
        sigma,
        t_rate,
        t_vol,
    })
}

/// Adjust spot price for discrete dividends using the present-value method.
///
/// This is the QuantLib-standard approach for handling discrete dividends in
/// the Black-Scholes framework. The adjusted spot replaces the original spot
/// in all BS formulas (pricing, Greeks, implied vol):
///
/// ```text
/// S_adj = S - Σ D_i × e^{-r × t_i}
/// ```
///
/// where:
/// - `S` = current spot price
/// - `D_i` = dividend amount at time `t_i`
/// - `r` = risk-free rate
/// - `t_i` = time to dividend payment in years (only dividends before expiry)
///
/// # Arguments
///
/// * `spot` - Current spot price of the underlying
/// * `rate` - Risk-free rate (annualized, continuous compounding)
/// * `dividends` - Slice of `(time_to_payment, dividend_amount)` pairs
///   where `time_to_payment` is in years from valuation date
///
/// # Returns
///
/// Adjusted spot price. Always returns at least `1e-8` to avoid degenerate
/// pricing when PV of dividends exceeds spot (deep ITM dividend scenario).
///
/// # Example
///
/// ```rust,no_run
/// # fn main() {
/// // Stock at $100, dividend of $2 in 0.25 years, r = 5%
/// // s_adj ≈ 100 - 2 × e^{-0.05×0.25} ≈ 98.01
/// let s_adj = 100.0 - 2.0 * (-0.05_f64 * 0.25).exp();
/// assert!((s_adj - 98.01).abs() < 0.01);
/// # }
/// ```
///
/// # References
///
/// - Hull, J. C. (2018). *Options, Futures, and Other Derivatives*, Chapter 15.
/// - QuantLib: `DividendVanillaOption` with `AnalyticEuropeanEngine`
pub fn adjust_spot_for_discrete_dividends(spot: f64, rate: f64, dividends: &[(f64, f64)]) -> f64 {
    let pv_dividends: f64 = dividends
        .iter()
        .filter(|(t, _)| *t > 0.0)
        .map(|(t, d)| d * (-rate * t).exp())
        .sum();
    (spot - pv_dividends).max(1e-8)
}

/// Unit price under Black–Scholes (no contract size scaling).
#[inline]
pub fn price_bs_unit(
    spot: f64,
    strike: f64,
    r: f64,
    q: f64,
    sigma: f64,
    t: f64,
    option_type: OptionType,
) -> f64 {
    bs_price(spot, strike, r, q, sigma, t, option_type)
}

/// Cash greeks for an equity option (scaled by contract size; vega per 1% vol).
#[derive(Debug, Clone, Copy, Default)]
pub struct EquityOptionGreeks {
    /// Delta: sensitivity to underlying price (scaled by contract size)
    pub delta: f64,
    /// Gamma: rate of change of delta with respect to underlying price
    pub gamma: f64,
    /// Vega: sensitivity to 1% change in volatility
    pub vega: f64,
    /// Theta: time decay per day
    pub theta: f64,
    /// Rho: sensitivity to 1% change in risk-free rate
    pub rho: f64,
}

/// Compute greeks consistent with the pricing inputs.
///
/// Uses proper day count handling:
/// - Rate lookups use the discount curve's day count
/// - Vol time uses ACT/365F (equity market standard)
pub fn compute_greeks(
    inst: &EquityOption,
    curves: &MarketContext,
    as_of: Date,
) -> Result<EquityOptionGreeks> {
    let inputs = collect_inputs_extended(inst, curves, as_of)?;
    let (spot, r, q, sigma, t) = (inputs.spot, inputs.r, inputs.q, inputs.sigma, inputs.t_vol);

    if t <= 0.0 {
        // At expiry, delta is the step function of the payoff.
        // ATM (spot == strike) uses the convention 0.5 / -0.5,
        // consistent with QuantLib and Bloomberg.
        let strike = inst.strike;
        let delta_unit = match inst.option_type {
            OptionType::Call => {
                if spot > strike {
                    1.0
                } else if (spot - strike).abs() < 1e-12 * strike.abs().max(1.0) {
                    0.5
                } else {
                    0.0
                }
            }
            OptionType::Put => {
                if spot < strike {
                    -1.0
                } else if (spot - strike).abs() < 1e-12 * strike.abs().max(1.0) {
                    -0.5
                } else {
                    0.0
                }
            }
        };
        let scale = inst.notional.amount();
        return Ok(EquityOptionGreeks {
            delta: delta_unit * scale,
            ..Default::default()
        });
    }

    match inst.exercise_style {
        ExerciseStyle::European => {
            let greeks_unit = bs_greeks(
                spot,
                inst.strike,
                r,
                q,
                sigma,
                t,
                inst.option_type,
                TRADING_DAYS_PER_YEAR,
            );
            let scale = inst.notional.amount();
            Ok(EquityOptionGreeks {
                delta: greeks_unit.delta * scale,
                gamma: greeks_unit.gamma * scale,
                vega: greeks_unit.vega * scale,
                theta: greeks_unit.theta * scale,
                rho: greeks_unit.rho_r * scale,
            })
        }
        ExerciseStyle::American => {
            // American: Use Tree with Finite Differences
            let steps = inst
                .pricing_overrides
                .model_config
                .tree_steps
                .unwrap_or(201);
            let tree = BinomialTree::leisen_reimer(steps);
            let params = OptionMarketParams {
                spot,
                strike: inst.strike,
                rate: r,
                dividend_yield: q,
                volatility: sigma,
                time_to_expiry: t,
                option_type: inst.option_type,
            };

            // Helper to price
            let price_fn = |p: &OptionMarketParams| -> Result<f64> { tree.price_american(p) };

            let base_price = price_fn(&params)?;

            // Delta & Gamma (1% spot bump)
            let h_s = spot * 0.01;
            let mut p_up = params.clone();
            p_up.spot += h_s;
            let price_up = price_fn(&p_up)?;
            let mut p_dn = params.clone();
            p_dn.spot -= h_s;
            let price_dn = price_fn(&p_dn)?;

            let delta_unit = (price_up - price_dn) / (2.0 * h_s);
            let gamma_unit = (price_up - 2.0 * base_price + price_dn) / (h_s * h_s);

            // Vega (1% vol bump) - central difference for O(h²) accuracy
            let h_v = 0.01;
            let mut p_v_up = params.clone();
            p_v_up.volatility += h_v;
            let price_v_up = price_fn(&p_v_up)?;
            let mut p_v_dn = params.clone();
            p_v_dn.volatility = (p_v_dn.volatility - h_v).max(1e-8); // Ensure vol stays positive
            let price_v_dn = price_fn(&p_v_dn)?;
            let vega_unit = (price_v_up - price_v_dn) / 2.0; // Per 1% vol change

            // Rho (1% rate bump) - central difference for O(h²) accuracy
            let h_r = 0.01;
            let mut p_r_up = params.clone();
            p_r_up.rate += h_r;
            let price_r_up = price_fn(&p_r_up)?;
            let mut p_r_dn = params.clone();
            p_r_dn.rate -= h_r;
            let price_r_dn = price_fn(&p_r_dn)?;
            let rho_unit = (price_r_up - price_r_dn) / 2.0; // Per 1% rate change

            // Theta (1 trading-day bump, consistent with European BS theta)
            let dt = 1.0 / TRADING_DAYS_PER_YEAR;
            let theta_unit = if t > dt {
                let mut p_t = params.clone();
                p_t.time_to_expiry -= dt;
                let price_t = price_fn(&p_t)?;
                price_t - base_price // change per trading day
            } else {
                0.0
            };

            let scale = inst.notional.amount();
            Ok(EquityOptionGreeks {
                delta: delta_unit * scale,
                gamma: gamma_unit * scale,
                vega: vega_unit * scale,
                theta: theta_unit * scale,
                rho: rho_unit * scale,
            })
        }
        ExerciseStyle::Bermudan => {
            let schedule = inst.exercise_schedule.as_ref().ok_or_else(|| {
                finstack_core::Error::Validation(
                    "Bermudan equity option requires exercise_schedule".to_string(),
                )
            })?;
            let steps = inst
                .pricing_overrides
                .model_config
                .tree_steps
                .unwrap_or(201);
            let tree = BinomialTree::leisen_reimer(steps);
            let params = OptionMarketParams {
                spot,
                strike: inst.strike,
                rate: r,
                dividend_yield: q,
                volatility: sigma,
                time_to_expiry: t,
                option_type: inst.option_type,
            };
            let exercise_times: Vec<f64> = schedule
                .iter()
                .filter_map(|d| {
                    let yf = DayCount::Act365F
                        .year_fraction(as_of, *d, Default::default())
                        .ok()?;
                    if yf > 0.0 && yf <= t {
                        Some(yf)
                    } else {
                        None
                    }
                })
                .collect();

            let price_fn =
                |p: &OptionMarketParams| -> Result<f64> { tree.price_bermudan(p, &exercise_times) };

            let base_price = price_fn(&params)?;

            // Delta & Gamma (1% spot bump, central difference)
            let h_s = spot * 0.01;
            let mut p_up = params.clone();
            p_up.spot += h_s;
            let price_up = price_fn(&p_up)?;
            let mut p_dn = params.clone();
            p_dn.spot -= h_s;
            let price_dn = price_fn(&p_dn)?;

            let delta_unit = (price_up - price_dn) / (2.0 * h_s);
            let gamma_unit = (price_up - 2.0 * base_price + price_dn) / (h_s * h_s);

            // Vega (1% vol bump)
            let h_v = 0.01;
            let mut p_v_up = params.clone();
            p_v_up.volatility += h_v;
            let price_v_up = price_fn(&p_v_up)?;
            let mut p_v_dn = params.clone();
            p_v_dn.volatility = (p_v_dn.volatility - h_v).max(1e-8);
            let price_v_dn = price_fn(&p_v_dn)?;
            let vega_unit = (price_v_up - price_v_dn) / 2.0;

            // Rho (1% rate bump)
            let h_r = 0.01;
            let mut p_r_up = params.clone();
            p_r_up.rate += h_r;
            let price_r_up = price_fn(&p_r_up)?;
            let mut p_r_dn = params.clone();
            p_r_dn.rate -= h_r;
            let price_r_dn = price_fn(&p_r_dn)?;
            let rho_unit = (price_r_up - price_r_dn) / 2.0;

            // Theta (1 trading-day bump)
            let dt = 1.0 / TRADING_DAYS_PER_YEAR;
            let theta_unit = if t > dt {
                let mut p_t = params.clone();
                p_t.time_to_expiry -= dt;
                let price_t = price_fn(&p_t)?;
                price_t - base_price
            } else {
                0.0
            };

            let scale = inst.notional.amount();
            Ok(EquityOptionGreeks {
                delta: delta_unit * scale,
                gamma: gamma_unit * scale,
                vega: vega_unit * scale,
                theta: theta_unit * scale,
                rho: rho_unit * scale,
            })
        }
    }
}

/// Unit greeks (per share, not scaled by contract size).
pub type UnitGreeks = BsGreeks;

/// Compute unit greeks from explicit inputs (no market lookups).
#[allow(dead_code)] // May be used by external bindings or tests
#[inline]
pub fn greeks_unit(
    spot: f64,
    strike: f64,
    r: f64,
    q: f64,
    sigma: f64,
    t: f64,
    option_type: OptionType,
) -> UnitGreeks {
    if t <= 0.0 {
        // ATM convention: 0.5 / -0.5 (QuantLib/Bloomberg standard)
        let delta = match option_type {
            OptionType::Call => {
                if spot > strike {
                    1.0
                } else if (spot - strike).abs() < 1e-12 * strike.abs().max(1.0) {
                    0.5
                } else {
                    0.0
                }
            }
            OptionType::Put => {
                if spot < strike {
                    -1.0
                } else if (spot - strike).abs() < 1e-12 * strike.abs().max(1.0) {
                    -0.5
                } else {
                    0.0
                }
            }
        };
        return UnitGreeks {
            delta,
            ..Default::default()
        };
    }

    bs_greeks(
        spot,
        strike,
        r,
        q,
        sigma,
        t,
        option_type,
        TRADING_DAYS_PER_YEAR,
    )
}

// ========================= REGISTRY PRICER =========================

/// Registry pricer for Equity Option using Black-Scholes model
pub struct SimpleEquityOptionBlackPricer {
    model: crate::pricer::ModelKey,
}

impl SimpleEquityOptionBlackPricer {
    /// Create new Black-Scholes pricer with default model.
    ///
    /// Uses `ModelKey::Black76` which is the library-wide convention for
    /// lognormal option pricing.  BSM and Black-76 are mathematically
    /// equivalent (BSM is Black-76 applied to the forward
    /// `F = S × exp((r-q)T)`), so the same model key covers both.
    pub fn new() -> Self {
        Self {
            model: crate::pricer::ModelKey::Black76,
        }
    }

    /// Create pricer with specified model key
    pub fn with_model(model: crate::pricer::ModelKey) -> Self {
        Self { model }
    }
}

impl Default for SimpleEquityOptionBlackPricer {
    fn default() -> Self {
        Self::new()
    }
}

impl crate::pricer::Pricer for SimpleEquityOptionBlackPricer {
    fn key(&self) -> crate::pricer::PricerKey {
        crate::pricer::PricerKey::new(crate::pricer::InstrumentType::EquityOption, self.model)
    }

    fn price_dyn(
        &self,
        instrument: &dyn crate::instruments::common_impl::traits::Instrument,
        market: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> std::result::Result<crate::results::ValuationResult, crate::pricer::PricingError> {
        use crate::instruments::common_impl::traits::Instrument;

        // Type-safe downcasting
        let equity_option = instrument
            .as_any()
            .downcast_ref::<crate::instruments::equity::equity_option::EquityOption>()
            .ok_or_else(|| {
                crate::pricer::PricingError::type_mismatch(
                    crate::pricer::InstrumentType::EquityOption,
                    instrument.key(),
                )
            })?;

        // Use the provided as_of date for consistency
        // Compute present value using the engine
        let pv = compute_pv(equity_option, market, as_of).map_err(|e| {
            crate::pricer::PricingError::model_failure_with_context(
                e.to_string(),
                crate::pricer::PricingErrorContext::default(),
            )
        })?;

        // Return stamped result
        Ok(crate::results::ValuationResult::stamped(
            equity_option.id(),
            as_of,
            pv,
        ))
    }
}

// ========================= HESTON FOURIER PRICER =========================

#[cfg(feature = "mc")]
use crate::instruments::common_impl::models::closed_form::heston::{
    heston_call_price_fourier, heston_put_price_fourier, HestonParams,
};
#[cfg(feature = "mc")]
use crate::instruments::common_impl::traits::Instrument;

/// Equity option Heston semi-analytical pricer (Fourier inversion).
#[cfg(feature = "mc")]
pub struct EquityOptionHestonFourierPricer;

#[cfg(feature = "mc")]
impl EquityOptionHestonFourierPricer {
    /// Create a new Heston Fourier transform pricer
    pub fn new() -> Self {
        Self
    }
}

#[cfg(feature = "mc")]
impl Default for EquityOptionHestonFourierPricer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "mc")]
impl crate::pricer::Pricer for EquityOptionHestonFourierPricer {
    fn key(&self) -> crate::pricer::PricerKey {
        crate::pricer::PricerKey::new(
            crate::pricer::InstrumentType::EquityOption,
            crate::pricer::ModelKey::HestonFourier,
        )
    }

    fn price_dyn(
        &self,
        instrument: &dyn crate::instruments::common_impl::traits::Instrument,
        market: &MarketContext,
        as_of: Date,
    ) -> crate::pricer::PricingResult<crate::results::ValuationResult> {
        let equity_option = instrument
            .as_any()
            .downcast_ref::<EquityOption>()
            .ok_or_else(|| {
                crate::pricer::PricingError::type_mismatch(
                    crate::pricer::InstrumentType::EquityOption,
                    instrument.key(),
                )
            })?;

        let inputs = collect_inputs_extended(equity_option, market, as_of).map_err(|e| {
            crate::pricer::PricingError::model_failure_with_context(
                e.to_string(),
                crate::pricer::PricingErrorContext::default(),
            )
        })?;
        let (spot, r, q, _sigma, t) = (inputs.spot, inputs.r, inputs.q, inputs.sigma, inputs.t_vol);

        if t <= 0.0 {
            let intrinsic = match equity_option.option_type {
                OptionType::Call => (spot - equity_option.strike).max(0.0),
                OptionType::Put => (equity_option.strike - spot).max(0.0),
            };
            return Ok(crate::results::ValuationResult::stamped(
                equity_option.id(),
                as_of,
                Money::new(
                    intrinsic * equity_option.notional.amount(),
                    option_currency(equity_option),
                ),
            ));
        }

        // Fetch Heston parameters from market data or use defaults
        // Priority: instrument overrides > market scalars > defaults
        let kappa = market
            .get_price("HESTON_KAPPA")
            .ok()
            .and_then(|s| match s {
                finstack_core::market_data::scalars::MarketScalar::Unitless(v) => Some(*v),
                _ => None,
            })
            .unwrap_or(2.0);

        let theta = market
            .get_price("HESTON_THETA")
            .ok()
            .and_then(|s| match s {
                finstack_core::market_data::scalars::MarketScalar::Unitless(v) => Some(*v),
                _ => None,
            })
            .unwrap_or(0.04);

        let sigma_v = market
            .get_price("HESTON_SIGMA_V")
            .ok()
            .and_then(|s| match s {
                finstack_core::market_data::scalars::MarketScalar::Unitless(v) => Some(*v),
                _ => None,
            })
            .unwrap_or(0.3);

        let rho = market
            .get_price("HESTON_RHO")
            .ok()
            .and_then(|s| match s {
                finstack_core::market_data::scalars::MarketScalar::Unitless(v) => Some(*v),
                _ => None,
            })
            .unwrap_or(-0.7);

        let v0 = market
            .get_price("HESTON_V0")
            .ok()
            .and_then(|s| match s {
                finstack_core::market_data::scalars::MarketScalar::Unitless(v) => Some(*v),
                _ => None,
            })
            .unwrap_or(0.04);

        let err_ctx = crate::pricer::PricingErrorContext::from_instrument(equity_option)
            .model(crate::pricer::ModelKey::HestonFourier);
        let params = HestonParams::new(r, q, kappa, theta, sigma_v, rho, v0)
            .map_err(|e| crate::pricer::PricingError::from_core(e, err_ctx))?;

        let price = match equity_option.option_type {
            OptionType::Call => heston_call_price_fourier(spot, equity_option.strike, t, &params),
            OptionType::Put => heston_put_price_fourier(spot, equity_option.strike, t, &params),
        };

        let pv = Money::new(
            price * equity_option.notional.amount(),
            option_currency(equity_option),
        );
        Ok(crate::results::ValuationResult::stamped(
            equity_option.id(),
            as_of,
            pv,
        ))
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::instruments::equity::equity_option::types::EquityOption;
    use crate::instruments::{Attributes, PricingOverrides, SettlementType};
    use finstack_core::market_data::context::MarketContext;
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
            .expect("curve");
        let surface = VolSurface::builder("SPX-VOL")
            .expiries(&[0.25, 0.5, 1.0, 2.0])
            .strikes(&[80.0, 100.0, 120.0, 150.0])
            .row(&[vol, vol, vol, vol])
            .row(&[vol, vol, vol, vol])
            .row(&[vol, vol, vol, vol])
            .row(&[vol, vol, vol, vol])
            .build()
            .expect("surface");

        MarketContext::new()
            .insert(curve)
            .insert_surface(surface)
            .insert_price("SPX-SPOT", MarketScalar::Unitless(spot))
            .insert_price("SPX-DIV", MarketScalar::Unitless(div_yield))
    }

    fn option(
        expiry: Date,
        option_type: OptionType,
        exercise_style: ExerciseStyle,
    ) -> EquityOption {
        EquityOption::builder()
            .id(InstrumentId::new("EQ-OPT-TEST"))
            .underlying_ticker("SPX".to_string())
            .strike(100.0)
            .option_type(option_type)
            .exercise_style(exercise_style)
            .expiry(expiry)
            .notional(Money::new(100.0, Currency::USD))
            .day_count(DayCount::Act365F)
            .settlement(SettlementType::Cash)
            .discount_curve_id(CurveId::new("USD-OIS"))
            .spot_id("SPX-SPOT".into())
            .vol_surface_id(CurveId::new("SPX-VOL"))
            .div_yield_id_opt(Some(CurveId::new("SPX-DIV")))
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .expect("equity option")
    }

    #[test]
    fn test_adjust_spot_for_discrete_dividends_single() {
        // Stock at $100, dividend of $2 in 0.25 years, r = 5%
        let s_adj = adjust_spot_for_discrete_dividends(100.0, 0.05, &[(0.25, 2.0)]);
        // PV(div) = 2 × e^{-0.05×0.25} ≈ 1.9751
        assert!((s_adj - 98.0248).abs() < 0.01);
    }

    #[test]
    fn test_adjust_spot_for_discrete_dividends_multiple() {
        let s_adj = adjust_spot_for_discrete_dividends(100.0, 0.05, &[(0.25, 1.5), (0.5, 1.5)]);
        let expected = 100.0 - 1.5 * (-0.05 * 0.25_f64).exp() - 1.5 * (-0.05 * 0.5_f64).exp();
        assert!((s_adj - expected).abs() < 1e-10);
    }

    #[test]
    fn test_adjust_spot_for_discrete_dividends_floor() {
        // Dividends exceed spot → clamped to 1e-8
        let s_adj = adjust_spot_for_discrete_dividends(1.0, 0.01, &[(0.1, 50.0)]);
        assert!((s_adj - 1e-8).abs() < 1e-12);
    }

    #[test]
    fn test_adjust_spot_for_discrete_dividends_empty() {
        let s_adj = adjust_spot_for_discrete_dividends(100.0, 0.05, &[]);
        assert!((s_adj - 100.0).abs() < 1e-12);
    }

    #[test]
    fn test_adjust_spot_for_discrete_dividends_skips_past() {
        // Dividend at t=0 or negative should be skipped
        let s_adj = adjust_spot_for_discrete_dividends(100.0, 0.05, &[(0.0, 5.0), (-0.1, 3.0)]);
        assert!((s_adj - 100.0).abs() < 1e-12);
    }

    #[test]
    fn test_expired_atm_delta_convention_matches_compute_greeks_and_unit_greeks() {
        let as_of = date(2025, 1, 1);
        let call = option(as_of, OptionType::Call, ExerciseStyle::European);
        let put = option(as_of, OptionType::Put, ExerciseStyle::European);
        let curves = market(as_of, 100.0, 0.20, 0.03, 0.01);

        let call_greeks = compute_greeks(&call, &curves, as_of).expect("call greeks");
        let put_greeks = compute_greeks(&put, &curves, as_of).expect("put greeks");
        let call_unit = greeks_unit(100.0, 100.0, 0.03, 0.01, 0.20, 0.0, OptionType::Call);
        let put_unit = greeks_unit(100.0, 100.0, 0.03, 0.01, 0.20, 0.0, OptionType::Put);

        assert_eq!(call_greeks.delta, 50.0);
        assert_eq!(put_greeks.delta, -50.0);
        assert_eq!(call_unit.delta, 0.5);
        assert_eq!(put_unit.delta, -0.5);
        assert_eq!(call_greeks.gamma, 0.0);
        assert_eq!(put_greeks.gamma, 0.0);
    }

    #[test]
    fn test_american_call_tree_path_prices_above_european() {
        let as_of = date(2025, 1, 1);
        let expiry = date(2025, 7, 1);
        let mut european = option(expiry, OptionType::Call, ExerciseStyle::European);
        let mut american = option(expiry, OptionType::Call, ExerciseStyle::American);
        european.pricing_overrides.model_config.tree_steps = Some(51);
        american.pricing_overrides.model_config.tree_steps = Some(51);
        let curves = market(as_of, 105.0, 0.22, 0.03, 0.01);

        let european_pv = compute_pv(&european, &curves, as_of).expect("european pv");
        let american_pv = compute_pv(&american, &curves, as_of).expect("american pv");

        assert!(american_pv.amount().is_finite());
        assert!(american_pv.amount() >= european_pv.amount());
    }

    #[test]
    fn test_bermudan_schedule_filters_invalid_dates_before_tree_pricing() {
        let as_of = date(2025, 1, 1);
        let expiry = date(2025, 7, 1);
        let mut filtered = option(expiry, OptionType::Put, ExerciseStyle::Bermudan);
        let mut noisy = option(expiry, OptionType::Put, ExerciseStyle::Bermudan);
        filtered.pricing_overrides.model_config.tree_steps = Some(51);
        noisy.pricing_overrides.model_config.tree_steps = Some(51);
        filtered.exercise_schedule = Some(vec![date(2025, 3, 1), date(2025, 5, 1)]);
        noisy.exercise_schedule = Some(vec![
            as_of,
            date(2024, 12, 15),
            date(2025, 3, 1),
            date(2025, 5, 1),
            date(2025, 8, 1),
        ]);
        let curves = market(as_of, 95.0, 0.25, 0.03, 0.0);

        let filtered_pv = compute_pv(&filtered, &curves, as_of).expect("filtered bermudan pv");
        let noisy_pv = compute_pv(&noisy, &curves, as_of).expect("noisy bermudan pv");

        assert!((filtered_pv.amount() - noisy_pv.amount()).abs() < 1e-10);
    }
}
