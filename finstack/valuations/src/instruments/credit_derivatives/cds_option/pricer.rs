//! Core CDS Option pricing engine and helpers.
//!
//! Provides deterministic valuation for options on CDS spreads using a
//! Black-style framework on spreads, with inputs derived from market
//! curves (discount and hazard) and an implied volatility surface.
//!
//! The engine exposes a simple `npv` function that composes the
//! forward spread, risky annuity, and discount factor into the option
//! PV via the modified Black formula implemented on the instrument.

use crate::constants::{credit, numerical, ONE_BASIS_POINT};
use crate::instruments::common_impl::models::{d1, d2, norm_cdf, norm_pdf};
use crate::instruments::common_impl::parameters::OptionType;
use crate::instruments::common_impl::traits::Instrument;
use crate::instruments::credit_derivatives::cds::pricer::{CDSPricer, CDSPricerConfig};
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
        let t = option.black_time_to_expiry(as_of)?;

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

        let strike_f64 = decimal_to_f64(option.strike, "strike")?;
        let sigma = crate::instruments::common_impl::vol_resolution::resolve_sigma_at(
            &option.pricing_overrides.market_quotes,
            curves,
            option.vol_surface_id.as_str(),
            t,
            strike_f64,
        )?;

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
        option.effective_underlying_effective_date(),
        option.cds_maturity,
        option.recovery_rate,
        option.discount_curve_id.to_owned(),
        option.credit_curve_id.to_owned(),
    )?;
    if cds.premium.start < option.expiry {
        cds.protection_effective_date = Some(option.expiry);
    }
    if option
        .pricing_overrides
        .model_config
        .cds_act360_include_last_day
    {
        cds.protection_effective_date =
            Some(option.effective_underlying_effective_date() - time::Duration::days(1));
        cds.protection.settlement_delay = 0;
    }
    // Forward CDS uses the Bloomberg CDSW conventions shown on the option
    // underlying screen: adjusted-to-adjusted accruals plus the +1-day rule
    // on the final ACT/360 period.
    cds.pricing_overrides.model_config = option.pricing_overrides.model_config.clone();
    cds.valuation_convention = if cds
        .pricing_overrides
        .model_config
        .cds_act360_include_last_day
    {
        CdsValuationConvention::IsdaDirty
    } else {
        CdsValuationConvention::BloombergCdswClean
    };
    Ok(cds)
}

struct ForwardCdsBlackInputs {
    forward_spread_bp: f64,
    risky_annuity: f64,
}

impl CDSOptionPricer {
    fn forward_cds_black_inputs(
        &self,
        option: &CDSOption,
        disc: &finstack_core::market_data::term_structures::DiscountCurve,
        surv: &finstack_core::market_data::term_structures::HazardCurve,
        as_of: finstack_core::dates::Date,
    ) -> Result<ForwardCdsBlackInputs> {
        let cds = synthetic_underlying_cds(option)?;
        let cds_pricer = CDSPricer::new();
        let premium_per_bp =
            cds_pricer.forward_premium_leg_pv_per_bp(&cds, disc, surv, as_of, option.expiry)?;
        let risky_annuity = premium_per_bp / ONE_BASIS_POINT;
        let denominator = risky_annuity * cds.notional.amount();
        if denominator.abs() < numerical::RATE_COMPARISON_TOLERANCE {
            return Err(finstack_core::Error::Validation(
                "forward CDS annuity is too small for CDS option forward spread".to_string(),
            ));
        }

        let protection_pv = cds_pricer
            .pv_protection_leg(&cds, disc, surv, as_of)?
            .amount();
        let forward_protection_pv = if cds
            .pricing_overrides
            .model_config
            .cds_act360_include_last_day
        {
            let mut scheduled_config = CDSPricerConfig::from_cds(&cds);
            scheduled_config.include_accrual = false;
            let scheduled_per_bp = CDSPricer::with_config(scheduled_config)
                .forward_premium_leg_pv_per_bp(&cds, disc, surv, as_of, option.expiry)?;
            let spread_bp = decimal_to_f64(cds.premium.spread_bp, "underlying CDS spread_bp")?;
            let spread = spread_bp / self.config.bp_per_unit;
            let half_day_rebate_midpoint = spread * cds.notional.amount() * (0.125 / 360.0);
            protection_pv + spread_bp * (premium_per_bp - scheduled_per_bp) * cds.notional.amount()
                - half_day_rebate_midpoint
        } else {
            protection_pv
        };
        Ok(ForwardCdsBlackInputs {
            forward_spread_bp: forward_protection_pv / denominator * self.config.bp_per_unit,
            risky_annuity,
        })
    }

