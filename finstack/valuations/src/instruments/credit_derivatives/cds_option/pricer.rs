//! Core CDS Option pricing engine and helpers.
//!
//! Provides deterministic valuation for options on CDS spreads using a
//! Black-style framework on spreads, with inputs derived from market
//! curves (discount and hazard) and an implied volatility surface.
//!
//! The engine exposes a simple `npv` function that composes the
//! forward spread, risky annuity, and discount factor into the option
//! PV via the modified Black formula implemented on the instrument.

use crate::constants::credit;
use crate::instruments::common_impl::models::{d1, d2, norm_cdf, norm_pdf};
use crate::instruments::common_impl::parameters::OptionType;
use crate::instruments::common_impl::traits::Instrument;
use crate::instruments::credit_derivatives::cds::pricer::CDSPricer;
use crate::instruments::credit_derivatives::cds::{CreditDefaultSwap, PayReceive};
use crate::instruments::credit_derivatives::cds_option::CDSOption;
use finstack_core::market_data::context::MarketContext;
use finstack_core::math::solver::{BrentSolver, Solver};
use finstack_core::money::Money;
use finstack_core::Result;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;

fn decimal_to_f64(value: Decimal, field: &str) -> Result<f64> {
    value.to_f64().ok_or_else(|| {
        finstack_core::Error::Validation(format!(
            "CDS option {field} ({value}) cannot be converted to f64"
        ))
    })
}

/// Pricing engine for `CDSOption`.
///
/// Stateless wrapper that sources required market inputs and delegates
/// to the instrument's pricing math for the Black-on-spreads formula.
/// Configuration for CDS option pricing
#[derive(Debug, Clone)]
pub(crate) struct CDSOptionPricerConfig {
    /// Whether to use ISDA standard RPV01 schedule
    pub(crate) use_isda_schedule_rpv01: bool,
    /// Basis points per unit for spread conversion
    pub(crate) bp_per_unit: f64,
    /// Days per year for theta calculation
    pub(crate) theta_days_per_year: f64,
    /// Initial guess for implied volatility solver
    pub(crate) iv_initial_guess: f64,
}

impl Default for CDSOptionPricerConfig {
    fn default() -> Self {
        Self {
            use_isda_schedule_rpv01: true,
            bp_per_unit: 10000.0,
            theta_days_per_year: 365.0,
            iv_initial_guess: 0.20,
        }
    }
}

/// CDS option pricer implementing Black76 model on CDS spreads.
#[derive(Default)]
pub(crate) struct CDSOptionPricer {
    config: CDSOptionPricerConfig,
}

