//! Equity option Black–Scholes pricing engine and greeks.
//!
//! Provides deterministic PV and greeks for `EquityOption` using the
//! Black–Scholes model with continuous dividend yield. Volatility is
//! sourced from a surface (clamped) unless overridden. This mirrors the
//! structure used by `fx_option` and keeps pricing logic separate from
//! instrument definitions.
// Allow dead_code for public API items exposed via Python (finstack-py) and WASM bindings.
// Key items: npv, compute_greeks, EquityOptionGreeks, SimpleEquityOptionBlackPricer.
#![allow(dead_code)]

use crate::instruments::common::models::trees::binomial_tree::BinomialTree;
use crate::instruments::common::models::{bs_greeks, bs_price, BsGreeks};
use crate::instruments::common::parameters::{OptionMarketParams, OptionType};
use crate::instruments::equity_option::types::EquityOption;
use crate::instruments::ExerciseStyle;
use finstack_core::dates::{Date, DayCount};
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_core::Result;

/// Trading days per year for equity options (market standard for theta calculations)
const TRADING_DAYS_PER_YEAR: f64 = 252.0;

/// Present value using Black–Scholes; result currency is the strike currency.
pub fn npv(inst: &EquityOption, curves: &MarketContext, as_of: Date) -> Result<Money> {
    let (spot, r, q, sigma, t) = collect_inputs(inst, curves, as_of)?;

    if t <= 0.0 {
        // Expired: intrinsic value scaled by contract size
        let intrinsic = match inst.option_type {
            OptionType::Call => (spot - inst.strike.amount()).max(0.0),
            OptionType::Put => (inst.strike.amount() - spot).max(0.0),
        };
        return Ok(Money::new(
            intrinsic * inst.contract_size,
            inst.strike.currency(),
        ));
    }

    // Dispatch based on exercise style
    let unit_price = match inst.exercise_style {
        ExerciseStyle::European => {
            price_bs_unit(spot, inst.strike.amount(), r, q, sigma, t, inst.option_type)
        }
        ExerciseStyle::American => {
            // Use Leisen-Reimer tree for American options
            // 201 steps gives good accuracy/performance trade-off (~10c precision)
            let tree = BinomialTree::leisen_reimer(201);
            let params = OptionMarketParams {
                spot,
                strike: inst.strike.amount(),
                rate: r,
                dividend_yield: q,
                volatility: sigma,
                time_to_expiry: t,
                option_type: inst.option_type,
            };
            tree.price_american(&params)?
        }
        ExerciseStyle::Bermudan => {
            return Err(finstack_core::Error::Validation(
                "Bermudan equity option requires an exercise schedule; pricing not supported yet"
                    .to_string(),
            ));
        }
    };

    Ok(Money::new(
        unit_price * inst.contract_size,
        inst.strike.currency(),
    ))
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
    pub t_rate: f64,
    /// Time to expiry for vol calculations (ACT/365F standard)
    pub t_vol: f64,
}

impl EquityOptionInputs {
    /// Returns the pricing time (t_vol for consistency with Black-Scholes)
    #[allow(dead_code)] // May be used by external bindings or tests
    #[inline]
    pub fn t(&self) -> f64 {
        self.t_vol
    }
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
    // For backwards compatibility, return t_vol as the primary time
    Ok((inputs.spot, inputs.r, inputs.q, inputs.sigma, inputs.t_vol))
}

