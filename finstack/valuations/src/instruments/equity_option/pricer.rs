//! Equity option Black–Scholes pricing engine and greeks.
//!
//! Provides deterministic PV and greeks for `EquityOption` using the
//! Black–Scholes model with continuous dividend yield. Volatility is
//! sourced from a surface (clamped) unless overridden. This mirrors the
//! structure used by `fx_option` and keeps pricing logic separate from
//! instrument definitions.

use crate::instruments::common::models::{d1, d2};
use crate::instruments::common::parameters::OptionType;
use crate::instruments::equity_option::types::EquityOption;
use finstack_core::dates::{Date, DayCount};
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::Result;

/// Trading days per year for equity options (market standard for theta calculations)
const TRADING_DAYS_PER_YEAR: f64 = 252.0;
const ONE_PERCENT: f64 = 100.0;

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

    let unit_price = price_bs_unit(spot, inst.strike.amount(), r, q, sigma, t, inst.option_type);
    Ok(Money::new(
        unit_price * inst.contract_size,
        inst.strike.currency(),
    ))
}

/// Collect standard inputs (spot, risk-free, dividend yield, vol, time to expiry).
pub fn collect_inputs(
    inst: &EquityOption,
    curves: &MarketContext,
    as_of: Date,
) -> Result<(f64, f64, f64, f64, f64)> {
    let t = year_fraction(as_of, inst.expiry, inst.day_count)?;

    // Discount curve -> zero rate
    let disc_curve = curves.get_discount_ref(inst.disc_id.as_str())?;
    let r = disc_curve.zero(t);

    // Spot from scalar id (unitless or price)
    let spot_scalar = curves.price(&inst.spot_id)?;
    let spot = match spot_scalar {
        finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
        finstack_core::market_data::scalars::MarketScalar::Price(m) => m.amount(),
    };

    // Dividend yield from scalar id if provided
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

    // Volatility from override or surface
    let sigma = if let Some(impl_vol) = inst.pricing_overrides.implied_volatility {
        impl_vol
    } else {
        let vol_surface = curves.surface_ref(inst.vol_id.as_str())?;
        vol_surface.value_clamped(t, inst.strike.amount())
    };

    Ok((spot, r, q, sigma, t))
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
    if t <= 0.0 {
        return match option_type {
            OptionType::Call => (spot - strike).max(0.0),
            OptionType::Put => (strike - spot).max(0.0),
        };
    }
    let d1 = d1(spot, strike, r, sigma, t, q);
    let d2 = d2(spot, strike, r, sigma, t, q);
    match option_type {
        OptionType::Call => {
            spot * (-q * t).exp() * finstack_core::math::norm_cdf(d1)
                - strike * (-r * t).exp() * finstack_core::math::norm_cdf(d2)
        }
        OptionType::Put => {
            strike * (-r * t).exp() * finstack_core::math::norm_cdf(-d2)
                - spot * (-q * t).exp() * finstack_core::math::norm_cdf(-d1)
        }
    }
}

/// Cash greeks for an equity option (scaled by contract size; vega per 1% vol).
#[derive(Clone, Copy, Debug, Default)]
pub struct EquityOptionGreeks {
    pub delta: f64,
    pub gamma: f64,
    pub vega: f64,
    pub theta: f64,
    pub rho: f64,
}

