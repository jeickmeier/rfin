//! CdsOption instrument: option on a CDS spread.
//!
//! This module defines the `CdsOption` data structure and integrates with the
//! common instrument trait via `impl_instrument!`. All pricing math and metrics
//! are implemented in the `pricing/` and `metrics/` submodules.
//!
//! # Validation
//!
//! `CdsOption::try_new` validates all inputs at construction time:
//! - Strike spread must be positive (≤0 is invalid)
//! - Option expiry must precede underlying CDS maturity
//! - Recovery rate must be in (0, 1)
//! - Index factor must be in (0, 1] when specified
//! - Implied volatility override must be in (0, 5] when specified
//!
//! # Volatility Convention
//!
//! All volatilities are expressed as **lognormal (Black) volatility** in decimal form.
//! For example, 30% volatility is represented as 0.30.

use crate::instruments::common::parameters::CreditParams;
use crate::instruments::common::traits::Attributes;
use crate::instruments::PricingOverrides;
use crate::instruments::{ExerciseStyle, OptionType, SettlementType};
use finstack_core::dates::{Date, DayCount, DayCountCtx};
use finstack_core::money::Money;
use finstack_core::types::{InstrumentId, Percentage};

use super::parameters::CdsOptionParams;

/// Minimum valid recovery rate (exclusive lower bound).
pub const MIN_RECOVERY_RATE: f64 = 0.0;
/// Maximum valid recovery rate (exclusive upper bound).
pub const MAX_RECOVERY_RATE: f64 = 1.0;
/// Minimum valid implied volatility (exclusive lower bound).
pub const MIN_IMPLIED_VOL: f64 = 0.0;
/// Maximum valid implied volatility (inclusive upper bound).
/// 500% lognormal vol is extremely high but theoretically valid.
pub const MAX_IMPLIED_VOL: f64 = 5.0;

/// Credit option instrument (option on CDS spread)
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct CdsOption {
    /// Unique instrument identifier
    pub id: InstrumentId,
    /// Strike spread in basis points
    pub strike_spread_bp: f64,
    /// Option type (Call = right to buy protection, Put = right to sell protection)
    pub option_type: OptionType,
    /// Exercise style
    pub exercise_style: ExerciseStyle,
    /// Option expiry date
    pub expiry: Date,
    /// Underlying CDS maturity date
    pub cds_maturity: Date,
    /// Day count convention for time calculations
    pub day_count: DayCount,
    /// Notional amount
    pub notional: Money,
    /// Settlement type
    pub settlement: SettlementType,
    /// Recovery rate assumption
    pub recovery_rate: f64,
    /// Discount curve identifier
    pub discount_curve_id: finstack_core::types::CurveId,
    /// Credit curve identifier
    pub credit_curve_id: finstack_core::types::CurveId,
    /// Volatility surface identifier
    pub vol_surface_id: finstack_core::types::CurveId,
    /// Pricing overrides (including implied volatility)
    pub pricing_overrides: PricingOverrides,
    /// Additional attributes
    pub attributes: Attributes,
    /// If true, the underlying is a CDS index; else single-name CDS
    pub underlying_is_index: bool,
    /// Optional index factor scaling for index underlying
    pub index_factor: Option<f64>,
    /// Forward spread adjustment (bp) to apply for forward computation
    pub forward_spread_adjust_bp: f64,
}

// Implement HasCreditCurve for generic CS01 calculator
impl crate::metrics::HasCreditCurve for CdsOption {
    fn credit_curve_id(&self) -> &finstack_core::types::CurveId {
        &self.credit_curve_id
    }
}