impl CDSOptionPricer {
    /// Price the CDS option and return its present value as of the discount curve base date.
    pub(crate) fn npv(
        &self,
        option: &CDSOption,
        curves: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> Result<Money> {
        option.validate_supported_configuration()?;
        // Time to expiry
        let t = option.day_count.year_fraction(
            as_of,
            option.expiry,
            finstack_core::dates::DayCountContext::default(),
        )?;

        // Note: t <= 0 (expired) is handled downstream in credit_option_price
        // which computes the intrinsic payoff.

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
        let sigma = if let Some(vol) = option.pricing_overrides.market_quotes.implied_volatility {
            vol
        } else {
            let strike_f64 = decimal_to_f64(option.strike, "strike")?;
            curves
                .get_surface(option.vol_surface_id.as_str())?
                .value_clamped(t, strike_f64)
        };

        // Price using Black-style on spreads
        // Note: risky_annuity is already PV'd to as_of, so we pass df=1.0 (or remove it from formula)
        let black_pv =
            self.credit_option_price(option, forward_spread_bp, risky_annuity, sigma, t)?;

        // Front-End Protection (FEP) adjustment for index payer options.
        //
        // Standard market convention per Pedersen (2003) and O'Kane (2008)
        // Ch. 12: payer options on a defaultable CDS index receive a cash
        // benefit equal to the protection payout for names that default
        // between `as_of` and the option expiry. Receiver options knock
        // out on defaulted names so do not receive FEP.
        //
        // FEP_PV = ∫_0^{T_exp} (1 − R) · h(s) · S(s) · DF(0, s) ds
        //
        // Scaled by the option notional and the index factor. This PV is
        // added to the Black-on-spreads value for payer (Call) index
        // options only — see `front_end_protection_pv` for numerics.
        let fep_pv = if option.underlying_is_index
            && matches!(option.option_type, OptionType::Call)
            && t > 0.0
        {
            let scale = option.index_factor.unwrap_or(1.0);
            scale
                * option.notional.amount()
                * Self::front_end_protection_pv(
                    disc.as_ref(),
                    hazard.as_ref(),
                    t,
                    option.recovery_rate,
                )
        } else {
            0.0
        };

        if fep_pv.abs() > 0.0 {
            let ccy = black_pv.currency();
            Ok(Money::new(black_pv.amount() + fep_pv, ccy))
        } else {
            Ok(black_pv)
        }
    }

    /// Front-End Protection PV per unit notional for a CDS index payer
    /// option expiring at time `t_expiry` (year-fraction from `as_of`).
    ///
    /// Implements the standard integral
    ///
    /// ```text
    /// FEP/N = (1 − R) · ∫_0^{T_exp} h(s) · S(s) · DF(0, s) ds
    /// ```
    ///
    /// via a mid-point quadrature with 40 sub-intervals. For typical index
    /// option expiries (T ≤ 2y) on investment-grade curves (h ≈ 1–3%) this
    /// has absolute quadrature error below 0.1 bp of upfront — well inside
    /// the Black-model uncertainty on σ and R.
    ///
    /// **Validity regime**: the 0.1 bp bound was characterised for
    /// `T_expiry ≤ 2y` AND `h(s) ≤ 3%` over the integration window (i.e. CDX.IG
    /// or iTraxx Main). For HY indices (CDX.HY, iTraxx Crossover) where local
    /// hazard can hit 5–10%, or expiries beyond ~3y, the integrand develops
    /// non-trivial curvature and the fixed-step midpoint rule degrades. In
    /// those regimes consider raising `STEPS` or moving to an adaptive scheme;
    /// the current constant is sized for the IG single-name and main-index use
    /// case which dominates trading volume.
    ///
    /// # References
    /// - Pedersen, C. (2003). "Valuation of Portfolio Credit Default
    ///   Swaptions." Lehman Brothers.
    /// - O'Kane, D. (2008). *Modelling Single-name and Multi-name Credit
    ///   Derivatives*. Wiley. Ch. 12.
    fn front_end_protection_pv(
        disc: &finstack_core::market_data::term_structures::DiscountCurve,
        hazard: &finstack_core::market_data::term_structures::HazardCurve,
        t_expiry: f64,
        recovery: f64,
    ) -> f64 {
        if t_expiry <= 0.0 {
            return 0.0;
        }
        const STEPS: usize = 40;
        let dt = t_expiry / STEPS as f64;
        let mut integral = 0.0_f64;
        for i in 0..STEPS {
            let t_mid = (i as f64 + 0.5) * dt;
            let h = hazard.hazard_rate(t_mid);
            let s = hazard.sp(t_mid);
            let df = disc.df(t_mid);
            integral += h * s * df * dt;
        }
        (1.0 - recovery) * integral
    }

    /// Convenience method: compute the forward spread in bp at the underlying CDS maturity.
    pub(crate) fn forward_spread_bp(
        &self,
        option: &CDSOption,
        curves: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> Result<f64> {
        option.validate_supported_configuration()?;
        let hazard = curves.get_hazard(&option.credit_curve_id)?;
        let disc = curves.get_discount(&option.discount_curve_id)?;
        self.forward_spread_from_pricer(option, disc.as_ref(), hazard.as_ref(), as_of)
    }

    /// Convenience method: compute the risky annuity (PV of 1bp spread) from option expiry to underlying maturity.
    ///
    /// Returns the Present Value (at `as_of`) of the annuity.
    pub(crate) fn risky_annuity(
        &self,
        option: &CDSOption,
        curves: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> Result<f64> {
        option.validate_supported_configuration()?;
        let disc = curves.get_discount(&option.discount_curve_id)?;
        let hazard = curves.get_hazard(&option.credit_curve_id)?;
        self.risky_annuity_from_pricer(option, disc.as_ref(), hazard.as_ref(), curves, as_of)
    }
}

fn synthetic_underlying_cds(option: &CDSOption) -> Result<CreditDefaultSwap> {
    use crate::instruments::credit_derivatives::cds::CdsValuationConvention;

    // Convert decimal strike to bp for CDS constructor (which expects bp)
    let spread_bp = option.strike * Decimal::new(10000, 0);
    let mut cds = CreditDefaultSwap::new_isda(
        option.id.to_owned(),
        Money::new(option.notional.amount(), option.notional.currency()),
        PayReceive::PayFixed,
        option.underlying_convention,
        spread_bp,
        option.expiry,
        option.cds_maturity,
        option.recovery_rate,
        option.discount_curve_id.to_owned(),
        option.credit_curve_id.to_owned(),
    )?;
    // The CDSO Black-76 forward spread / risky annuity model is anchored on
    // the ISDA Standard CDS Model conventions (unadjusted IMM accruals,
    // dirty PV). Pin the synthetic underlying explicitly so that the option
    // pricer is invariant to the default `CreditDefaultSwap` convention.
    cds.valuation_convention = CdsValuationConvention::IsdaDirty;
    Ok(cds)
}

impl CDSOptionPricer {
    fn forward_spread_from_pricer(
        &self,
        option: &CDSOption,
        disc: &finstack_core::market_data::term_structures::DiscountCurve,
        surv: &finstack_core::market_data::term_structures::HazardCurve,
        as_of: finstack_core::dates::Date,
    ) -> Result<f64> {
        let cds = synthetic_underlying_cds(option)?;
        let pricer = CDSPricer::new();
        let mut forward_bp = pricer.par_spread(&cds, disc, surv, as_of)?;
        if option.underlying_is_index {
            forward_bp += decimal_to_f64(option.forward_spread_adjust, "forward_spread_adjust")?
                * self.config.bp_per_unit;
        }
        Ok(forward_bp)
    }

    /// Black-on-spreads price for CDS option.
    ///
    /// Note: `risky_annuity` should be the Present Value (at pricing date) of the annuity.
    /// The formula does not apply an additional discount factor.
    ///
    /// # Numerical Stability
    ///
    /// When the forward spread is below `credit::MIN_FORWARD_SPREAD`, the Black formula's
    /// log(forward/strike) becomes numerically unstable. In this case, the option value
    /// is returned as zero (the forward is effectively at or below zero).
    pub(crate) fn credit_option_price(
        &self,
        option: &CDSOption,
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
        let strike = decimal_to_f64(option.strike, "strike")?;

        if t <= 0.0 {
            let forward = forward_spread_bp / self.config.bp_per_unit;
            let intrinsic = match option.option_type {
                OptionType::Call => (forward - strike).max(0.0),
                OptionType::Put => (strike - forward).max(0.0),
            };
            return Ok(Money::new(
                scale * intrinsic * risky_annuity * option.notional.amount(),
                option.notional.currency(),
            ));
        }

        let forward = forward_spread_bp / self.config.bp_per_unit;

        // Guard against invalid strike
        if strike <= 0.0 {
            return Ok(Money::new(0.0, option.notional.currency()));
        }

        // Guard against numerically unstable forward values.
        // The Black formula requires positive forward for log(F/K).
        // When forward → 0:
        //   - Call (payer): worthless, return 0
        //   - Put (receiver): converges to K × RPV01 × Notional (full intrinsic)
        if forward < credit::MIN_FORWARD_SPREAD {
            return match option.option_type {
                OptionType::Put => Ok(Money::new(
                    scale * strike * risky_annuity * option.notional.amount(),
                    option.notional.currency(),
                )),
                OptionType::Call => Ok(Money::new(0.0, option.notional.currency())),
            };
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

    /// Delta for CDS option w.r.t. forward spread (per unit spread, i.e., per 100%).
    ///
    /// **WARNING**: Returns sensitivity per *decimal* spread, not per basis point.
    /// For a per-bp delta, divide by `bp_per_unit` (10,000).
    ///
    /// Note: Requires `risky_annuity` (PV of annuity) to scale the result properly.
    pub(crate) fn delta(
        &self,
        option: &CDSOption,
        forward_spread_bp: f64,
        risky_annuity: f64,
        sigma: f64,
        t: f64,
    ) -> Result<f64> {
        let scale = if option.underlying_is_index {
            option.index_factor.unwrap_or(1.0)
        } else {
            1.0
        };
        let strike = decimal_to_f64(option.strike, "strike")?;

        if t <= 0.0 || sigma <= 0.0 {
            let forward = forward_spread_bp / self.config.bp_per_unit;
            return Ok(match option.option_type {
                OptionType::Call => {
                    if forward > strike {
                        scale * risky_annuity
                    } else {
                        0.0
                    }
                }
                OptionType::Put => {
                    if forward < strike {
                        -scale * risky_annuity
                    } else {
                        0.0
                    }
                }
            });
        }
        let forward = forward_spread_bp / self.config.bp_per_unit;
        if forward <= 0.0 || strike <= 0.0 {
            return Ok(0.0);
        }
        let d1 = d1(forward, strike, 0.0, sigma, t, 0.0);
        Ok(match option.option_type {
            OptionType::Call => scale * risky_annuity * norm_cdf(d1),
            OptionType::Put => -scale * risky_annuity * norm_cdf(-d1),
        })
    }

    /// Gamma per unit spread squared (i.e., ∂²V/∂S² where S is the decimal spread).
    ///
    /// **WARNING**: Returns sensitivity per *decimal* spread squared, not per bp squared.
    /// For a per-bp² gamma, divide by `bp_per_unit²` (10⁸).
    ///
    /// Returns 0.0 when time-to-expiry or volatility are too small for stable
    /// numerical calculation (denominator approaches zero).
    pub(crate) fn gamma(
        &self,
        option: &CDSOption,
        forward_spread_bp: f64,
        risky_annuity: f64,
        sigma: f64,
        t: f64,
    ) -> Result<f64> {
        let scale = if option.underlying_is_index {
            option.index_factor.unwrap_or(1.0)
        } else {
            1.0
        };
        let strike = decimal_to_f64(option.strike, "strike")?;

        if t < credit::MIN_TIME_TO_EXPIRY_GREEKS || sigma < credit::MIN_VOLATILITY_GREEKS {
            return Ok(0.0);
        }
        let forward = forward_spread_bp / self.config.bp_per_unit;
        if forward <= 0.0 || strike <= 0.0 {
            return Ok(0.0);
        }
        let d1 = d1(forward, strike, 0.0, sigma, t, 0.0);
        Ok(scale * risky_annuity * norm_pdf(d1) / (forward * sigma * t.sqrt()))
    }

    /// Vega per 1% vol change.
    ///
    /// Returns 0.0 when time-to-expiry is too small for stable calculation.
    pub(crate) fn vega(
        &self,
        option: &CDSOption,
        forward_spread_bp: f64,
        risky_annuity: f64,
        sigma: f64,
        t: f64,
    ) -> Result<f64> {
        let scale = if option.underlying_is_index {
            option.index_factor.unwrap_or(1.0)
        } else {
            1.0
        };
        let strike = decimal_to_f64(option.strike, "strike")?;

        if t < credit::MIN_TIME_TO_EXPIRY_GREEKS {
            return Ok(0.0);
        }
        let forward = forward_spread_bp / self.config.bp_per_unit;
        if forward <= 0.0 || strike <= 0.0 {
            return Ok(0.0);
        }
        let d1 = if sigma >= credit::MIN_VOLATILITY_GREEKS {
            d1(forward, strike, 0.0, sigma, t, 0.0)
        } else {
            0.0
        };
        Ok(scale * risky_annuity * forward * norm_pdf(d1) * t.sqrt() / 100.0)
    }

    /// Finite-difference theta (complete, including risky annuity decay).
    ///
    /// Computes theta by repricing the option with a 1-day time shift.
    /// This captures both the Black formula time decay and the risky annuity decay,
    /// providing a complete theta measure.
    ///
    /// # Formula
    ///
    /// θ_daily = V(as_of + 1d) - V(as_of)
    ///
    /// This is the 1-day dollar theta (not annualized). Moving forward in time
    /// by one calendar day reduces time-to-expiry, capturing both Black formula
    /// time decay and risky annuity decay.
    ///
    /// # Returns
    ///
    /// The dollar value change per day (typically negative for long positions).
    pub(crate) fn theta_finite_diff(
        &self,
        option: &CDSOption,
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
            finstack_core::dates::DayCountContext::default(),
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
        option: &CDSOption,
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
                finstack_core::dates::DayCountContext::default(),
            )?;
            let t0 = option.day_count.year_fraction(
                as_of,
                option.expiry,
                finstack_core::dates::DayCountContext::default(),
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
    ///
    /// # Errors
    ///
    /// Returns an error if `target_price` is negative (options have non-negative value).
    pub(crate) fn implied_vol(
        &self,
        option: &CDSOption,
        curves: &MarketContext,
        as_of: finstack_core::dates::Date,
        target_price: f64,
        initial_guess: Option<f64>,
    ) -> Result<f64> {
        // Options have non-negative value; reject negative target prices
        if target_price < 0.0 {
            return Err(finstack_core::Error::Validation(
                "Implied vol target price must be non-negative".to_string(),
            ));
        }

        // Pre-compute market inputs independent of σ
        let t = option.day_count.year_fraction(
            as_of,
            option.expiry,
            finstack_core::dates::DayCountContext::default(),
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
        let sigma0 = if let Some(v) = option.pricing_overrides.market_quotes.implied_volatility {
            v
        } else {
            curves
                .get_surface(option.vol_surface_id.as_str())
                .ok()
                .map(|s| s.value_clamped(t, decimal_to_f64(option.strike, "strike").unwrap_or(0.0)))
                .unwrap_or(self.config.iv_initial_guess)
        };
        let x0 = (initial_guess.unwrap_or(sigma0.max(credit::MIN_VOLATILITY_GREEKS))).ln();

        let solver = BrentSolver::new().tolerance(1e-10);
        let root = solver.solve(f, x0)?;
        // Ensure minimum volatility floor for numerical stability
        Ok(root.exp().max(credit::MIN_VOLATILITY_GREEKS))
    }
}

// ========================= REGISTRY PRICER =========================

/// Registry pricer for CDS Option using Black76 model
pub(crate) struct SimpleCDSOptionBlackPricer {
    model_key: crate::pricer::ModelKey,
}

impl SimpleCDSOptionBlackPricer {
    /// Create a new CDS option pricer with default Black76 model
    pub(crate) fn new() -> Self {
        Self::with_model(crate::pricer::ModelKey::Black76)
    }

    /// Create a CDS option pricer with specified model key
    pub(crate) fn with_model(model_key: crate::pricer::ModelKey) -> Self {
        Self { model_key }
    }
}

impl Default for SimpleCDSOptionBlackPricer {
    fn default() -> Self {
        Self::new()
    }
}

impl crate::pricer::Pricer for SimpleCDSOptionBlackPricer {
    fn key(&self) -> crate::pricer::PricerKey {
        crate::pricer::PricerKey::new(crate::pricer::InstrumentType::CDSOption, self.model_key)
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> std::result::Result<crate::results::ValuationResult, crate::pricer::PricingError> {
        use crate::instruments::common_impl::traits::Instrument;

        // Type-safe downcasting
        let cds_option = instrument
            .as_any()
            .downcast_ref::<crate::instruments::credit_derivatives::cds_option::CDSOption>()
            .ok_or_else(|| {
                crate::pricer::PricingError::type_mismatch(
                    crate::pricer::InstrumentType::CDSOption,
                    instrument.key(),
                )
            })?;

        // Use the provided as_of date for valuation
        // Compute present value using the engine
        let pv = CDSOptionPricer::default()
            .npv(cds_option, market, as_of)
            .map_err(|e| {
                crate::pricer::PricingError::model_failure_with_context(
                    e.to_string(),
                    crate::pricer::PricingErrorContext::default(),
                )
            })?;

        // Return stamped result
        Ok(
            crate::results::ValuationResult::stamped(cds_option.id(), as_of, pv).with_details(
                crate::results::ValuationDetails::CreditDerivative(
                    crate::results::CreditDerivativeValuationDetails {
                        model_key: format!("{:?}", self.model_key),
                        integration_method: None,
                    },
                ),
            ),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruments::credit_derivatives::cds::CDSConvention;

    #[test]
    fn synthetic_underlying_cds_uses_option_convention() {
        let mut option = CDSOption::example().expect("CDSOption example is valid");
        option.underlying_convention = CDSConvention::IsdaEu;

        let cds = synthetic_underlying_cds(&option).expect("synthetic CDS should build");
        assert_eq!(cds.convention, CDSConvention::IsdaEu);
        assert_eq!(cds.premium.day_count, CDSConvention::IsdaEu.day_count());
        assert_eq!(
            cds.protection.settlement_delay,
            CDSConvention::IsdaEu.settlement_delay()
        );
    }

    /// Diagnostic-only: prints the intermediate Black-on-spreads inputs that
    /// the CDS option pricer feeds the Black formula for the Bloomberg
    /// `cds_option_payer_atm_3m` golden, plus comparable values when the
    /// synthetic underlying premium leg starts at the prior IMM (Bloomberg
    /// CDSO convention) instead of the option expiry. Run with:
    /// `cargo test -p finstack-valuations diag_cds_option_payer_atm_3m -- --nocapture`.
    #[test]
    #[ignore = "diagnostic only; print intermediate Black-on-spreads inputs for Bloomberg CDSO golden"]
    #[allow(clippy::excessive_precision)]
    fn diag_cds_option_payer_atm_3m() {
        use crate::instruments::common_impl::parameters::CreditParams;
        use crate::instruments::credit_derivatives::cds::{CreditDefaultSwap, PayReceive};
        use crate::instruments::credit_derivatives::cds_option::parameters::CDSOptionParams;
        use crate::instruments::credit_derivatives::cds_option::CDSOption;
        use finstack_core::currency::Currency;
        use finstack_core::dates::{Date, DayCountContext};
        use finstack_core::market_data::context::MarketContext;
        use finstack_core::market_data::term_structures::{DiscountCurve, HazardCurve};
        use finstack_core::math::interp::InterpStyle;
        use finstack_core::money::Money;
        use rust_decimal::Decimal;

        let as_of = Date::from_calendar_date(2026, time::Month::May, 2).unwrap();

        // Discount curve transcribed from the Bloomberg CDSW cashflow schedule.
        let disc_knots: Vec<(f64, f64)> = vec![
            (0.0, 1.0),
            (0.13972602739726028, 0.9948564304744848),
            (0.38904109589041097, 0.98574857),
            (0.6383561643835617, 0.97668173),
            (0.8876712328767123, 0.96764815),
            (1.1369863013698631, 0.95875431),
            (1.3863013698630137, 0.94999432),
            (1.6356164383561644, 0.94131437),
            (1.8849315068493151, 0.93271373),
            (2.136986301369863, 0.92419467),
            (2.389041095890411, 0.91584876),
            (2.6383561643835617, 0.9076677),
            (2.884931506849315, 0.89964844),
            (3.136986301369863, 0.8914406),
            (3.389041095890411, 0.88321729),
            (3.6383561643835617, 0.87515799),
            (3.884931506849315, 0.86725959),
            (4.136986301369863, 0.85911547),
            (4.389041095890411, 0.85089899),
            (4.638356164383562, 0.84284912),
            (4.884931506849315, 0.83496263),
            (5.136986301369863, 0.82680571),
        ];
        let disc = DiscountCurve::builder("USD-S531-SWAP")
            .base_date(as_of)
            .knots(disc_knots)
            .interp(InterpStyle::LogLinear)
            .build()
            .expect("disc curve");

        let haz_knots: Vec<(f64, f64)> = vec![
            (0.13972602739726028, 0.00997646002277055),
            (0.38904109589041097, 0.010174852397118335),
            (0.6383561643835617, 0.010172469869400271),
            (0.8876712328767123, 0.010174050726913558),
            (1.1369863013698631, 0.010175577951597636),
            (1.3863013698630137, 0.010177051103745225),
            (1.6356164383561644, 0.010174391557496821),
            (1.8849315068493151, 0.010171645976754756),
            (2.136986301369863, 0.010175861524607866),
            (2.389041095890411, 0.010173574090087208),
            (2.6383561643835617, 0.010175851741102628),
            (2.884931506849315, 0.010177267513353205),
            (3.136986301369863, 0.010173574731178204),
            (3.389041095890411, 0.010175091002821083),
            (3.6383561643835617, 0.01017184051056745),
            (3.884931506849315, 0.010176000795259657),
            (4.136986301369863, 0.01017429786109359),
            (4.389041095890411, 0.010175565889646949),
            (4.638356164383562, 0.010175080613668764),
            (4.884931506849315, 0.010173650631709215),
            (5.136986301369863, 0.010286841673740938),
        ];
        let par_knots: Vec<(f64, f64)> = vec![
            (0.5, 10.5),
            (1.0, 14.5),
            (2.0, 22.0),
            (3.0, 32.0),
            (4.0, 43.5),
            (5.0, 57.0),
            (7.0, 68.5),
            (10.0, 80.4),
        ];
        let hazard = HazardCurve::builder("IBM-USD-SENIOR")
            .base_date(as_of)
            .recovery_rate(0.4)
            .knots(haz_knots)
            .par_spreads(par_knots)
            .build()
            .expect("haz");

        let market = MarketContext::new().insert(disc).insert(hazard);

        let params = CDSOptionParams::call(
            Decimal::from_str_exact("0.0058395400").unwrap(),
            Date::from_calendar_date(2026, time::Month::June, 26).unwrap(),
            Date::from_calendar_date(2031, time::Month::June, 20).unwrap(),
            Money::new(10_000_000.0, Currency::USD),
        )
        .unwrap();
        let credit = CreditParams::corporate_standard("IBM", "IBM-USD-SENIOR");
        let mut option =
            CDSOption::new("DIAG", &params, &credit, "USD-S531-SWAP", "DIAG-VOL").unwrap();
        option.pricing_overrides.market_quotes.implied_volatility = Some(0.8102);

        let t = option
            .day_count
            .year_fraction(as_of, option.expiry, DayCountContext::default())
            .unwrap();

        eprintln!("\n=== CDS OPTION DIAGNOSTIC ===");
        eprintln!(
            "  as_of {} expiry {} cds_maturity {} day_count {:?}",
            as_of, option.expiry, option.cds_maturity, option.day_count
        );
        eprintln!(
            "  t (yf): {t:.10}    (calendar days {})",
            (option.expiry - as_of).whole_days()
        );

        let synth_spread_bp = option.strike * Decimal::new(10000, 0);
        let synth = CreditDefaultSwap::new_isda(
            option.id.clone(),
            Money::new(option.notional.amount(), option.notional.currency()),
            PayReceive::PayFixed,
            option.underlying_convention,
            synth_spread_bp,
            option.expiry,
            option.cds_maturity,
            option.recovery_rate,
            option.discount_curve_id.clone(),
            option.credit_curve_id.clone(),
        )
        .unwrap();

        let disc_arc = market.get_discount(&option.discount_curve_id).unwrap();
        let haz_arc = market.get_hazard(&option.credit_curve_id).unwrap();

        let cds_pricer = CDSPricer::new();
        let par = cds_pricer
            .par_spread(&synth, disc_arc.as_ref(), haz_arc.as_ref(), as_of)
            .unwrap();
        let mut zero = synth.clone();
        zero.premium.spread_bp = Decimal::ZERO;
        let ra = cds_pricer
            .risky_annuity(&zero, disc_arc.as_ref(), haz_arc.as_ref(), as_of)
            .unwrap();
        eprintln!("  [premium.start = expiry]                        F={par:.6} bp, RA={ra:.10}");

        // Bloomberg-style: premium leg starts at prior IMM (06/22/26 BDC-rolled).
        let prior_imm = Date::from_calendar_date(2026, time::Month::June, 22).unwrap();
        let alt = CreditDefaultSwap::new_isda(
            option.id.clone(),
            Money::new(option.notional.amount(), option.notional.currency()),
            PayReceive::PayFixed,
            option.underlying_convention,
            synth_spread_bp,
            prior_imm,
            option.cds_maturity,
            option.recovery_rate,
            option.discount_curve_id.clone(),
            option.credit_curve_id.clone(),
        )
        .unwrap();
        let par_alt = cds_pricer
            .par_spread(&alt, disc_arc.as_ref(), haz_arc.as_ref(), as_of)
            .unwrap();
        let mut zero_alt = alt.clone();
        zero_alt.premium.spread_bp = Decimal::ZERO;
        let ra_alt = cds_pricer
            .risky_annuity(&zero_alt, disc_arc.as_ref(), haz_arc.as_ref(), as_of)
            .unwrap();
        eprintln!(
            "  [premium.start = prior IMM 2026-06-22]          F={par_alt:.6} bp, RA={ra_alt:.10}"
        );

        // And with protection_effective_date set to the option expiry.
        let mut alt_fwd = alt.clone();
        alt_fwd.protection_effective_date = Some(option.expiry);
        let par_fwd = cds_pricer
            .par_spread(&alt_fwd, disc_arc.as_ref(), haz_arc.as_ref(), as_of)
            .unwrap();
        let mut zero_fwd = alt_fwd.clone();
        zero_fwd.premium.spread_bp = Decimal::ZERO;
        let ra_fwd = cds_pricer
            .risky_annuity(&zero_fwd, disc_arc.as_ref(), haz_arc.as_ref(), as_of)
            .unwrap();
        eprintln!(
            "  [premium=prior IMM, protection_eff=expiry]      F={par_fwd:.6} bp, RA={ra_fwd:.10}"
        );

        // Manual Black PV with current finstack inputs ----------------------
        let pricer = CDSOptionPricer::default();
        let pv_now = pricer
            .credit_option_price(&option, par, ra, 0.8102, t)
            .unwrap()
            .amount();
        let pv_alt = pricer
            .credit_option_price(&option, par_fwd, ra_fwd, 0.8102, t)
            .unwrap()
            .amount();
        eprintln!("  Black PV (premium.start = expiry):                {pv_now:.4}");
        eprintln!("  Black PV (premium=prior IMM, prot_eff=expiry):    {pv_alt:.4}");

        eprintln!("\n  Bloomberg targets: PV=31177.82, F=58.3954, sigma=0.8102, Delta(N(d1))=54.05%, RPV01_implied_from_DV01=4.74");

        // ----- bootstrap hazards from the par-spread term structure -----
        eprintln!("\n  --- Bootstrapping hazards from par spreads (sequential Brent) ---");
        let par_term: Vec<(f64, f64)> = vec![
            (0.5, 10.5),
            (1.0, 14.5),
            (2.0, 22.0),
            (3.0, 32.0),
            (4.0, 43.5),
            (5.0, 57.0),
            (7.0, 68.5),
            (10.0, 80.4),
        ];
        let recovery = 0.4_f64;
        let dc = option.day_count;
        let bootstrapped = bootstrap_flat_hazard_pieces(&disc_arc, &par_term, recovery, dc, as_of);
        for (i, &(t, h)) in bootstrapped.iter().enumerate() {
            let s = par_term[i].1;
            eprintln!("    par={s:>5.1}bp  knot_t_act365={t:.16}  h={h:.16}");
        }

        // Build a hazard curve with these bootstrapped knots, then re-run forward
        // spread + risky annuity for the option's synthetic CDS.
        let bootstrap_haz = HazardCurve::builder("IBM-USD-SENIOR-BOOT")
            .base_date(as_of)
            .recovery_rate(recovery)
            .knots(bootstrapped.clone())
            .par_spreads(par_term.clone())
            .build()
            .expect("bootstrapped haz");
        let market_b = market.clone().insert(bootstrap_haz);
        let haz_b = market_b.get_hazard("IBM-USD-SENIOR-BOOT").unwrap();
        let synth_b = CreditDefaultSwap::new_isda(
            option.id.clone(),
            Money::new(option.notional.amount(), option.notional.currency()),
            PayReceive::PayFixed,
            option.underlying_convention,
            synth_spread_bp,
            option.expiry,
            option.cds_maturity,
            recovery,
            option.discount_curve_id.clone(),
            "IBM-USD-SENIOR-BOOT",
        )
        .unwrap();
        let f_boot = cds_pricer
            .par_spread(&synth_b, disc_arc.as_ref(), haz_b.as_ref(), as_of)
            .unwrap();
        let mut zero_b = synth_b.clone();
        zero_b.premium.spread_bp = Decimal::ZERO;
        let ra_boot = cds_pricer
            .risky_annuity(&zero_b, disc_arc.as_ref(), haz_b.as_ref(), as_of)
            .unwrap();
        eprintln!(
            "  [bootstrapped haz, premium.start = expiry]      F={f_boot:.6} bp, RA={ra_boot:.10}"
        );

        // What does the CDS pricer report as the *5Y spot* par spread under
        // the bootstrapped curve? (This sanity-checks the bootstrap.)
        let spot_5y = CreditDefaultSwap::new_isda(
            option.id.clone(),
            Money::new(option.notional.amount(), option.notional.currency()),
            PayReceive::PayFixed,
            option.underlying_convention,
            Decimal::new(57, 0),
            as_of,
            option.cds_maturity,
            recovery,
            option.discount_curve_id.clone(),
            "IBM-USD-SENIOR-BOOT",
        )
        .unwrap();
        let par_5y_spot = cds_pricer
            .par_spread(&spot_5y, disc_arc.as_ref(), haz_b.as_ref(), as_of)
            .unwrap();
        eprintln!(
            "  Sanity: spot 5Y par on bootstrapped curve  = {par_5y_spot:.6} bp (expect ~57)"
        );

        let pv_boot = pricer
            .credit_option_price(&option, f_boot, ra_boot, 0.8102, t)
            .unwrap()
            .amount();
        eprintln!("  Black PV (bootstrapped curve)              = {pv_boot:.4}");
    }

    /// Bootstrap a piecewise-flat hazard curve from a par-spread term structure
    /// using the CDS pricer + Brent root finding sequentially per pillar.
    /// Returns vector of (tenor_year, piecewise-flat hazard rate up to that
    /// tenor). This is a diagnostic helper, not a production calibrator.
    #[cfg(test)]
    fn bootstrap_flat_hazard_pieces(
        disc: &std::sync::Arc<finstack_core::market_data::term_structures::DiscountCurve>,
        par_spreads_bp: &[(f64, f64)],
        recovery: f64,
        day_count: finstack_core::dates::DayCount,
        as_of: finstack_core::dates::Date,
    ) -> Vec<(f64, f64)> {
        use crate::instruments::credit_derivatives::cds::CDSConvention;
        use crate::instruments::credit_derivatives::cds::{CreditDefaultSwap, PayReceive};
        use finstack_core::currency::Currency;
        use finstack_core::dates::DayCountContext;
        use finstack_core::market_data::term_structures::HazardCurve;
        use finstack_core::math::solver::BrentSolver;
        use finstack_core::money::Money;
        use rust_decimal::Decimal;

        let mut accumulated: Vec<(f64, f64)> = Vec::with_capacity(par_spreads_bp.len());
        let pricer = CDSPricer::new();
        let ctx = DayCountContext::default();
        let _ = day_count;
        let _ = ctx;

        for (i, &(tenor_years, par_bp)) in par_spreads_bp.iter().enumerate() {
            // Calibrate hazard h_i (flat between previous pillar and this one)
            // such that CDS expiring at this pillar with coupon = par_bp prices
            // to par. Maturity is rolled to the next ISDA IMM date (03/20,
            // 06/20, 09/20, 12/20) at or after as_of + tenor_years to match
            // Bloomberg / SNAC curve definitions; the hazard knot time is the
            // IMM-rolled tenor in act/365f years for self-consistency with
            // par_spread evaluation.
            let target = as_of + time::Duration::days((tenor_years * 365.25).round() as i64);
            let maturity = next_isda_imm_on_or_after(target);
            let knot_t = (maturity - as_of).whole_days() as f64 / 365.0_f64;

            let pillar_idx = i;
            let prev_knots = accumulated.clone();

            let f_pv = |h: f64| -> f64 {
                if h <= 0.0 {
                    return f64::NAN;
                }
                let mut knots = prev_knots.clone();
                knots.push((knot_t, h));
                let haz = HazardCurve::builder("BOOT-TMP")
                    .base_date(as_of)
                    .recovery_rate(recovery)
                    .knots(knots)
                    .build()
                    .expect("hazard");

                let cds = CreditDefaultSwap::new_isda(
                    "BOOT",
                    Money::new(10_000_000.0, Currency::USD),
                    PayReceive::PayFixed,
                    CDSConvention::IsdaNa,
                    Decimal::from_f64_retain(par_bp).expect("par bp"),
                    as_of,
                    maturity,
                    recovery,
                    "DUMMY",
                    "BOOT-TMP",
                )
                .expect("synth cds for bootstrap");
                pricer
                    .par_spread(&cds, disc.as_ref(), &haz, as_of)
                    .map(|fitted_par| fitted_par - par_bp)
                    .unwrap_or(f64::NAN)
            };

            // Solve in an explicit bracket [1e-7, 1.0]. par_spread is monotone
            // increasing in the trailing hazard piece h_i, so this bracket is
            // wide enough for any IG/HY single-name curve.
            let solver = BrentSolver::new().tolerance(1e-12);
            let h_solved = solver
                .solve_in_bracket(&f_pv, 1e-7, 1.0)
                .expect("hazard bootstrap should converge in [1e-7, 1.0]");
            accumulated.push((knot_t, h_solved));
            let _ = pillar_idx;
        }

        accumulated
    }

    /// Next standard ISDA IMM date (20th of Mar/Jun/Sep/Dec) on or after the
    /// supplied date. Diagnostic helper.
    #[cfg(test)]
    fn next_isda_imm_on_or_after(d: finstack_core::dates::Date) -> finstack_core::dates::Date {
        use finstack_core::dates::Date;
        use time::Month;
        let imms = [Month::March, Month::June, Month::September, Month::December];
        let mut year = d.year();
        loop {
            for &m in &imms {
                if let Ok(candidate) = Date::from_calendar_date(year, m, 20) {
                    if candidate >= d {
                        return candidate;
                    }
                }
            }
            year += 1;
            if year > d.year() + 30 {
                panic!("could not roll to IMM on or after {d}");
            }
        }
    }
}
