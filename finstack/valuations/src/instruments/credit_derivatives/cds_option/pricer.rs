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
            finstack_core::dates::DayCountCtx::default(),
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
        self.credit_option_price(option, forward_spread_bp, risky_annuity, sigma, t)
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
    // Convert decimal strike to bp for CDS constructor (which expects bp)
    let spread_bp = option.strike * Decimal::new(10000, 0);
    CreditDefaultSwap::new_isda(
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
    )
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
    /// For a per-bp delta, use [`delta_per_bp`] or divide by `bp_per_unit` (10,000).
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
    /// For a per-bp² gamma, use [`gamma_per_bp`] or divide by `bp_per_unit²` (10⁸).
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

    /// Delta per basis point: sensitivity of option value to a 1bp change in forward spread.
    ///
    /// This is the market-standard unit for credit option delta on trading desks.
    /// Equals `delta(...)  / bp_per_unit`.
    #[allow(dead_code)]
    pub(crate) fn delta_per_bp(
        &self,
        option: &CDSOption,
        forward_spread_bp: f64,
        risky_annuity: f64,
        sigma: f64,
        t: f64,
    ) -> Result<f64> {
        Ok(
            self.delta(option, forward_spread_bp, risky_annuity, sigma, t)?
                / self.config.bp_per_unit,
        )
    }

    /// Gamma per basis point squared: second derivative per 1bp² change in spread.
    ///
    /// Equals `gamma(...)  / bp_per_unit²`.
    #[allow(dead_code)]
    pub(crate) fn gamma_per_bp(
        &self,
        option: &CDSOption,
        forward_spread_bp: f64,
        risky_annuity: f64,
        sigma: f64,
        t: f64,
    ) -> Result<f64> {
        Ok(
            self.gamma(option, forward_spread_bp, risky_annuity, sigma, t)?
                / (self.config.bp_per_unit * self.config.bp_per_unit),
        )
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
        Ok(crate::results::ValuationResult::stamped(
            cds_option.id(),
            as_of,
            pv,
        ))
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
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
}