impl CdsOption {
    /// Validate the CdsOption parameters.
    fn validate(&self) -> finstack_core::Result<()> {
        // Strike validation
        if self.strike_spread_bp <= super::parameters::MIN_STRIKE_SPREAD_BP {
            return Err(finstack_core::Error::Validation(format!(
                "strike_spread_bp must be positive, got {}",
                self.strike_spread_bp
            )));
        }
        if self.strike_spread_bp > super::parameters::MAX_STRIKE_SPREAD_BP {
            return Err(finstack_core::Error::Validation(format!(
                "strike_spread_bp {} exceeds maximum {} bp",
                self.strike_spread_bp,
                super::parameters::MAX_STRIKE_SPREAD_BP
            )));
        }

        // Date validation
        if self.expiry >= self.cds_maturity {
            return Err(finstack_core::Error::Validation(format!(
                "option expiry ({}) must be before CDS maturity ({})",
                self.expiry, self.cds_maturity
            )));
        }

        // Recovery rate validation
        if self.recovery_rate <= MIN_RECOVERY_RATE || self.recovery_rate >= MAX_RECOVERY_RATE {
            return Err(finstack_core::Error::Validation(format!(
                "recovery_rate must be in (0, 1), got {}",
                self.recovery_rate
            )));
        }

        // Index factor validation
        if let Some(factor) = self.index_factor {
            if factor <= 0.0 || factor > 1.0 {
                return Err(finstack_core::Error::Validation(format!(
                    "index_factor must be in (0, 1], got {}",
                    factor
                )));
            }
        }

        // Implied volatility override validation
        if let Some(vol) = self.pricing_overrides.implied_volatility {
            if vol <= MIN_IMPLIED_VOL {
                return Err(finstack_core::Error::Validation(format!(
                    "implied_volatility must be positive, got {}",
                    vol
                )));
            }
            if vol > MAX_IMPLIED_VOL {
                tracing::warn!(
                    implied_vol = vol,
                    max_vol = MAX_IMPLIED_VOL,
                    "Implied volatility {} exceeds typical maximum {}. This may indicate a data error.",
                    vol,
                    MAX_IMPLIED_VOL
                );
            }
        }

        Ok(())
    }

    /// Create a canonical example CDS option (call on CDS spread).
    #[must_use]
    pub fn example() -> Self {
        use finstack_core::currency::Currency;
        use time::macros::date;
        let option_params = CdsOptionParams::call(
            100.0,
            date!(2025 - 06 - 20),
            date!(2030 - 06 - 20),
            Money::new(10_000_000.0, Currency::USD),
        )
        .unwrap_or_else(|_| {
            unreachable!("Example CdsOptionParams with valid constants should never fail")
        });
        let credit_params =
            crate::instruments::common::parameters::CreditParams::corporate_standard(
                "CORP",
                "CORP-HAZARD",
            );
        CdsOption::new(
            InstrumentId::new("CDSOPT-CALL-CORP-5Y"),
            &option_params,
            &credit_params,
            "USD-OIS",
            "CDSOPT-VOL",
        )
        .unwrap_or_else(|_| {
            unreachable!("Example CdsOption with valid constants should never fail")
        })
    }

    /// Create a new credit option using parameter structs with validation.
    ///
    /// # Arguments
    ///
    /// - `id`: Unique instrument identifier
    /// - `option_params`: deal-level fields (strike in bp, expiry, CDS maturity, notional, option type)
    /// - `credit_params`: reference entity, recovery rate, and the hazard `credit_id`
    /// - `discount_curve_id`: discount curve identifier for discounting cashflows
    /// - `vol_surface_id`: volatility surface identifier for the CDS option
    ///
    /// # Errors
    ///
    /// Returns an error if any validation fails. See [`CdsOptionParams`] for parameter constraints.
    pub fn new(
        id: impl Into<InstrumentId>,
        option_params: &CdsOptionParams,
        credit_params: &CreditParams,
        discount_curve_id: impl Into<finstack_core::types::CurveId>,
        vol_surface_id: impl Into<finstack_core::types::CurveId>,
    ) -> finstack_core::Result<Self> {
        let option = Self {
            id: id.into(),
            strike_spread_bp: option_params.strike_spread_bp,
            option_type: option_params.option_type,
            exercise_style: ExerciseStyle::European,
            expiry: option_params.expiry,
            cds_maturity: option_params.cds_maturity,
            day_count: option_params.day_count,
            notional: option_params.notional,
            settlement: SettlementType::Cash,
            recovery_rate: credit_params.recovery_rate,
            discount_curve_id: discount_curve_id.into(),
            credit_curve_id: credit_params.credit_curve_id.to_owned(),
            vol_surface_id: vol_surface_id.into(),
            pricing_overrides: PricingOverrides::default(),
            attributes: Attributes::new(),
            underlying_is_index: option_params.underlying_is_index,
            index_factor: option_params.index_factor,
            forward_spread_adjust_bp: option_params.forward_spread_adjust_bp,
        };
        option.validate()?;
        Ok(option)
    }