    fn forward_spread_from_pricer(
        &self,
        option: &CDSOption,
        disc: &finstack_core::market_data::term_structures::DiscountCurve,
        surv: &finstack_core::market_data::term_structures::HazardCurve,
        as_of: finstack_core::dates::Date,
    ) -> Result<f64> {
        let mut forward_bp = self
            .forward_cds_black_inputs(option, disc, surv, as_of)?
            .forward_spread_bp;
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
        let t = option.black_time_to_expiry(as_of)?;
        if t <= dt_years {
            return Ok(0.0);
        }

        // Base PV at as_of
        let pv_base = self.npv(option, curves, as_of)?.amount();

        // Shifted PV at as_of + 1 day (moving forward in time reduces time to expiry).
        // For Bloomberg-style cash-settled CDS options, the quoted premium settlement
        // date also rolls forward with the valuation date; the exercise settlement date
        // remains the fixed payoff date.
        let as_of_shifted = as_of + Duration::days(1);
        let mut shifted_option = option.clone();
        shifted_option.cash_settlement_date =
            Some(option.effective_cash_settlement_date(as_of)? + Duration::days(1));
        let pv_shifted = self.npv(&shifted_option, curves, as_of_shifted)?.amount();

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

        // Bloomberg CDSO scales Black PV by the forward premium-leg risky annuity.
        // Scheduled coupons follow the standard CDS schedule, while accrual-on-default
        // starts only once the forward CDS protection is live.
        Ok(self
            .forward_cds_black_inputs(option, disc, surv, as_of)?
            .risky_annuity)
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
        let t = option.black_time_to_expiry(as_of)?;
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

        let strike_f64 = decimal_to_f64(option.strike, "strike").unwrap_or(0.0);
        let sigma0 = crate::instruments::common_impl::vol_resolution::resolve_sigma_at(
            &option.pricing_overrides.market_quotes,
            curves,
            option.vol_surface_id.as_str(),
            t,
            strike_f64,
        )
        .unwrap_or(self.config.iv_initial_guess);
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

    #[test]
    fn synthetic_underlying_cds_uses_underlying_effective_date_for_accrual() {
        let mut option = CDSOption::example().expect("CDSOption example is valid");
        let effective =
            finstack_core::dates::Date::from_calendar_date(2025, time::Month::March, 20)
                .expect("valid date");
        option.underlying_effective_date = Some(effective);

        let cds = synthetic_underlying_cds(&option).expect("synthetic CDS should build");

        assert_eq!(cds.premium.start, effective);
        assert_eq!(cds.protection_effective_date, Some(option.expiry));
    }

    #[test]
    fn forward_cds_annuity_excludes_pre_expiry_accrual_on_default() {
        use finstack_core::dates::{DayCount, DayCountContext};
        use finstack_core::market_data::context::MarketContext;
        use finstack_core::market_data::term_structures::{DiscountCurve, HazardCurve};
        use time::macros::date;

        let as_of = date!(2025 - 01 - 02);
        let forward_start = date!(2025 - 02 - 20);
        let first_coupon = date!(2025 - 03 - 20);
        let maturity = date!(2025 - 06 - 20);

        let mut option = CDSOption::example().expect("CDSOption example is valid");
        option.expiry = forward_start;
        option.cds_maturity = maturity;
        option.underlying_effective_date = Some(date!(2024 - 12 - 20));

        let t_forward_start = DayCount::Act365F
            .year_fraction(as_of, forward_start, DayCountContext::default())
            .expect("valid forward-start year fraction");
        let t_maturity = DayCount::Act365F
            .year_fraction(as_of, maturity, DayCountContext::default())
            .expect("valid maturity year fraction");

        let discount = DiscountCurve::builder("USD-OIS")
            .base_date(as_of)
            .day_count(DayCount::Act365F)
            .knots([(0.0, 1.0), (1.0, 1.0)])
            .build()
            .expect("flat discount curve");
        let hazard = HazardCurve::builder("CORP-HAZARD")
            .base_date(as_of)
            .day_count(DayCount::Act365F)
            .recovery_rate(option.recovery_rate)
            .knots([(0.0, 5.0), (t_forward_start, 0.0), (t_maturity, 0.0)])
            .build()
            .expect("front-loaded hazard curve");
        let market = MarketContext::new().insert(discount).insert(hazard);

        let pricer = CDSOptionPricer::default();
        let actual = pricer
            .risky_annuity(&option, &market, as_of)
            .expect("risky annuity");

        let hazard = market
            .get_hazard("CORP-HAZARD")
            .expect("hazard curve should be present");
        let first_coupon_survival = hazard
            .sp_on_date(first_coupon)
            .expect("first coupon survival");
        let final_coupon_survival = hazard.sp_on_date(maturity).expect("maturity survival");
        let first_coupon_accrual = DayCount::Act360
            .year_fraction(
                option.underlying_effective_date.expect("effective date"),
                first_coupon,
                DayCountContext::default(),
            )
            .expect("first coupon accrual");
        let final_coupon_accrual =
            (DayCount::calendar_days(first_coupon, maturity) + 1).max(0) as f64 / 360.0;
        let expected = first_coupon_accrual * first_coupon_survival
            + final_coupon_accrual * final_coupon_survival;

        assert!(
            (actual - expected).abs() < 1e-12,
            "forward CDS annuity must not include accrual-on-default before option expiry: \
             actual={actual:.12}, expected={expected:.12}"
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
        use finstack_core::dates::Date;
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
            (0.6356164383561644, 0.0017745644177803),
            (1.1342465753424658, 0.0033197558026043),
            (2.1369863013698631, 0.0051940157462903),
            (3.1369863013698631, 0.0092299223474419),
            (4.1369863013698627, 0.0139861785227869),
            (5.1369863013698627, 0.0202486808214905),
            (7.1397260273972600, 0.0174996558653136),
            (10.1424657534246574, 0.0198215118527936),
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

        option.cash_settlement_date =
            Some(Date::from_calendar_date(2026, time::Month::May, 7).unwrap());
        option.exercise_settlement_date =
            Some(Date::from_calendar_date(2026, time::Month::June, 24).unwrap());
        option.underlying_effective_date =
            Some(Date::from_calendar_date(2026, time::Month::June, 21).unwrap());

        let t = option.black_time_to_expiry(as_of).unwrap();

        eprintln!("\n=== CDS OPTION DIAGNOSTIC ===");
        eprintln!(
            "  as_of {} expiry {} cds_maturity {} day_count {:?}",
            as_of, option.expiry, option.cds_maturity, option.day_count
        );
        eprintln!(
            "  t (yf): {t:.10}    (settlement days {})",
            (option.exercise_settlement_date.unwrap() - option.cash_settlement_date.unwrap())
                .whole_days()
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

        let underlying_effective = Date::from_calendar_date(2026, time::Month::June, 21).unwrap();
        let underlying_effective_adjusted =
            Date::from_calendar_date(2026, time::Month::June, 22).unwrap();
        for (label, start) in [
            ("underlying effective 2026-06-21", underlying_effective),
            (
                "underlying effective adjusted 2026-06-22",
                underlying_effective_adjusted,
            ),
        ] {
            let eff = CreditDefaultSwap::new_isda(
                option.id.clone(),
                Money::new(option.notional.amount(), option.notional.currency()),
                PayReceive::PayFixed,
                option.underlying_convention,
                synth_spread_bp,
                start,
                option.cds_maturity,
                option.recovery_rate,
                option.discount_curve_id.clone(),
                option.credit_curve_id.clone(),
            )
            .unwrap();
            let par_eff = cds_pricer
                .par_spread(&eff, disc_arc.as_ref(), haz_arc.as_ref(), as_of)
                .unwrap();
            let mut zero_eff = eff.clone();
            zero_eff.premium.spread_bp = Decimal::ZERO;
            let ra_eff = cds_pricer
                .risky_annuity(&zero_eff, disc_arc.as_ref(), haz_arc.as_ref(), as_of)
                .unwrap();
            eprintln!("  [{label}]                F={par_eff:.6} bp, RA={ra_eff:.10}");
        }

        let mut exercise_settle_prot = CreditDefaultSwap::new_isda(
            option.id.clone(),
            Money::new(option.notional.amount(), option.notional.currency()),
            PayReceive::PayFixed,
            option.underlying_convention,
            synth_spread_bp,
            underlying_effective,
            option.cds_maturity,
            option.recovery_rate,
            option.discount_curve_id.clone(),
            option.credit_curve_id.clone(),
        )
        .unwrap();
        exercise_settle_prot.protection_effective_date = option.exercise_settlement_date;
        let par_exercise_settle = cds_pricer
            .par_spread(
                &exercise_settle_prot,
                disc_arc.as_ref(),
                haz_arc.as_ref(),
                as_of,
            )
            .unwrap();
        let mut zero_exercise_settle = exercise_settle_prot.clone();
        zero_exercise_settle.premium.spread_bp = Decimal::ZERO;
        let ra_exercise_settle = cds_pricer
            .risky_annuity(
                &zero_exercise_settle,
                disc_arc.as_ref(),
                haz_arc.as_ref(),
                as_of,
            )
            .unwrap();
        eprintln!(
            "  [premium=2026-06-21, prot_eff=exercise_settle] F={par_exercise_settle:.6} bp, RA={ra_exercise_settle:.10}"
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
        for (label, start, protection_effective_date) in [
            (
                "boot haz, underlying effective 2026-06-21",
                underlying_effective,
                None,
            ),
            (
                "boot haz, effective adjusted 2026-06-22",
                underlying_effective_adjusted,
                None,
            ),
            (
                "boot haz, premium=2026-06-21, prot_eff=expiry",
                underlying_effective,
                Some(option.expiry),
            ),
            (
                "boot haz, premium=2026-06-22, prot_eff=expiry",
                underlying_effective_adjusted,
                Some(option.expiry),
            ),
        ] {
            let mut eff_b = CreditDefaultSwap::new_isda(
                option.id.clone(),
                Money::new(option.notional.amount(), option.notional.currency()),
                PayReceive::PayFixed,
                option.underlying_convention,
                synth_spread_bp,
                start,
                option.cds_maturity,
                recovery,
                option.discount_curve_id.clone(),
                "IBM-USD-SENIOR-BOOT",
            )
            .unwrap();
            eff_b.protection_effective_date = protection_effective_date;
            let f_eff_b = cds_pricer
                .par_spread(&eff_b, disc_arc.as_ref(), haz_b.as_ref(), as_of)
                .unwrap();
            let mut zero_eff_b = eff_b.clone();
            zero_eff_b.premium.spread_bp = Decimal::ZERO;
            let ra_eff_b = cds_pricer
                .risky_annuity(&zero_eff_b, disc_arc.as_ref(), haz_b.as_ref(), as_of)
                .unwrap();
            eprintln!("  [{label}]     F={f_eff_b:.6} bp, RA={ra_eff_b:.10}");
        }

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

    /// Diagnostic for the Bloomberg `cds_option_payer_atm_3m` golden.
    ///
    /// Decomposes the residual gaps vs. Bloomberg into:
    /// 1. **Forward CDS RPV01** — per-period schedule + DF + SP rows under
    ///    several calendar / hazard / AOD variants, then totals.
    /// 2. **Black time-to-expiry** — sweeps `(start, end, day_count)`
    ///    combinations holding annuity, forward, and σ fixed.
    /// 3. **CS01 building blocks** — `∂F/∂S` from a par-spread bump, and
    ///    the analytic `annuity · Φ(d1) · ∂F/∂S · N · 1bp` reference.
    /// 4. **Production sanity** — invokes the actual CDSO pricer through
    ///    the registered metric path so the diagnostic and runner agree.
    ///
    /// Run with:
    /// ```text
    /// cargo test -p finstack-valuations --lib \
    ///   instruments::credit_derivatives::cds_option::pricer::tests::diag_cds_option_phase_a \
    ///   -- --include-ignored --nocapture
    /// ```
    #[test]
    #[ignore = "diagnostic only; prints per-period RPV01 contributions and T-convention NPV table"]
    #[allow(clippy::excessive_precision, clippy::too_many_lines)]
    fn diag_cds_option_phase_a() {
        use crate::constants::ONE_BASIS_POINT;
        use crate::instruments::credit_derivatives::cds::{
            CDSConvention, CdsValuationConvention, CreditDefaultSwap, PayReceive,
        };
        use finstack_core::currency::Currency;
        use finstack_core::dates::calendar::{calendar_by_id, CompositeCalendar, CompositeMode};
        use finstack_core::dates::{
            adjust, BusinessDayConvention, Date, DateExt, DayCount, DayCountContext,
        };
        use finstack_core::market_data::context::MarketContext;
        use finstack_core::market_data::term_structures::{
            DiscountCurve, DiscountCurveRateCalibration, DiscountCurveRateQuote,
            DiscountCurveRateQuoteType, HazardCurve,
        };
        use finstack_core::math::interp::InterpStyle;
        use finstack_core::money::Money;
        use rust_decimal::Decimal;

        // ----- 1. Build the same market context as the golden ------------------
        let as_of = Date::from_calendar_date(2026, time::Month::May, 2).unwrap();

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
        let disc_arc = std::sync::Arc::new(disc.clone());

        // Bootstrap hazard knots from the par-spread term structure
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
        let dc = finstack_core::dates::DayCount::Act360;
        let bootstrapped = bootstrap_flat_hazard_pieces(&disc_arc, &par_term, recovery, dc, as_of);

        let hazard = HazardCurve::builder("IBM-USD-SENIOR-BOOT")
            .base_date(as_of)
            .recovery_rate(recovery)
            .knots(bootstrapped.clone())
            .par_spreads(par_term.clone())
            .build()
            .expect("haz");
        let market = MarketContext::new()
            .insert(disc.clone())
            .insert(hazard.clone());
        let haz_arc = market.get_hazard("IBM-USD-SENIOR-BOOT").unwrap();

        // ----- 2. Build the option's underlying forward CDS exactly --------------
        //
        // Bloomberg CDSW screen for the option underlying (cf. screenshots):
        //   1st Accr Start: 06/21/26 (Sun, displayed unadjusted)
        //   1st Coupon:    09/21/26 (BDC-adjusted from 09/20 Sun → next BD)
        //   Pen Coupon:    03/20/31
        //   Maturity:      06/20/31 (Sat, BDC-adjusted to 06/22 Mon)
        //   Day Cnt:       ACT/360
        //   Freq:          Q (quarterly)
        //   Calendar:      US, GB
        //   Bus Day Adj:   Modified Following

        let underlying_eff = Date::from_calendar_date(2026, time::Month::June, 21).unwrap();
        let cds_maturity = Date::from_calendar_date(2031, time::Month::June, 20).unwrap();
        let option_expiry = Date::from_calendar_date(2026, time::Month::June, 26).unwrap();
        let cash_settle = Date::from_calendar_date(2026, time::Month::May, 7).unwrap();
        let exercise_settle = Date::from_calendar_date(2026, time::Month::June, 24).unwrap();
        let strike_bp = 58.3954_f64;

        // Build several CDS variants. The CDS pricer's per-period APIs are
        // private; we instead vary the *total* annuity via `risky_annuity`
        // (no AOD) and `premium_leg_pv_per_bp` (with AOD), and reconstruct
        // the schedule via the public `generate_isda_schedule` to print
        // per-period DF/SP rows for context.
        let make_cds = |id: &str, calendar_id: Option<&str>| -> CreditDefaultSwap {
            let mut cds = CreditDefaultSwap::new_isda(
                id,
                Money::new(10_000_000.0, Currency::USD),
                PayReceive::PayFixed,
                CDSConvention::IsdaNa,
                Decimal::from_str_exact("58.3954").unwrap(),
                underlying_eff,
                cds_maturity,
                recovery,
                "USD-S531-SWAP",
                "IBM-USD-SENIOR-BOOT",
            )
            .unwrap();
            cds.protection_effective_date = Some(option_expiry);
            cds.valuation_convention = CdsValuationConvention::IsdaDirty;
            if let Some(id) = calendar_id {
                cds.premium.calendar_id = Some(id.to_string());
            }
            cds
        };

        let cds_nyse = make_cds("DIAG-NYSE", Some("nyse"));
        let cds_gblo = make_cds("DIAG-GBLO", Some("gblo"));
        let cds_no_cal = make_cds("DIAG-NOCAL", None);

        // Variant: BloombergCdswClean — uses adjusted-to-adjusted accrual periods
        // (matches the cds_5y_par_spread golden) AND applies the +1 day rule on
        // the final ACT/360 period (CDSW convention). This is the most likely
        // Bloomberg-aligned premium accrual scheme.
        let mut cds_cdsw = make_cds("DIAG-CDSW", Some("nyse"));
        cds_cdsw.valuation_convention = CdsValuationConvention::BloombergCdswClean;

        // Variant: hazard axis ACT/365L
        let haz_365l = HazardCurve::builder("IBM-365L")
            .base_date(as_of)
            .recovery_rate(recovery)
            .day_count(DayCount::Act365L)
            .knots(bootstrapped.clone())
            .build()
            .expect("haz_365l");

        let cds_pricer = CDSPricer::new();
        let dc_ctx = DayCountContext::default();
        eprintln!("\n=== Forward CDS RPV01 attribution ===");
        eprintln!("  Bloomberg CDSW (option underlying) Spread DV01 = 4515.52");
        eprintln!("    ⇒ implied RPV01_per_unit_notional               = 4.51552");
        eprintln!("    Finstack today (boot haz, NYSE calendar)         = 4.5029  (-0.28%)\n");

        // Per-period table for the NYSE variant: accrual fractions come from
        // public premium_cashflow_accruals; SP/DF from the curves directly.
        let schedule_nyse = cds_pricer.generate_isda_schedule(&cds_nyse).unwrap();
        let cf_accruals_nyse = cds_pricer
            .premium_cashflow_accruals(&cds_nyse, as_of)
            .unwrap();
        eprintln!("  [period schedule, calendar=NYSE, ACT/360, MF, IsdaNa, IsdaDirty]");
        eprintln!(
            "    {:>3}  {:>10}  {:>10}  {:>9}  {:>10}  {:>10}  {:>11}",
            "i", "accr_start", "pay_date", "yf_a360", "SP_pay", "DF_pay", "coupon/bp"
        );
        let mut sum_no_aod_manual = 0.0_f64;
        for (i, ((pay_date, yf), &start)) in cf_accruals_nyse
            .iter()
            .zip(schedule_nyse.iter())
            .enumerate()
        {
            // pay_date is BDC-adjusted IMM; use it both as "accrual end (effective)"
            // and as the discounting date.
            let t_end = haz_arc
                .day_count()
                .year_fraction(haz_arc.base_date(), *pay_date, dc_ctx)
                .unwrap();
            let sp_end = haz_arc.sp(t_end);
            let df_pay = disc.df_between_dates(as_of, *pay_date).unwrap();
            let contrib = yf * sp_end * df_pay;
            sum_no_aod_manual += contrib;
            eprintln!(
                "    {:>3}  {:>10}  {:>10}  {:>9.6}  {:>10.7}  {:>10.7}  {:>11.7}",
                i, start, pay_date, yf, sp_end, df_pay, contrib
            );
        }
        eprintln!("    Σ manual no-AOD (using pay_date for SP & DF) = {sum_no_aod_manual:.7}");

        // Total annuity (with and without AOD) for each variant
        eprintln!(
            "\n  Total annuity (per unit notional, decimal RPV01) — Bloomberg implied = 4.51552:"
        );
        let print_totals = |label: &str, cds: &CreditDefaultSwap, haz: &HazardCurve| -> () {
            let no_aod = cds_pricer.risky_annuity(cds, &disc, haz, as_of).unwrap();
            let with_aod = cds_pricer
                .premium_leg_pv_per_bp(cds, &disc, haz, as_of)
                .unwrap()
                / ONE_BASIS_POINT;
            let aod_only = with_aod - no_aod;
            eprintln!(
                "    {label:<46}  no_aod = {no_aod:.7}  with_aod = {with_aod:.7}  AOD = {aod_only:.7}  Δ_vs_BBG = {:+.5}",
                with_aod - 4.51552
            );
        };
        print_totals(
            "IsdaDirty,         calendar=NYSE,  haz ACT/365F (current)",
            &cds_nyse,
            haz_arc.as_ref(),
        );
        print_totals(
            "IsdaDirty,         calendar=GBLO,  haz ACT/365F",
            &cds_gblo,
            haz_arc.as_ref(),
        );
        print_totals(
            "IsdaDirty,         calendar=<none>, haz ACT/365F",
            &cds_no_cal,
            haz_arc.as_ref(),
        );
        print_totals(
            "IsdaDirty,         calendar=NYSE,  haz ACT/365L",
            &cds_nyse,
            &haz_365l,
        );
        print_totals(
            "BloombergCdswClean, calendar=NYSE, haz ACT/365F",
            &cds_cdsw,
            haz_arc.as_ref(),
        );

        let current_forward_annuity = cds_pricer
            .forward_premium_leg_pv_per_bp(
                &cds_cdsw,
                disc_arc.as_ref(),
                haz_arc.as_ref(),
                as_of,
                option_expiry,
            )
            .unwrap()
            / ONE_BASIS_POINT;
        let protection_pv = cds_pricer
            .pv_protection_leg(&cds_cdsw, disc_arc.as_ref(), haz_arc.as_ref(), as_of)
            .unwrap()
            .amount();
        let t_expiry = haz_arc
            .day_count()
            .year_fraction(haz_arc.base_date(), option_expiry, dc_ctx)
            .unwrap();
        let survival_to_expiry = haz_arc.sp(t_expiry);
        let accrual_delta = cds_cdsw
            .premium
            .day_count
            .year_fraction(underlying_eff, option_expiry, dc_ctx)
            .unwrap();
        let nyse_holiday_cal = calendar_by_id("nyse").expect("NYSE calendar");
        let forward_cash_settle = option_expiry
            .add_business_days(
                cds_cdsw.protection.settlement_delay.into(),
                nyse_holiday_cal,
            )
            .unwrap();
        let screen_df = disc.df_between_dates(as_of, exercise_settle).unwrap();
        let forward_cash_df = disc.df_between_dates(as_of, forward_cash_settle).unwrap();
        let screen_clean_adjustment = screen_df * survival_to_expiry * accrual_delta;
        let forward_cash_clean_adjustment = forward_cash_df * survival_to_expiry * accrual_delta;
        let print_og_candidate = |label: &str, annuity: f64| {
            let forward_bp = protection_pv / (annuity * 10_000_000.0) * 10_000.0;
            let forward = forward_bp / 10_000.0;
            let strike = strike_bp / 10_000.0;
            let sigma = 0.8102_f64;
            let t = 49.0_f64 / 360.0;
            let t_minus_1d = 48.0_f64 / 360.0;
            let d1_current = d1(forward, strike, 0.0, sigma, t, 0.0);
            let d2_current = d2(forward, strike, 0.0, sigma, t, 0.0);
            let npv = annuity
                * 10_000_000.0
                * (forward * norm_cdf(d1_current) - strike * norm_cdf(d2_current));
            let vega = annuity * 10_000_000.0 * forward * norm_pdf(d1_current) * t.sqrt() / 100.0;
            let d1_prev = d1(forward, strike, 0.0, sigma, t_minus_1d, 0.0);
            let d2_prev = d2(forward, strike, 0.0, sigma, t_minus_1d, 0.0);
            let npv_prev =
                annuity * 10_000_000.0 * (forward * norm_cdf(d1_prev) - strike * norm_cdf(d2_prev));
            eprintln!(
                "    {label:<46}  A={annuity:.7} F={forward_bp:.6}bp NPV={npv:.2} (Δ {:+.2}) Vega={vega:.2} (Δ {:+.2}) BlackTheta1d={:+.2}",
                npv - 31_177.82,
                vega - 381.02,
                npv_prev - npv,
            );
        };
        eprintln!("\n  OpenGamma clean forward annuity adjustment candidates:");
        eprintln!(
            "    current_forward_annuity = {current_forward_annuity:.7}; Delta(underlying_eff, expiry) = {accrual_delta:.7}; Q(t,Te) = {survival_to_expiry:.7}"
        );
        eprintln!(
            "    screen tes {exercise_settle}: adjustment = {screen_clean_adjustment:.7}; expiry+settle {forward_cash_settle}: adjustment = {forward_cash_clean_adjustment:.7}"
        );
        print_og_candidate("current forward annuity", current_forward_annuity);
        print_og_candidate(
            "OpenGamma + screen exercise settlement",
            current_forward_annuity + screen_clean_adjustment,
        );
        print_og_candidate(
            "OpenGamma + expiry spot CDS cash settle",
            current_forward_annuity + forward_cash_clean_adjustment,
        );

        // Try a "joint NYSE+GBLO" via custom-built schedule manually constructed
        // from the union calendar. We can't pass a composite to
        // `premium_leg_pv_per_bp` directly through `calendar_id`, so we
        // approximate by computing the schedule manually using the union
        // calendar then summing the contributions (no-AOD only — AOD is
        // private).
        let calendars: Vec<&'static dyn finstack_core::dates::HolidayCalendar> = vec![
            calendar_by_id("nyse").expect("NYSE calendar"),
            calendar_by_id("gblo").expect("GBLO calendar"),
        ];
        let composite = CompositeCalendar::with_mode(&calendars, CompositeMode::Union);
        let mut accrual_dates_union = vec![cds_nyse.premium.start];
        let mut current = cds_nyse.premium.start;
        while current < cds_nyse.premium.end {
            current = finstack_core::dates::next_cds_date(current);
            if current <= cds_nyse.premium.end {
                accrual_dates_union.push(current);
            }
        }
        if accrual_dates_union.last() != Some(&cds_nyse.premium.end) {
            accrual_dates_union.push(cds_nyse.premium.end);
        }
        let n_union = accrual_dates_union.len();
        let mut union_no_aod = 0.0_f64;
        for (idx, window) in accrual_dates_union.windows(2).enumerate() {
            let pay = adjust(
                window[1],
                BusinessDayConvention::ModifiedFollowing,
                &composite,
            )
            .unwrap_or(window[1]);
            // accrual fraction (Act/360) — apply +1 day rule on final period
            let mut days = (window[1] - window[0]).whole_days();
            if idx + 2 == n_union && pay == cds_nyse.premium.end && days > 0 {
                days += 1;
            }
            let yf = (days as f64) / 360.0;
            let t_end = haz_arc
                .day_count()
                .year_fraction(haz_arc.base_date(), pay, dc_ctx)
                .unwrap();
            let sp_end = haz_arc.sp(t_end);
            let df_pay = disc.df_between_dates(as_of, pay).unwrap();
            union_no_aod += yf * sp_end * df_pay;
        }
        eprintln!(
            "    {:<46}  no_aod = {union_no_aod:.7}                                                  Δ_vs_BBG = {:+.5}",
            "calendar=NYSE∪GBLO (manual), no AOD",
            union_no_aod - 4.51552
        );

        // ----- 6. T-convention table ---------------------------------------------
        //
        // Hold annuity = 4.51552, F = K = 0.0058395, σ = 0.8102 fixed; vary T.
        // NPV(T) = annuity * 10M * F * (2N(0.5σ√T) - 1)
        eprintln!(
            "\n=== Black T-convention NPV table (annuity=4.51552, F=K=58.3954bp, σ=0.8102) ==="
        );
        eprintln!("  Bloomberg target NPV = 31177.82 (back-implied T ≈ 0.13455 → 48.44 days/360)");

        let strike = strike_bp / 10_000.0;
        let sigma = 0.8102_f64;
        let ra_ref = 4.51552_f64;
        let notional = 10_000_000.0_f64;
        let bloomberg_npv = 31_177.82_f64;

        let nyse_cal = calendar_by_id("nyse").expect("NYSE");
        let gblo_cal = calendar_by_id("gblo").expect("GBLO");
        let cals = vec![nyse_cal, gblo_cal];
        let composite_cal = CompositeCalendar::with_mode(&cals, CompositeMode::Union);

        let bdc = BusinessDayConvention::ModifiedFollowing;
        let bdc_prev = BusinessDayConvention::Preceding;
        let pbdc = BusinessDayConvention::ModifiedPreceding;
        let _ = (pbdc, bdc_prev);

        // Variations. Endpoints we sweep across.
        let cs_minus_1 = adjust(cash_settle - time::Duration::days(1), bdc, &composite_cal)
            .unwrap_or(cash_settle - time::Duration::days(1));
        let cs_plus_1 = adjust(cash_settle + time::Duration::days(1), bdc, &composite_cal)
            .unwrap_or(cash_settle + time::Duration::days(1));
        let xs_minus_1 = adjust(
            exercise_settle - time::Duration::days(1),
            bdc,
            &composite_cal,
        )
        .unwrap_or(exercise_settle - time::Duration::days(1));
        let xs_plus_1 = adjust(
            exercise_settle + time::Duration::days(1),
            bdc,
            &composite_cal,
        )
        .unwrap_or(exercise_settle + time::Duration::days(1));

        let cases: Vec<(&str, Date, Date, i32)> = vec![
            (
                "(cs, xs)            current ACT/360",
                cash_settle,
                exercise_settle,
                0,
            ),
            (
                "(cs, xs)            +1d (CDSW final-period rule)",
                cash_settle,
                exercise_settle,
                1,
            ),
            (
                "(cs-1d, xs)         start back one day",
                cs_minus_1,
                exercise_settle,
                0,
            ),
            (
                "(cs+1d, xs)         start forward one day",
                cs_plus_1,
                exercise_settle,
                0,
            ),
            (
                "(cs, xs-1d)         end back one day (BDC adj)",
                cash_settle,
                xs_minus_1,
                0,
            ),
            (
                "(cs, xs+1d)         end forward one day",
                cash_settle,
                xs_plus_1,
                0,
            ),
            (
                "(cs, expiry)        cash settle to expiration",
                cash_settle,
                option_expiry,
                0,
            ),
            (
                "(cs, expiry-1d)     cash settle to (expiry - 1 BD)",
                cash_settle,
                adjust(option_expiry - time::Duration::days(1), bdc, &composite_cal)
                    .unwrap_or(option_expiry),
                0,
            ),
            (
                "(cs, expiry+1d)",
                cash_settle,
                option_expiry + time::Duration::days(1),
                0,
            ),
            (
                "(trade, xs)         trade date 05/02 to xs",
                as_of,
                exercise_settle,
                0,
            ),
            (
                "(trade_adj, xs)     trade date BDC-adj 05/04 to xs",
                adjust(as_of, bdc, &composite_cal).unwrap_or(as_of),
                exercise_settle,
                0,
            ),
        ];

        // Two day-count denominators: ACT/360 (premium-leg style) and ACT/365
        // (typical option/vol annualization convention used by OVME/OVCV/OVSW).
        // CDSO is documented as Black-on-spreads with σ in lognormal vol, T in
        // years; the "year" convention is not on the screen.
        let denoms: [(f64, &str); 2] = [(360.0, "ACT/360"), (365.0, "ACT/365")];
        eprintln!(
            "  {:<48}  {:>10}  {:>10}  {:>5}  {:>9}  {:>9}  {:>10}",
            "case", "start", "end", "days", "denom", "T", "NPV (Δ)"
        );
        for (label, start, end, plus) in &cases {
            let raw_days = (*end - *start).whole_days() + i64::from(*plus);
            for (denom, denom_name) in denoms.iter() {
                let t = (raw_days as f64) / *denom;
                let d1 = 0.5 * sigma * t.sqrt();
                let two_n = 2.0 * finstack_core::math::special_functions::norm_cdf(d1) - 1.0;
                let npv = ra_ref * notional * strike * two_n;
                let diff = npv - bloomberg_npv;
                eprintln!(
                    "  {:<48}  {:>10}  {:>10}  {:>5}  {:>9}  {:>9.6}  {:>9.2} ({:+7.2})",
                    label, start, end, raw_days, denom_name, t, npv, diff,
                );
            }
        }

        // Bloomberg-implied T (back-solved from NPV via Φ⁻¹).
        let target_two_n = bloomberg_npv / (ra_ref * notional * strike);
        let n_d1 = 0.5 * (1.0 + target_two_n);
        let d1_implied = finstack_core::math::special_functions::standard_normal_inv_cdf(n_d1);
        let t_implied = (d1_implied / (0.5 * sigma)).powi(2);
        eprintln!(
            "\n  ⇒ Bloomberg-implied T (annuity=4.51552, ATM, σ=0.8102) = {t_implied:.6}  →  {:.3} days × ACT/360",
            t_implied * 360.0
        );

        // ----- 7. CS01 building blocks ------------------------------------------
        //
        // ∂F/∂S_par via numerical bump of par spreads + rebootstrap, holding the
        // synthetic CDS conventions fixed.
        let mut bumped_up = par_term.clone();
        let mut bumped_dn = par_term.clone();
        for (_, s) in bumped_up.iter_mut() {
            *s += 1.0;
        }
        for (_, s) in bumped_dn.iter_mut() {
            *s -= 1.0;
        }
        let boot_up = bootstrap_flat_hazard_pieces(&disc_arc, &bumped_up, recovery, dc, as_of);
        let boot_dn = bootstrap_flat_hazard_pieces(&disc_arc, &bumped_dn, recovery, dc, as_of);
        let haz_up = HazardCurve::builder("IBM-UP")
            .base_date(as_of)
            .recovery_rate(recovery)
            .knots(boot_up)
            .build()
            .unwrap();
        let haz_dn = HazardCurve::builder("IBM-DN")
            .base_date(as_of)
            .recovery_rate(recovery)
            .knots(boot_dn)
            .build()
            .unwrap();
        let f_up = cds_pricer
            .par_spread(&cds_nyse, disc_arc.as_ref(), &haz_up, as_of)
            .unwrap();
        let f_dn = cds_pricer
            .par_spread(&cds_nyse, disc_arc.as_ref(), &haz_dn, as_of)
            .unwrap();
        let df_ds = (f_up - f_dn) / 2.0;
        eprintln!(
            "\n  ∂F/∂S (par-spread bump ±1bp, boot-hazard re-cal): F+1bp={f_up:.4}  F-1bp={f_dn:.4}  ⇒ ∂F/∂S = {df_ds:.4}"
        );
        // Reference Bloomberg CS01 = annuity * Φ(d1) * (∂F/∂S) * notional * 1bp
        let t_cur: f64 = 48.0 / 360.0;
        let d1_cur = 0.5 * sigma * t_cur.sqrt();
        let phi_d1 = finstack_core::math::special_functions::norm_cdf(d1_cur);
        let cs01_analytical = ra_ref * phi_d1 * df_ds * notional * 1e-4;
        eprintln!(
            "  Analytic CS01 = annuity·Φ(d1)·(∂F/∂S)·N·1bp = {ra_ref}·{phi_d1:.4}·{df_ds:.4}·10M·1bp = {cs01_analytical:.2}"
        );
        eprintln!("  Bloomberg Spread DV01 = 2563.44");
        let target_df_ds = 2563.44 / (ra_ref * phi_d1 * notional * 1e-4);
        eprintln!("  Bloomberg-implied ∂F/∂S = {target_df_ds:.4}");

        let cs01_from_df_ds =
            |df_ds_candidate: f64| ra_ref * phi_d1 * df_ds_candidate * notional * 1e-4;
        let print_cs01_candidate = |label: &str, df_ds_candidate: f64| {
            let cs01 = cs01_from_df_ds(df_ds_candidate);
            eprintln!(
                "    {label:<34} ∂F/∂S={df_ds_candidate:>8.4}  CS01={cs01:>8.2}  Δ={:+.2}",
                cs01 - 2563.44
            );
        };
        print_cs01_candidate("direct forward-spread shock", 1.0);
        print_cs01_candidate("parallel par-spread bump", df_ds);

        let f_for_par_term = |terms: &[(f64, f64)], id: &str| {
            let knots = bootstrap_flat_hazard_pieces(&disc_arc, terms, recovery, dc, as_of);
            let haz = HazardCurve::builder(id)
                .base_date(as_of)
                .recovery_rate(recovery)
                .knots(knots)
                .build()
                .unwrap();
            cds_pricer
                .par_spread(&cds_nyse, disc_arc.as_ref(), &haz, as_of)
                .unwrap()
        };
        let mut bucketed_df_ds_sum = 0.0_f64;
        for idx in 0..par_term.len() {
            let mut key_up = par_term.clone();
            let mut key_dn = par_term.clone();
            key_up[idx].1 += 1.0;
            key_dn[idx].1 -= 1.0;
            let key_df_ds = (f_for_par_term(&key_up, "IBM-KEY-UP")
                - f_for_par_term(&key_dn, "IBM-KEY-DN"))
                / 2.0;
            bucketed_df_ds_sum += key_df_ds;
            if (par_term[idx].0 - 5.0).abs() <= f64::EPSILON {
                print_cs01_candidate("5Y key-rate par-spread bump", key_df_ds);
            }
            if idx + 1 == par_term.len() {
                print_cs01_candidate("maturity key-rate bump", key_df_ds);
            }
        }
        print_cs01_candidate("bucketed key-rate sum", bucketed_df_ds_sum);

        // ----- Production CDSO pricer sanity --------------------------------
        // Build the option with Bloomberg's exact dates and pin the σ override,
        // then invoke the production pricer for NPV/Vega/Theta and the metrics
        // registry for CS01/DV01.
        use crate::instruments::common_impl::parameters::CreditParams;
        use crate::instruments::common_impl::traits::Instrument;
        use crate::instruments::credit_derivatives::cds_option::parameters::CDSOptionParams;
        use crate::instruments::credit_derivatives::cds_option::CDSOption;
        let option_params = CDSOptionParams::call(
            Decimal::from_str_exact("0.0058395400").unwrap(),
            option_expiry,
            cds_maturity,
            Money::new(notional, Currency::USD),
        )
        .unwrap();
        let credit = CreditParams::corporate_standard("IBM", "IBM-USD-SENIOR-BOOT");
        let mut option = CDSOption::new(
            "DIAG-PRODUCTION",
            &option_params,
            &credit,
            "USD-S531-SWAP",
            "DIAG-VOL",
        )
        .unwrap();
        option.pricing_overrides.market_quotes.implied_volatility = Some(0.8102);
        option.cash_settlement_date = Some(cash_settle);
        option.exercise_settlement_date = Some(exercise_settle);
        option.underlying_effective_date = Some(underlying_eff);

        let production_pricer = CDSOptionPricer::default();
        let actual_f_bp = production_pricer
            .forward_spread_bp(&option, &market, as_of)
            .unwrap();
        let actual_ra = production_pricer
            .risky_annuity(&option, &market, as_of)
            .unwrap();
        eprintln!("\n=== Production CDSO pricer NPV under T-convention sweep ===");
        eprintln!("  Forward F_bp     = {actual_f_bp:.6}  (Bloomberg 58.3954)");
        eprintln!("  Risky annuity    = {actual_ra:.7}    (Bloomberg-implied 4.51552)");

        let day_count_variants = [DayCount::Act360, DayCount::Act365F];
        for dc in &day_count_variants {
            for inclusive in [false, true] {
                let mut probe = option.clone();
                probe.day_count = *dc;
                let raw_t = if inclusive {
                    dc.year_fraction(
                        cash_settle,
                        exercise_settle + time::Duration::days(1),
                        DayCountContext::default(),
                    )
                    .unwrap()
                } else {
                    dc.year_fraction(cash_settle, exercise_settle, DayCountContext::default())
                        .unwrap()
                };
                // Reproduce production NPV using credit_option_price with this T value.
                let pv = production_pricer
                    .credit_option_price(&probe, actual_f_bp, actual_ra, 0.8102, raw_t)
                    .unwrap()
                    .amount();
                eprintln!(
                    "    {:?}, inclusive={}: T = {:.6}  →  NPV = {:.2}  (Δ vs Bloomberg = {:+.2})",
                    dc,
                    inclusive,
                    raw_t,
                    pv,
                    pv - bloomberg_npv
                );
            }
        }

        let actual_npv = option.value(&market, as_of).unwrap().amount();
        let actual_t = option.black_time_to_expiry(as_of).unwrap();
        let actual_vega = option.vega(&market, as_of).unwrap();
        let actual_theta = option.theta(&market, as_of).unwrap();
        eprintln!("  → live black_time_to_expiry = {actual_t:.6}");
        eprintln!(
            "  → live NPV    = {actual_npv:.4}  (Δ {:+.2})",
            actual_npv - bloomberg_npv
        );
        eprintln!(
            "  → live Vega   = {actual_vega:.4}  (Δ {:+.4})",
            actual_vega - 381.02
        );
        eprintln!(
            "  → live Theta  = {actual_theta:.4}  (Δ {:+.4})",
            actual_theta - (-309.08)
        );

        // CS01 and DV01 — invoke through the registry-based path so we hit
        // the registered Cs01Calculator (par-spread bump + re-bootstrap) and
        // CdsOptionDv01Calculator (rate bump + re-bootstrap), the same code
        // exercised by the golden runner.
        use crate::pricer::price_instrument_json_with_metrics;
        let instrument_json = serde_json::to_string(&serde_json::json!({
            "type": "cds_option",
            "spec": option.clone(),
        }))
        .unwrap();
        let result = price_instrument_json_with_metrics(
            &instrument_json,
            &market,
            "2026-05-02",
            "black76",
            &["cs01".to_string(), "dv01".to_string()],
            None,
        )
        .unwrap();
        let cs01 = result.measures.get("cs01").copied().unwrap_or(f64::NAN);
        let dv01 = result.measures.get("dv01").copied().unwrap_or(f64::NAN);
        eprintln!(
            "  → live CS01   = {cs01:.4}  (Bloomberg 2563.44, Δ {:+.4})",
            cs01 - 2563.44
        );
        eprintln!(
            "  → live DV01   = {dv01:.4}  (Bloomberg -5.77,    Δ {:+.4})",
            dv01 - (-5.77)
        );

        let swap_quotes_pct = [
            ("1M", 3.644),
            ("2M", 3.6528),
            ("3M", 3.6583),
            ("6M", 3.672),
            ("1Y", 3.7255),
            ("2Y", 3.7125),
            ("3Y", 3.681),
            ("4Y", 3.684),
            ("5Y", 3.7115),
            ("6Y", 3.7555),
            ("7Y", 3.8045),
            ("8Y", 3.8505),
            ("9Y", 3.894),
            ("10Y", 3.937),
            ("12Y", 4.0235),
            ("15Y", 4.133),
            ("20Y", 4.221),
            ("25Y", 4.2265),
            ("30Y", 4.1935),
        ];
        let rate_calibration = DiscountCurveRateCalibration {
            index_id: "USD-SOFR-OIS".to_string(),
            currency: Currency::USD,
            quotes: swap_quotes_pct
                .iter()
                .map(|(tenor, rate_pct)| DiscountCurveRateQuote {
                    quote_type: match *tenor {
                        "1M" | "2M" | "3M" | "6M" => DiscountCurveRateQuoteType::Deposit,
                        _ => DiscountCurveRateQuoteType::Swap,
                    },
                    tenor: (*tenor).to_string(),
                    rate: rate_pct / 100.0,
                })
                .collect(),
        };
        let disc_with_calibration = disc
            .to_builder_with_id("USD-S531-SWAP")
            .rate_calibration(rate_calibration)
            .build()
            .unwrap();
        let market_with_rate_calibration = MarketContext::new()
            .insert(disc_with_calibration)
            .insert(hazard.clone());
        let quote_rebuild_result = price_instrument_json_with_metrics(
            &instrument_json,
            &market_with_rate_calibration,
            "2026-05-02",
            "black76",
            &["dv01".to_string()],
            None,
        )
        .unwrap();
        let quote_rebuild_dv01 = quote_rebuild_result
            .measures
            .get("dv01")
            .copied()
            .unwrap_or(f64::NAN);
        eprintln!(
            "  → DV01 quote-rebuild diagnostic = {quote_rebuild_dv01:.4}  (Bloomberg -5.77, Δ {:+.4}; improvement vs direct {:+.4})",
            quote_rebuild_dv01 - (-5.77),
            (quote_rebuild_dv01 - (-5.77)).abs() - (dv01 - (-5.77)).abs()
        );
    }
}
