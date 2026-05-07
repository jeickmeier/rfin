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
    // on the final ACT/360 period. With `cds_act360_include_last_day` the
    // option pricer is in QuantLib `IsdaCdsEngine` parity mode, so the synthetic
    // CDS uses the matching dirty + adjusted + full-premium convention bundle.
    cds.pricing_overrides.model_config = option.pricing_overrides.model_config.clone();
    cds.valuation_convention = if cds
        .pricing_overrides
        .model_config
        .cds_act360_include_last_day
    {
        CdsValuationConvention::QuantLibIsdaParity
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
}
