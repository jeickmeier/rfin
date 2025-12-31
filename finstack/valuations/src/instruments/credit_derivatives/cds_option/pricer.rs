//! Core CDS Option pricing engine and helpers.
//!
//! Provides deterministic valuation for options on CDS spreads using a
//! Black-style framework on spreads, with inputs derived from market
//! curves (discount and hazard) and an implied volatility surface.
//!
//! The engine exposes a simple `npv` function that composes the
//! forward spread, risky annuity, and discount factor into the option
//! PV via the modified Black formula implemented on the instrument.

use crate::instruments::cds::pricer::CDSPricer;
use crate::instruments::cds::{CDSConvention, CreditDefaultSwap, PayReceive};
use crate::instruments::cds_option::CdsOption;
use crate::instruments::common::models::{d1, d2, norm_cdf, norm_pdf};
use crate::instruments::common::parameters::OptionType;
use crate::instruments::common::traits::Instrument;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::ParInterp;
use finstack_core::math::solver::{BrentSolver, Solver};
use finstack_core::money::Money;
use finstack_core::Result;
use rust_decimal::Decimal;

/// Pricing engine for `CdsOption`.
///
/// Stateless wrapper that sources required market inputs and delegates
/// to the instrument's pricing math for the Black-on-spreads formula.
/// Configuration for CDS option pricing
#[derive(Clone, Debug)]
pub struct CdsOptionPricerConfig {
    /// Whether to use ISDA standard RPV01 schedule
    pub use_isda_schedule_rpv01: bool,
    /// Basis points per unit for spread conversion
    pub bp_per_unit: f64,
    /// Days per year for theta calculation
    pub theta_days_per_year: f64,
    /// Initial guess for implied volatility solver
    pub iv_initial_guess: f64,
    /// Forward rate interpolation method
    pub forward_interp: ParInterp,
}

impl Default for CdsOptionPricerConfig {
    fn default() -> Self {
        Self {
            use_isda_schedule_rpv01: true,
            bp_per_unit: 10000.0,
            theta_days_per_year: 365.0,
            iv_initial_guess: 0.20,
            forward_interp: ParInterp::Linear,
        }
    }
}

/// CDS option pricer implementing Black76 model on CDS spreads.
#[derive(Default)]
pub struct CdsOptionPricer {
    config: CdsOptionPricerConfig,
}