    /// Set implied volatility override with validation.
    ///
    /// # Arguments
    ///
    /// * `vol` - Lognormal (Black) volatility in decimal form (e.g., 0.30 for 30%)
    ///
    /// # Errors
    ///
    /// Returns an error if volatility is not positive.
    pub fn with_implied_vol(mut self, vol: f64) -> finstack_core::Result<Self> {
        if vol <= MIN_IMPLIED_VOL {
            return Err(finstack_core::Error::Validation(format!(
                "implied_volatility must be positive, got {}",
                vol
            )));
        }
        self.pricing_overrides.implied_volatility = Some(vol);
        Ok(self)
    }

    /// Set implied volatility override using a typed percentage.
    ///
    /// # Errors
    ///
    /// Returns an error if volatility is not positive.
    pub fn with_implied_vol_pct(mut self, vol: Percentage) -> finstack_core::Result<Self> {
        let vol_decimal = vol.as_decimal();
        if vol_decimal <= MIN_IMPLIED_VOL {
            return Err(finstack_core::Error::Validation(format!(
                "implied_volatility must be positive, got {}",
                vol_decimal
            )));
        }
        self.pricing_overrides.implied_volatility = Some(vol_decimal);
        Ok(self)
    }

    /// Calculate the net present value of this CDS option
    pub fn npv(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        let pricer = crate::instruments::cds_option::pricer::CdsOptionPricer::default();
        pricer.npv(self, curves, as_of)
    }