/// Collect inputs with separate rate and vol time fractions.
///
/// Returns `EquityOptionInputs` with properly separated day count handling:
/// - `t_rate`: Uses the discount curve's day count for rate lookups
/// - `t_vol`: Uses ACT/365F for volatility surface lookups (equity market standard)
pub fn collect_inputs_extended(
    inst: &EquityOption,
    curves: &MarketContext,
    as_of: Date,
) -> Result<EquityOptionInputs> {
    // Discount curve lookup - use curve's own day count for discount factor time
    let disc_curve = curves.get_discount(inst.discount_curve_id.as_str())?;
    let curve_dc = disc_curve.day_count();
    let t_rate = year_fraction(as_of, inst.expiry, curve_dc)?;
    let df = disc_curve.df(t_rate);

    // Vol time uses ACT/365F (equity market standard for vol surfaces)
    // This is consistent with how equity volatility is quoted in the market
    let t_vol = year_fraction(as_of, inst.expiry, DayCount::Act365F)?;
    let r = if t_vol > 0.0 { -df.ln() / t_vol } else { 0.0 };

    // Spot from scalar id (unitless or price)
    let spot_scalar = curves.price(&inst.spot_id)?;
    let spot = match spot_scalar {
        finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
        finstack_core::market_data::scalars::MarketScalar::Price(m) => m.amount(),
    };

    // Dividend yield from scalar id if provided
    //
    // When a dividend yield ID is explicitly provided, we require the lookup to succeed
    // and return a unitless scalar. Silent fallback to 0.0 would mask market data
    // configuration errors.
    let q = if let Some(div_id) = &inst.div_yield_id {
        let ms = curves.price(div_id.as_str()).map_err(|e| {
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

    // Volatility from override or surface (using t_vol for surface lookup)
    let sigma = if let Some(impl_vol) = inst.pricing_overrides.implied_volatility {
        impl_vol
    } else {
        let vol_surface = curves.surface(inst.vol_surface_id.as_str())?;
        vol_surface.value_clamped(t_vol, inst.strike.amount())
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

/// Year fraction helper using instrument day-count.
#[inline]
pub fn year_fraction(start: Date, end: Date, dc: DayCount) -> Result<f64> {
    dc.year_fraction(start, end, finstack_core::dates::DayCountCtx::default())
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
#[derive(Clone, Copy, Debug, Default)]
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
        let spot_gt_strike = spot > inst.strike.amount();
        let delta_unit = match inst.option_type {
            OptionType::Call => {
                if spot_gt_strike {
                    1.0
                } else {
                    0.0
                }
            }
            OptionType::Put => {
                if !spot_gt_strike {
                    -1.0
                } else {
                    0.0
                }
            }
        };
        let scale = inst.contract_size;
        return Ok(EquityOptionGreeks {
            delta: delta_unit * scale,
            ..Default::default()
        });
    }

    match inst.exercise_style {
        ExerciseStyle::European => {
            let greeks_unit = bs_greeks(
                spot,
                inst.strike.amount(),
                r,
                q,
                sigma,
                t,
                inst.option_type,
                TRADING_DAYS_PER_YEAR,
            );
            let scale = inst.contract_size;
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
            let tree = BinomialTree::leisen_reimer(201);
            let params = OptionMarketParams {
                spot,
                strike: inst.strike.amount(),
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

            // Theta (1 day bump)
            let dt = 1.0 / 365.25;
            let theta_unit = if t > dt {
                let mut p_t = params.clone();
                p_t.time_to_expiry -= dt;
                let price_t = price_fn(&p_t)?;
                price_t - base_price // change per day
            } else {
                0.0
            };

            let scale = inst.contract_size;
            Ok(EquityOptionGreeks {
                delta: delta_unit * scale,
                gamma: gamma_unit * scale,
                vega: vega_unit * scale,
                theta: theta_unit * scale,
                rho: rho_unit * scale,
            })
        }
        ExerciseStyle::Bermudan => Err(finstack_core::Error::Validation(
            "Bermudan equity option requires an exercise schedule; greeks not supported yet"
                .to_string(),
        )),
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
        let delta = match option_type {
            OptionType::Call => {
                if spot > strike {
                    1.0
                } else {
                    0.0
                }
            }
            OptionType::Put => {
                if spot < strike {
                    -1.0
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
    /// Create new Black-Scholes pricer with default model
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
        instrument: &dyn crate::instruments::common::traits::Instrument,
        market: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> std::result::Result<crate::results::ValuationResult, crate::pricer::PricingError> {
        use crate::instruments::common::traits::Instrument;

        // Type-safe downcasting
        let equity_option = instrument
            .as_any()
            .downcast_ref::<crate::instruments::equity_option::EquityOption>()
            .ok_or_else(|| {
                crate::pricer::PricingError::type_mismatch(
                    crate::pricer::InstrumentType::EquityOption,
                    instrument.key(),
                )
            })?;

        // Use the provided as_of date for consistency
        // Compute present value using the engine
        let pv = npv(equity_option, market, as_of).map_err(|e| {
            crate::pricer::PricingError::model_failure_ctx(
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
use crate::instruments::common::models::closed_form::heston::{
    heston_call_price_fourier, heston_put_price_fourier, HestonParams,
};
#[cfg(feature = "mc")]
use crate::instruments::common::traits::Instrument;

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
        instrument: &dyn crate::instruments::common::traits::Instrument,
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
            crate::pricer::PricingError::model_failure_ctx(
                e.to_string(),
                crate::pricer::PricingErrorContext::default(),
            )
        })?;
        let (spot, r, q, _sigma, t) = (inputs.spot, inputs.r, inputs.q, inputs.sigma, inputs.t_vol);

        if t <= 0.0 {
            let intrinsic = match equity_option.option_type {
                OptionType::Call => (spot - equity_option.strike.amount()).max(0.0),
                OptionType::Put => (equity_option.strike.amount() - spot).max(0.0),
            };
            return Ok(crate::results::ValuationResult::stamped(
                equity_option.id(),
                as_of,
                Money::new(
                    intrinsic * equity_option.contract_size,
                    equity_option.strike.currency(),
                ),
            ));
        }

        // Fetch Heston parameters from market data or use defaults
        // Priority: instrument overrides > market scalars > defaults
        let kappa = market
            .price("HESTON_KAPPA")
            .ok()
            .and_then(|s| match s {
                finstack_core::market_data::scalars::MarketScalar::Unitless(v) => Some(*v),
                _ => None,
            })
            .unwrap_or(2.0);

        let theta = market
            .price("HESTON_THETA")
            .ok()
            .and_then(|s| match s {
                finstack_core::market_data::scalars::MarketScalar::Unitless(v) => Some(*v),
                _ => None,
            })
            .unwrap_or(0.04);

        let sigma_v = market
            .price("HESTON_SIGMA_V")
            .ok()
            .and_then(|s| match s {
                finstack_core::market_data::scalars::MarketScalar::Unitless(v) => Some(*v),
                _ => None,
            })
            .unwrap_or(0.3);

        let rho = market
            .price("HESTON_RHO")
            .ok()
            .and_then(|s| match s {
                finstack_core::market_data::scalars::MarketScalar::Unitless(v) => Some(*v),
                _ => None,
            })
            .unwrap_or(-0.7);

        let v0 = market
            .price("HESTON_V0")
            .ok()
            .and_then(|s| match s {
                finstack_core::market_data::scalars::MarketScalar::Unitless(v) => Some(*v),
                _ => None,
            })
            .unwrap_or(0.04);

        let params = HestonParams::new(r, q, kappa, theta, sigma_v, rho, v0);

        let price = match equity_option.option_type {
            OptionType::Call => {
                heston_call_price_fourier(spot, equity_option.strike.amount(), t, &params)
            }
            OptionType::Put => {
                heston_put_price_fourier(spot, equity_option.strike.amount(), t, &params)
            }
        };

        let pv = Money::new(
            price * equity_option.contract_size,
            equity_option.strike.currency(),
        );
        Ok(crate::results::ValuationResult::stamped(
            equity_option.id(),
            as_of,
            pv,
        ))
    }
}