/// Compute greeks consistent with the pricing inputs.
pub fn compute_greeks(
    inst: &EquityOption,
    curves: &MarketContext,
    as_of: Date,
) -> Result<EquityOptionGreeks> {
    let (spot, r, q, sigma, t) = collect_inputs(inst, curves, as_of)?;

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

    let d1 = d1(spot, inst.strike.amount(), r, sigma, t, q);
    let d2 = d2(spot, inst.strike.amount(), r, sigma, t, q);
    let exp_q_t = (-q * t).exp();
    let exp_r_t = (-r * t).exp();
    let sqrt_t = t.sqrt();
    let pdf_d1 = finstack_core::math::norm_pdf(d1);
    let cdf_d1 = finstack_core::math::norm_cdf(d1);
    let cdf_m_d1 = finstack_core::math::norm_cdf(-d1);
    let cdf_d2 = finstack_core::math::norm_cdf(d2);
    let cdf_m_d2 = finstack_core::math::norm_cdf(-d2);

    let delta_unit = match inst.option_type {
        OptionType::Call => exp_q_t * cdf_d1,
        OptionType::Put => -exp_q_t * cdf_m_d1,
    };
    let gamma_unit = if sigma <= 0.0 {
        0.0
    } else {
        exp_q_t * pdf_d1 / (spot * sigma * sqrt_t)
    };
    let vega_unit = spot * exp_q_t * pdf_d1 * sqrt_t / ONE_PERCENT; // per 1% vol
    let theta_unit = match inst.option_type {
        OptionType::Call => {
            let term1 = -spot * pdf_d1 * sigma * exp_q_t / (2.0 * sqrt_t);
            let term2 = q * spot * cdf_d1 * exp_q_t;
            let term3 = -r * inst.strike.amount() * exp_r_t * cdf_d2;
            (term1 + term2 + term3) / TRADING_DAYS_PER_YEAR
        }
        OptionType::Put => {
            let term1 = -spot * pdf_d1 * sigma * exp_q_t / (2.0 * sqrt_t);
            let term2 = -q * spot * cdf_m_d1 * exp_q_t;
            let term3 = r * inst.strike.amount() * exp_r_t * cdf_m_d2;
            (term1 + term2 + term3) / TRADING_DAYS_PER_YEAR
        }
    };
    let rho_unit = match inst.option_type {
        OptionType::Call => inst.strike.amount() * t * exp_r_t * cdf_d2 / ONE_PERCENT,
        OptionType::Put => -inst.strike.amount() * t * exp_r_t * cdf_m_d2 / ONE_PERCENT,
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

/// Unit greeks (per share, not scaled by contract size).
#[derive(Clone, Copy, Debug, Default)]
pub struct UnitGreeks {
    pub delta: f64,
    pub gamma: f64,
    pub vega: f64,  // per 1% vol
    pub theta: f64, // per day
    pub rho: f64,   // per 1% rate
}

/// Compute unit greeks from explicit inputs (no market lookups).
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

    let d1 = d1(spot, strike, r, sigma, t, q);
    let d2 = d2(spot, strike, r, sigma, t, q);
    let exp_q_t = (-q * t).exp();
    let exp_r_t = (-r * t).exp();
    let sqrt_t = t.sqrt();
    let pdf_d1 = finstack_core::math::norm_pdf(d1);
    let cdf_d1 = finstack_core::math::norm_cdf(d1);
    let cdf_m_d1 = finstack_core::math::norm_cdf(-d1);
    let cdf_d2 = finstack_core::math::norm_cdf(d2);
    let cdf_m_d2 = finstack_core::math::norm_cdf(-d2);

    let delta = match option_type {
        OptionType::Call => exp_q_t * cdf_d1,
        OptionType::Put => -exp_q_t * cdf_m_d1,
    };
    let gamma = if sigma <= 0.0 {
        0.0
    } else {
        exp_q_t * pdf_d1 / (spot * sigma * sqrt_t)
    };
    let vega = spot * exp_q_t * pdf_d1 * sqrt_t / ONE_PERCENT; // per 1% vol
    let theta = match option_type {
        OptionType::Call => {
            let term1 = -spot * pdf_d1 * sigma * exp_q_t / (2.0 * sqrt_t);
            let term2 = q * spot * cdf_d1 * exp_q_t;
            let term3 = -r * strike * exp_r_t * cdf_d2;
            (term1 + term2 + term3) / TRADING_DAYS_PER_YEAR
        }
        OptionType::Put => {
            let term1 = -spot * pdf_d1 * sigma * exp_q_t / (2.0 * sqrt_t);
            let term2 = -q * spot * cdf_m_d1 * exp_q_t;
            let term3 = r * strike * exp_r_t * cdf_m_d2;
            (term1 + term2 + term3) / TRADING_DAYS_PER_YEAR
        }
    };
    let rho = match option_type {
        OptionType::Call => strike * t * exp_r_t * cdf_d2 / ONE_PERCENT,
        OptionType::Put => -strike * t * exp_r_t * cdf_m_d2 / ONE_PERCENT,
    };

    UnitGreeks {
        delta,
        gamma,
        vega,
        theta,
        rho,
    }
}

// ========================= REGISTRY PRICER =========================

/// Registry pricer for Equity Option using Black-Scholes model
pub struct SimpleEquityOptionBlackPricer {
    model: crate::pricer::ModelKey,
}

impl SimpleEquityOptionBlackPricer {
    pub fn new() -> Self {
        Self {
            model: crate::pricer::ModelKey::Black76,
        }
    }

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
        market: &finstack_core::market_data::MarketContext,
    ) -> std::result::Result<crate::results::ValuationResult, crate::pricer::PricingError> {
        use crate::instruments::common::traits::Instrument;

        // Type-safe downcasting
        let equity_option = instrument
            .as_any()
            .downcast_ref::<crate::instruments::equity_option::EquityOption>()
            .ok_or_else(|| crate::pricer::PricingError::TypeMismatch {
                expected: crate::pricer::InstrumentType::EquityOption,
                got: instrument.key(),
            })?;

        // Get as_of date from discount curve
        let disc = market
            .get_discount_ref(&equity_option.disc_id)
            .map_err(|e| crate::pricer::PricingError::ModelFailure(e.to_string()))?;
        let as_of = disc.base_date();

        // Compute present value using the engine
        let pv = npv(equity_option, market, as_of)
            .map_err(|e| crate::pricer::PricingError::ModelFailure(e.to_string()))?;

        // Return stamped result
        Ok(crate::results::ValuationResult::stamped(
            equity_option.id(),
            as_of,
            pv,
        ))
    }
}