    /// Extract common pricing inputs for Greek calculations.
    ///
    /// This helper consolidates the repeated logic for computing:
    /// - Time to expiry (t)
    /// - Forward spread (fwd_bp)
    /// - Implied volatility (sigma)
    /// - Risky annuity
    ///
    /// Returns `None` if the option has expired (t <= 0).
    fn pricing_inputs(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<Option<CdsOptionPricingInputs>> {
        let ctx = DayCountCtx::default();

        // Time to expiry
        let t = self.day_count.year_fraction(as_of, self.expiry, ctx)?;
        if t <= 0.0 {
            return Ok(None);
        }

        // Forward spread in bp
        let hazard_curve = curves.get_hazard_ref(&self.credit_curve_id)?;
        let current_tenor = self
            .day_count
            .year_fraction(as_of, self.cds_maturity, ctx)?;
        let fwd_bp = if current_tenor > 0.0 {
            use finstack_core::market_data::term_structures::ParInterp;
            hazard_curve.quoted_spread_bp(current_tenor, ParInterp::Linear)
        } else {
            self.strike_spread_bp
        };

        // Volatility (use override if present, else surface)
        let sigma = if let Some(v) = self.pricing_overrides.implied_volatility {
            v
        } else {
            curves
                .surface_ref(self.vol_surface_id.as_str())?
                .value_clamped(t, self.strike_spread_bp)
        };

        // Risky annuity
        let pricer = crate::instruments::cds_option::pricer::CdsOptionPricer::default();
        let risky_annuity = pricer.risky_annuity(self, curves, as_of)?;

        Ok(Some(CdsOptionPricingInputs {
            t,
            fwd_bp,
            sigma,
            risky_annuity,
        }))
    }

    /// Calculate delta of this CDS option.
    ///
    /// Delta measures the sensitivity of the option value to changes in the forward spread.
    /// Returns the dollar value change per 100% change in spread.
    pub fn delta(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<f64> {
        let Some(inputs) = self.pricing_inputs(curves, as_of)? else {
            return Ok(0.0);
        };

        let pricer = crate::instruments::cds_option::pricer::CdsOptionPricer::default();
        let delta = pricer.delta(
            self,
            inputs.fwd_bp,
            inputs.risky_annuity,
            inputs.sigma,
            inputs.t,
        );
        Ok(delta * self.notional.amount())
    }

    /// Calculate gamma of this CDS option.
    ///
    /// Gamma measures the rate of change of delta with respect to the forward spread.
    pub fn gamma(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<f64> {
        let Some(inputs) = self.pricing_inputs(curves, as_of)? else {
            return Ok(0.0);
        };

        let pricer = crate::instruments::cds_option::pricer::CdsOptionPricer::default();
        let gamma = pricer.gamma(
            self,
            inputs.fwd_bp,
            inputs.risky_annuity,
            inputs.sigma,
            inputs.t,
        );
        Ok(gamma * self.notional.amount())
    }

    /// Calculate vega of this CDS option.
    ///
    /// Vega measures the sensitivity of the option value to changes in implied volatility.
    /// Returns the dollar value change per 1% change in volatility.
    pub fn vega(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<f64> {
        let Some(inputs) = self.pricing_inputs(curves, as_of)? else {
            return Ok(0.0);
        };

        let pricer = crate::instruments::cds_option::pricer::CdsOptionPricer::default();
        let vega = pricer.vega(
            self,
            inputs.fwd_bp,
            inputs.risky_annuity,
            inputs.sigma,
            inputs.t,
        );
        Ok(vega * self.notional.amount())
    }

    /// Calculate theta of this CDS option using finite differences.
    ///
    /// Theta measures the sensitivity of the option value to the passage of time.
    /// This implementation uses a full finite-difference approach that captures
    /// both the Black formula time decay and the risky annuity decay.
    ///
    /// # Returns
    ///
    /// The dollar value change per day (negative for long positions).
    pub fn theta(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<f64> {
        let pricer = crate::instruments::cds_option::pricer::CdsOptionPricer::default();
        pricer.theta_finite_diff(self, curves, as_of)
    }

    /// Calculate implied volatility of this CDS option.
    ///
    /// Solves for the Black volatility σ such that model price(σ) = target_price.
    ///
    /// # Arguments
    ///
    /// * `curves` - Market context with discount and hazard curves
    /// * `as_of` - Valuation date
    /// * `target_price` - The observed market price to match
    /// * `initial_guess` - Optional starting point for the solver (defaults to surface vol or 20%)
    ///
    /// # Returns
    ///
    /// The implied lognormal volatility in decimal form (e.g., 0.30 for 30%).
    pub fn implied_vol(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
        target_price: f64,
        initial_guess: Option<f64>,
    ) -> finstack_core::Result<f64> {
        let pricer = crate::instruments::cds_option::pricer::CdsOptionPricer::default();
        pricer.implied_vol(self, curves, as_of, target_price, initial_guess)
    }
}

/// Common pricing inputs for CDS option Greeks calculations.
///
/// This struct consolidates the computed market inputs needed by all Greek methods,
/// eliminating code duplication while maintaining clear ownership of the computation.
#[derive(Debug, Clone, Copy)]
pub struct CdsOptionPricingInputs {
    /// Time to expiry in years
    pub t: f64,
    /// Forward CDS spread in basis points
    pub fwd_bp: f64,
    /// Implied volatility (lognormal, decimal)
    pub sigma: f64,
    /// Risky annuity (RPV01) in years
    pub risky_annuity: f64,
}

impl crate::instruments::common::traits::Instrument for CdsOption {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::CDSOption
    }

    fn as_any(&self) -> &dyn ::std::any::Any {
        self
    }

    fn attributes(&self) -> &crate::instruments::common::traits::Attributes {
        &self.attributes
    }

    fn attributes_mut(&mut self) -> &mut crate::instruments::common::traits::Attributes {
        &mut self.attributes
    }

    fn clone_box(&self) -> Box<dyn crate::instruments::common::traits::Instrument> {
        Box::new(self.clone())
    }

    fn value(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        self.npv(curves, as_of)
    }

    fn price_with_metrics(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
        metrics: &[crate::metrics::MetricId],
    ) -> finstack_core::Result<crate::results::ValuationResult> {
        let base_value = self.value(curves, as_of)?;
        crate::instruments::common::helpers::build_with_metrics_dyn(
            std::sync::Arc::new(self.clone()),
            std::sync::Arc::new(curves.clone()),
            as_of,
            base_value,
            metrics,
            None,
        )
    }
}

impl crate::instruments::common::pricing::HasDiscountCurve for CdsOption {
    fn discount_curve_id(&self) -> &finstack_core::types::CurveId {
        &self.discount_curve_id
    }
}

// Implement CurveDependencies for DV01 calculator
impl crate::instruments::common::traits::CurveDependencies for CdsOption {
    fn curve_dependencies(&self) -> crate::instruments::common::traits::InstrumentCurves {
        crate::instruments::common::traits::InstrumentCurves::builder()
            .discount(self.discount_curve_id.clone())
            .credit(self.credit_curve_id.clone())
            .build()
    }
}