impl CdsOptionPricer {
    /// Price the CDS option and return its present value as of the discount curve base date.
    pub fn npv(
        &self,
        option: &CdsOption,
        curves: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> Result<Money> {
        // Time to expiry
        let t = option.day_count.year_fraction(
            as_of,
            option.expiry,
            finstack_core::dates::DayCountCtx::default(),
        )?;

        if t <= 0.0 {
            // Expired: intrinsic payoff approximation using forward = strike
            // Delegate to instrument to compute intrinsic using risky annuity below
        }

        // Market curves
        let disc = curves.get_discount(&option.discount_curve_id)?;
        let hazard = curves.get_hazard(&option.credit_curve_id)?;

        // Forward spread at CDS maturity (bp)
        let forward_spread_bp =
            self.forward_spread_from_pricer(option, disc.as_ref(), hazard.as_ref(), as_of)?;

        // Risky annuity (RPV01) from option expiry to CDS maturity via CDS pricer
        let risky_annuity =
            self.risky_annuity_from_pricer(option, disc.as_ref(), hazard.as_ref(), curves, as_of)?;

        // Discount factor to option expiry (NOT used in pricing as risky_annuity is already PV)
        // let df_expiry = disc.df(t);

        // Volatility (use override if present, else surface)
        let sigma = if let Some(vol) = option.pricing_overrides.implied_volatility {
            vol
        } else {
            curves
                .surface(option.vol_surface_id.as_str())?
                .value_clamped(t, option.strike_spread_bp)
        };

        // Price using Black-style on spreads
        // Note: risky_annuity is already PV'd to as_of, so we pass df=1.0 (or remove it from formula)
        self.credit_option_price(option, forward_spread_bp, risky_annuity, sigma, t)
    }

    /// Convenience method: compute the forward spread in bp at the underlying CDS maturity.
    pub fn forward_spread_bp(
        &self,
        option: &CdsOption,
        curves: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> Result<f64> {
        let hazard = curves.get_hazard(&option.credit_curve_id)?;
        let disc = curves.get_discount(&option.discount_curve_id)?;
        self.forward_spread_from_pricer(option, disc.as_ref(), hazard.as_ref(), as_of)
    }

    /// Convenience method: compute the risky annuity (PV of 1bp spread) from option expiry to underlying maturity.
    ///
    /// Returns the Present Value (at `as_of`) of the annuity.
    pub fn risky_annuity(
        &self,
        option: &CdsOption,
        curves: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> Result<f64> {
        let disc = curves.get_discount(&option.discount_curve_id)?;
        let hazard = curves.get_hazard(&option.credit_curve_id)?;
        self.risky_annuity_from_pricer(option, disc.as_ref(), hazard.as_ref(), curves, as_of)
    }
}

fn synthetic_underlying_cds(option: &CdsOption) -> Result<CreditDefaultSwap> {
    let spread_bp = Decimal::try_from(option.strike_spread_bp).map_err(|e| {
        finstack_core::Error::Validation(format!(
            "strike_spread_bp {} cannot be represented as Decimal: {}",
            option.strike_spread_bp, e
        ))
    })?;
    CreditDefaultSwap::new_isda(
        option.id.to_owned(),
        Money::new(option.notional.amount(), option.notional.currency()),
        PayReceive::PayFixed,
        CDSConvention::IsdaNa,
        spread_bp,
        option.expiry,
        option.cds_maturity,
        option.recovery_rate,
        option.discount_curve_id.to_owned(),
        option.credit_curve_id.to_owned(),
    )
}

impl CdsOptionPricer {
    fn forward_spread_from_pricer(
        &self,
        option: &CdsOption,
        disc: &finstack_core::market_data::term_structures::DiscountCurve,
        surv: &finstack_core::market_data::term_structures::HazardCurve,
        as_of: finstack_core::dates::Date,
    ) -> Result<f64> {
        let cds = synthetic_underlying_cds(option)?;
        let pricer = CDSPricer::new();
        let mut forward_bp = pricer.par_spread(&cds, disc, surv, as_of)?;
        if option.underlying_is_index {
            forward_bp += option.forward_spread_adjust_bp;
        }
        Ok(forward_bp)
    }

    /// Black-on-spreads price for CDS option.
    ///
    /// Note: `risky_annuity` should be the Present Value (at pricing date) of the annuity.
    /// The formula does not apply an additional discount factor.
    pub fn credit_option_price(
        &self,
        option: &CdsOption,
        forward_spread_bp: f64,
        risky_annuity: f64,
        sigma: f64,
        t: f64,
    ) -> Result<Money> {
        // Apply index-factor scale when underlying is an index (market convention)
        let scale = if option.underlying_is_index {
            option.index_factor.unwrap_or(1.0)
        } else {
            1.0
        };
        if t <= 0.0 {
            let intrinsic = match option.option_type {
                OptionType::Call => (forward_spread_bp - option.strike_spread_bp).max(0.0),
                OptionType::Put => (option.strike_spread_bp - forward_spread_bp).max(0.0),
            };
            return Ok(Money::new(
                scale * intrinsic * risky_annuity * option.notional.amount()
                    / self.config.bp_per_unit,
                option.notional.currency(),
            ));
        }

        let forward = forward_spread_bp / self.config.bp_per_unit;
        let strike = option.strike_spread_bp / self.config.bp_per_unit;
        if forward <= 0.0 || strike <= 0.0 {
            return Ok(Money::new(0.0, option.notional.currency()));
        }

        // Use models::black helpers with forward-style inputs (r=q=0)
        let d1 = d1(forward, strike, 0.0, sigma, t, 0.0);
        let d2 = d2(forward, strike, 0.0, sigma, t, 0.0);

        let value = match option.option_type {
            OptionType::Call => {
                scale
                    * risky_annuity
                    * option.notional.amount()
                    * (forward * norm_cdf(d1) - strike * norm_cdf(d2))
            }
            OptionType::Put => {
                scale
                    * risky_annuity
                    * option.notional.amount()
                    * (strike * norm_cdf(-d2) - forward * norm_cdf(-d1))
            }
        };

        Ok(Money::new(value, option.notional.currency()))
    }

    /// Delta for CDS option w.r.t. forward spread (per unit notional and bp basis handled by caller).
    ///
    /// Note: Requires `risky_annuity` (PV of annuity) to scale the result properly.
    pub fn delta(
        &self,
        option: &CdsOption,
        forward_spread_bp: f64,
        risky_annuity: f64,
        sigma: f64,
        t: f64,
    ) -> f64 {
        let scale = if option.underlying_is_index {
            option.index_factor.unwrap_or(1.0)
        } else {
            1.0
        };
        // Delta is effectively A * Delta_Black
        if t <= 0.0 || sigma <= 0.0 {
            return match option.option_type {
                OptionType::Call => {
                    if forward_spread_bp > option.strike_spread_bp {
                        scale * risky_annuity
                    } else {
                        0.0
                    }
                }
                OptionType::Put => {
                    if forward_spread_bp < option.strike_spread_bp {
                        -scale * risky_annuity
                    } else {
                        0.0
                    }
                }
            };
        }
        let forward = forward_spread_bp / self.config.bp_per_unit;
        let strike = option.strike_spread_bp / self.config.bp_per_unit;
        if forward <= 0.0 || strike <= 0.0 {
            return 0.0;
        }
        let d1 = d1(forward, strike, 0.0, sigma, t, 0.0);
        match option.option_type {
            OptionType::Call => scale * risky_annuity * norm_cdf(d1),
            OptionType::Put => -scale * risky_annuity * norm_cdf(-d1),
        }
    }

    /// Gamma per bp of spread.
    pub fn gamma(
        &self,
        option: &CdsOption,
        forward_spread_bp: f64,
        risky_annuity: f64,
        sigma: f64,
        t: f64,
    ) -> f64 {
        let scale = if option.underlying_is_index {
            option.index_factor.unwrap_or(1.0)
        } else {
            1.0
        };
        if t <= 0.0 || sigma <= 0.0 {
            return 0.0;
        }
        let forward = forward_spread_bp / self.config.bp_per_unit;
        let strike = option.strike_spread_bp / self.config.bp_per_unit;
        if forward <= 0.0 || strike <= 0.0 {
            return 0.0;
        }
        let d1 = d1(forward, strike, 0.0, sigma, t, 0.0);
        scale * risky_annuity * norm_pdf(d1)
            / (forward * self.config.bp_per_unit * sigma * t.sqrt())
    }

    /// Vega per 1% vol change.
    pub fn vega(
        &self,
        option: &CdsOption,
        forward_spread_bp: f64,
        risky_annuity: f64,
        sigma: f64,
        t: f64,
    ) -> f64 {
        let scale = if option.underlying_is_index {
            option.index_factor.unwrap_or(1.0)
        } else {
            1.0
        };
        if t <= 0.0 {
            return 0.0;
        }
        let forward = forward_spread_bp / self.config.bp_per_unit;
        let strike = option.strike_spread_bp / self.config.bp_per_unit;
        if forward <= 0.0 || strike <= 0.0 {
            return 0.0;
        }
        let d1 = if sigma > 0.0 {
            d1(forward, strike, 0.0, sigma, t, 0.0)
        } else {
            0.0
        };
        scale * risky_annuity * forward * self.config.bp_per_unit * norm_pdf(d1) * t.sqrt() / 100.0
    }

    /// Analytical theta per day using Black-76 formula.
    ///
    /// # Note
    ///
    /// This is the analytical approximation that only captures the Black formula
    /// time decay (dBlack/dt) but not the risky annuity decay (dA/dt).
    /// For complete theta including annuity decay, use [`theta_finite_diff`](Self::theta_finite_diff).
    pub fn theta_analytical(
        &self,
        option: &CdsOption,
        forward_spread_bp: f64,
        risky_annuity: f64,
        r: f64,
        sigma: f64,
        t: f64,
    ) -> f64 {
        let scale = if option.underlying_is_index {
            option.index_factor.unwrap_or(1.0)
        } else {
            1.0
        };
        if t <= 0.0 {
            return 0.0;
        }
        let forward = forward_spread_bp / self.config.bp_per_unit;
        let strike = option.strike_spread_bp / self.config.bp_per_unit;
        if forward <= 0.0 || strike <= 0.0 {
            return 0.0;
        }
        let d1 = d1(forward, strike, 0.0, sigma, t, 0.0);
        let d2 = d2(forward, strike, 0.0, sigma, t, 0.0);
        let sqrt_t = t.sqrt();

        // Theta = dV/dt.
        // V = A * Black
        // dV/dt = dA/dt * Black + A * dBlack/dt
        // This analytical version only computes A * dBlack/dt.
        // For complete theta, use theta_finite_diff.

        match option.option_type {
            OptionType::Call => {
                let term1 = -forward * norm_pdf(d1) * sigma / (2.0 * sqrt_t);
                let term2 = -r * strike * (-r * t).exp() * norm_cdf(d2);
                scale * risky_annuity * (term1 + term2) * self.config.bp_per_unit
                    / self.config.theta_days_per_year
            }
            OptionType::Put => {
                let term1 = -forward * norm_pdf(d1) * sigma / (2.0 * sqrt_t);
                let term2 = r * strike * (-r * t).exp() * norm_cdf(-d2);
                scale * risky_annuity * (term1 + term2) * self.config.bp_per_unit
                    / self.config.theta_days_per_year
            }
        }
    }

    /// Finite-difference theta (complete, including risky annuity decay).
    ///
    /// Computes theta by repricing the option with a 1-day time shift.
    /// This captures both the Black formula time decay and the risky annuity decay,
    /// providing a complete theta measure.
    ///
    /// # Formula
    ///
    /// θ = (V(t - dt) - V(t)) / dt
    ///
    /// where dt = 1/365 (one calendar day).
    ///
    /// # Returns
    ///
    /// The dollar value change per day (typically negative for long positions).
    pub fn theta_finite_diff(
        &self,
        option: &CdsOption,
        curves: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> Result<f64> {
        use time::Duration;

        // Time step: 1 calendar day
        let dt_years = 1.0 / self.config.theta_days_per_year;

        // Check if already expired
        let t = option.day_count.year_fraction(
            as_of,
            option.expiry,
            finstack_core::dates::DayCountCtx::default(),
        )?;
        if t <= dt_years {
            return Ok(0.0);
        }

        // Base PV at as_of
        let pv_base = self.npv(option, curves, as_of)?.amount();

        // Shifted PV at as_of + 1 day (moving forward in time reduces time to expiry)
        let as_of_shifted = as_of + Duration::days(1);
        let pv_shifted = self.npv(option, curves, as_of_shifted)?.amount();

        // Theta = (PV_shifted - PV_base) / dt
        // Since we moved forward in time by 1 day, this gives daily theta
        let theta = pv_shifted - pv_base;

        Ok(theta)
    }

    fn risky_annuity_from_pricer(
        &self,
        option: &CdsOption,
        disc: &finstack_core::market_data::term_structures::DiscountCurve,
        surv: &finstack_core::market_data::term_structures::HazardCurve,
        _curves: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> Result<f64> {
        if !self.config.use_isda_schedule_rpv01 {
            // Fallback to simple approximation if ever disabled
            let cds_tenor = option.day_count.year_fraction(
                option.expiry,
                option.cds_maturity,
                finstack_core::dates::DayCountCtx::default(),
            )?;
            let t0 = option.day_count.year_fraction(
                as_of,
                option.expiry,
                finstack_core::dates::DayCountCtx::default(),
            )?;
            let mut ann = 0.0;
            let n = (cds_tenor * 4.0).ceil() as usize;
            for i in 1..=n {
                let tau = cds_tenor * (i as f64) / (n as f64);
                ann += 0.25 * disc.df(t0 + tau) * surv.sp(t0 + tau);
            }
            return Ok(ann);
        }

        // Market-standard: build a synthetic CDS from expiry to maturity and ask CDS pricer for RA
        let mut cds = synthetic_underlying_cds(option)?;
        cds.premium.spread_bp = Decimal::ZERO;
        let cds_pricer = CDSPricer::new();
        cds_pricer.risky_annuity(&cds, disc, surv, as_of)
    }

    /// Solve for implied volatility σ such that model price(σ) = target_price.
    ///
    /// Uses log-σ parameterization to enforce positivity and HybridSolver (Newton + Brent fallback).
    pub fn implied_vol(
        &self,
        option: &CdsOption,
        curves: &MarketContext,
        as_of: finstack_core::dates::Date,
        target_price: f64,
        initial_guess: Option<f64>,
    ) -> Result<f64> {
        // Pre-compute market inputs independent of σ
        let t = option.day_count.year_fraction(
            as_of,
            option.expiry,
            finstack_core::dates::DayCountCtx::default(),
        )?;
        if t <= 0.0 {
            return Ok(0.0);
        }

        let disc = curves.get_discount(&option.discount_curve_id)?;
        let hazard = curves.get_hazard(&option.credit_curve_id)?;

        // Forward spread at CDS maturity (bp)
        let fwd_bp =
            self.forward_spread_from_pricer(option, disc.as_ref(), hazard.as_ref(), as_of)?;

        // Risky annuity from expiry to maturity
        let ra =
            self.risky_annuity_from_pricer(option, disc.as_ref(), hazard.as_ref(), curves, as_of)?;
        // df_expiry not needed as ra is already PV

        // Objective in log-σ space to keep σ>0
        let price_for_sigma = |sigma: f64| -> f64 {
            match self.credit_option_price(option, fwd_bp, ra, sigma, t) {
                Ok(m) => m.amount(),
                Err(_) => f64::NAN,
            }
        };

        let target = target_price;
        let f = |x: f64| -> f64 {
            let sigma = x.exp();
            price_for_sigma(sigma) - target
        };

        // Initial guess: override vol, else surface vol, else config default
        let sigma0 = if let Some(v) = option.pricing_overrides.implied_volatility {
            v
        } else {
            curves
                .surface(option.vol_surface_id.as_str())
                .ok()
                .map(|s| s.value_clamped(t, option.strike_spread_bp))
                .unwrap_or(self.config.iv_initial_guess)
        };
        let x0 = (initial_guess.unwrap_or(sigma0.max(1e-6))).ln();

        let solver = BrentSolver::new().with_tolerance(1e-10);
        let root = solver.solve(f, x0)?;
        Ok(root.exp())
    }
}

// ========================= REGISTRY PRICER =========================

/// Registry pricer for CDS Option using Black76 model
pub struct SimpleCdsOptionBlackPricer {
    model_key: crate::pricer::ModelKey,
}

impl SimpleCdsOptionBlackPricer {
    /// Create a new CDS option pricer with default Black76 model
    pub fn new() -> Self {
        Self::with_model(crate::pricer::ModelKey::Black76)
    }

    /// Create a CDS option pricer with specified model key
    pub fn with_model(model_key: crate::pricer::ModelKey) -> Self {
        Self { model_key }
    }
}

impl Default for SimpleCdsOptionBlackPricer {
    fn default() -> Self {
        Self::new()
    }
}

impl crate::pricer::Pricer for SimpleCdsOptionBlackPricer {
    fn key(&self) -> crate::pricer::PricerKey {
        crate::pricer::PricerKey::new(crate::pricer::InstrumentType::CDSOption, self.model_key)
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> std::result::Result<crate::results::ValuationResult, crate::pricer::PricingError> {
        use crate::instruments::common::traits::Instrument;

        // Type-safe downcasting
        let cds_option = instrument
            .as_any()
            .downcast_ref::<crate::instruments::cds_option::CdsOption>()
            .ok_or_else(|| {
                crate::pricer::PricingError::type_mismatch(
                    crate::pricer::InstrumentType::CDSOption,
                    instrument.key(),
                )
            })?;

        // Use the provided as_of date for valuation
        // Compute present value using the engine
        let pv = CdsOptionPricer::default()
            .npv(cds_option, market, as_of)
            .map_err(|e| crate::pricer::PricingError::model_failure_ctx(e.to_string(), crate::pricer::PricingErrorContext::default()))?;

        // Return stamped result
        Ok(crate::results::ValuationResult::stamped(
            cds_option.id(),
            as_of,
            pv,
        ))
    }
}
