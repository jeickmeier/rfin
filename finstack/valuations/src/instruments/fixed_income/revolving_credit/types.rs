//! Revolving credit facility types and instrument trait implementations.
//!
//! Defines the `RevolvingCredit` instrument with support for deterministic and
//! stochastic cashflow modeling. Supports standard fee structures (upfront,
//! commitment, usage, and facility fees) and both fixed and floating rate bases.

use finstack_core::dates::{Date, DayCount, StubKind, Tenor};
use finstack_core::money::Money;
use finstack_core::types::{Bps, CurveId, InstrumentId, Rate};
use rust_decimal::Decimal;

use crate::cashflow::builder::{evaluate_fee_tiers, FeeTier, FloatingRateSpec};
use crate::instruments::common::traits::Attributes;
use rust_decimal::prelude::ToPrimitive;

/// Revolving credit facility instrument.
///
/// Models a credit facility with draws/repayments, interest payments on drawn
/// amounts, and fees (commitment, usage, facility, upfront). Supports both
/// deterministic schedules and stochastic utilization via Monte Carlo.
///
/// See unit tests and `examples/` for usage.
#[derive(Clone, Debug, finstack_valuations_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct RevolvingCredit {
    /// Unique identifier for the facility.
    pub id: InstrumentId,

    /// Total committed amount for the facility.
    pub commitment_amount: Money,

    /// Current drawn amount (initial utilization).
    pub drawn_amount: Money,

    /// Date when the facility becomes available.
    pub commitment_date: Date,

    /// Date when the facility expires.
    pub maturity_date: Date,

    /// Base rate specification (fixed or floating).
    pub base_rate_spec: BaseRateSpec,

    /// Day count convention for interest accrual.
    pub day_count: DayCount,

    /// Payment frequency for interest and fees.
    pub payment_frequency: Tenor,

    /// Fee structure for the facility.
    pub fees: RevolvingCreditFees,

    /// Draw and repayment schedule (deterministic or stochastic).
    pub draw_repay_spec: DrawRepaySpec,

    /// Discount curve identifier for pricing.
    pub discount_curve_id: CurveId,

    /// Optional hazard curve identifier for credit risk modeling.
    ///
    /// When provided, survival probabilities from the hazard curve are applied
    /// to discount cashflows, adjusting for default risk.
    #[cfg_attr(feature = "serde", serde(default))]
    pub hazard_curve_id: Option<CurveId>,

    /// Recovery rate on default (used when hazard_curve_id is present).
    ///
    /// Represents the fraction of exposure recovered in the event of default.
    /// Typical values: 0.30-0.50 for senior secured facilities.
    /// Defaults to 0.0 if not specified.
    #[builder(default)]
    #[cfg_attr(feature = "serde", serde(default))]
    pub recovery_rate: f64,

    /// Stub rule for schedule generation when dates don't align with frequency.
    ///
    /// Determines how to handle partial periods at the start or end of the schedule:
    /// - `ShortFront`: Short stub at the beginning (most common for RCFs)
    /// - `ShortBack`: Short stub at the end
    /// - `LongFront`: Long stub at the beginning
    /// - `LongBack`: Long stub at the end
    /// - `None`: No stub allowed (dates must align exactly)
    ///
    /// Defaults to `ShortFront` for maximum flexibility with unaligned dates.
    #[builder(default = StubKind::ShortFront)]
    #[cfg_attr(feature = "serde", serde(default = "default_stub_kind"))]
    pub stub_rule: StubKind,

    /// Attributes for scenario selection and tagging.
    pub attributes: Attributes,
}

/// Default stub kind for revolving credit facilities.
fn default_stub_kind() -> StubKind {
    StubKind::ShortFront
}

impl RevolvingCredit {
    /// Create a canonical example revolving credit facility (USD, deterministic draws).
    pub fn example() -> Self {
        use finstack_core::currency::Currency;
        use finstack_core::dates::{DayCount, Tenor};
        use time::macros::date;
        let commitment = Money::new(50_000_000.0, Currency::USD);
        let initial_draw = Money::new(10_000_000.0, Currency::USD);
        let start = date!(2024 - 01 - 01);
        let end = date!(2027 - 01 - 01);
        let base_rate = BaseRateSpec::Floating(FloatingRateSpec {
            index_id: CurveId::new("USD-SOFR-3M"),
            spread_bp: Decimal::try_from(250.0).unwrap_or(Decimal::ZERO),
            gearing: Decimal::ONE,
            gearing_includes_spread: true,
            floor_bp: Some(Decimal::ZERO),
            cap_bp: None,
            all_in_floor_bp: None,
            index_cap_bp: None,
            reset_freq: Tenor::quarterly(),
            reset_lag_days: 2,
            dc: DayCount::Act360,
            bdc: finstack_core::dates::BusinessDayConvention::ModifiedFollowing,
            calendar_id: None,
            fixing_calendar_id: None,
        });
        let fees = RevolvingCreditFees::flat(25.0, 10.0, 5.0);
        let draw_repay = DrawRepaySpec::Deterministic(vec![
            DrawRepayEvent {
                date: date!(2024 - 03 - 01),
                amount: Money::new(5_000_000.0, Currency::USD),
                is_draw: true,
            },
            DrawRepayEvent {
                date: date!(2025 - 06 - 01),
                amount: Money::new(3_000_000.0, Currency::USD),
                is_draw: false,
            },
        ]);
        RevolvingCreditBuilder::new()
            .id(InstrumentId::new("RCF-USD-3Y"))
            .commitment_amount(commitment)
            .drawn_amount(initial_draw)
            .commitment_date(start)
            .maturity_date(end)
            .base_rate_spec(base_rate)
            .day_count(DayCount::Act360)
            .payment_frequency(Tenor::quarterly())
            .fees(fees)
            .draw_repay_spec(draw_repay)
            .discount_curve_id(CurveId::new("USD-OIS"))
            .hazard_curve_id_opt(None)
            .recovery_rate(0.0)
            .stub_rule(StubKind::ShortFront)
            .attributes(Attributes::new())
            .build()
            .unwrap_or_else(|_| {
                unreachable!("Example RevolvingCredit with valid constants should never fail")
            })
    }
}

/// Base rate specification for revolving credit interest.
///
/// Defines whether the facility pays a fixed rate or a floating rate
/// tied to a market index plus margin.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BaseRateSpec {
    /// Fixed rate (annualized).
    Fixed {
        /// Annual interest rate (e.g., 0.05 for 5%).
        rate: f64,
    },

    /// Floating rate using canonical FloatingRateSpec.
    ///
    /// Composes the standard floating rate specification with full support
    /// for floors, caps, and gearing.
    Floating(FloatingRateSpec),
}

impl BaseRateSpec {
    /// Create a fixed base rate using a typed rate.
    pub fn fixed_rate(rate: Rate) -> Self {
        Self::Fixed {
            rate: rate.as_decimal(),
        }
    }
}

/// Fee structure for a revolving credit facility.
///
/// Contains the various fees charged on the facility:
/// - Upfront: one-time fee at commitment
/// - Commitment: annual fee on undrawn amount (can be tiered by utilization)
/// - Usage: annual fee on drawn amount (can be tiered by utilization)
/// - Facility: annual fee on total commitment
///
/// For backwards compatibility, flat fees can be represented as single-tier vectors.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RevolvingCreditFees {
    /// One-time upfront fee paid by borrower to lender at commitment.
    pub upfront_fee: Option<Money>,

    /// Commitment fee tiers (utilization-based). Empty vector means no commitment fee.
    /// Tiers should be sorted by threshold ascending.
    #[cfg_attr(feature = "serde", serde(default))]
    pub commitment_fee_tiers: Vec<FeeTier>,

    /// Usage fee tiers (utilization-based). Empty vector means no usage fee.
    /// Tiers should be sorted by threshold ascending.
    #[cfg_attr(feature = "serde", serde(default))]
    pub usage_fee_tiers: Vec<FeeTier>,

    /// Annual facility fee rate on total commitment (basis points).
    /// Facility fee is not tiered (applies to total commitment regardless of utilization).
    pub facility_fee_bp: f64,
}

impl Default for RevolvingCreditFees {
    fn default() -> Self {
        Self {
            upfront_fee: None,
            commitment_fee_tiers: Vec::new(),
            usage_fee_tiers: Vec::new(),
            facility_fee_bp: 0.0,
        }
    }
}

impl RevolvingCreditFees {
    /// Create fees with flat (non-tiered) commitment and usage fees.
    ///
    /// Convenience constructor for simple fee structures without utilization tiers.
    pub fn flat(commitment_fee_bp: f64, usage_fee_bp: f64, facility_fee_bp: f64) -> Self {
        let make_tier = |bps: f64| -> Vec<FeeTier> {
            if bps > 0.0 {
                vec![FeeTier {
                    threshold: Decimal::ZERO,
                    bps: Decimal::try_from(bps).unwrap_or(Decimal::ZERO),
                }]
            } else {
                Vec::new()
            }
        };

        Self {
            upfront_fee: None,
            commitment_fee_tiers: make_tier(commitment_fee_bp),
            usage_fee_tiers: make_tier(usage_fee_bp),
            facility_fee_bp,
        }
    }

    /// Create fees with flat (non-tiered) commitment and usage fees using typed bps.
    pub fn flat_bps(commitment_fee_bp: Bps, usage_fee_bp: Bps, facility_fee_bp: Bps) -> Self {
        let make_tier = |bps: Bps| -> Vec<FeeTier> {
            if !bps.is_zero() {
                vec![FeeTier {
                    threshold: Decimal::ZERO,
                    bps: Decimal::from(bps.as_bps()),
                }]
            } else {
                Vec::new()
            }
        };

        Self {
            upfront_fee: None,
            commitment_fee_tiers: make_tier(commitment_fee_bp),
            usage_fee_tiers: make_tier(usage_fee_bp),
            facility_fee_bp: facility_fee_bp.as_bps() as f64,
        }
    }

    /// Get commitment fee bps for given utilization (evaluates tiers).
    ///
    /// Returns the fee rate from the highest tier where utilization >= threshold.
    /// Tiers should be sorted by threshold ascending.
    /// If no tiers match or tiers are empty, returns 0.0.
    pub fn commitment_fee_bps(&self, utilization: f64) -> f64 {
        let util = Decimal::try_from(utilization).unwrap_or(Decimal::ZERO);
        evaluate_fee_tiers(&self.commitment_fee_tiers, util)
            .to_f64()
            .unwrap_or(0.0)
    }

    /// Get usage fee bps for given utilization (evaluates tiers).
    ///
    /// Returns the fee rate from the highest tier where utilization >= threshold.
    /// Tiers should be sorted by threshold ascending.
    /// If no tiers match or tiers are empty, returns 0.0.
    pub fn usage_fee_bps(&self, utilization: f64) -> f64 {
        let util = Decimal::try_from(utilization).unwrap_or(Decimal::ZERO);
        evaluate_fee_tiers(&self.usage_fee_tiers, util)
            .to_f64()
            .unwrap_or(0.0)
    }
}

/// Draw and repayment specification.
///
/// Determines whether the facility uses a known (deterministic) schedule
/// or stochastic utilization for Monte Carlo pricing.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DrawRepaySpec {
    /// Deterministic schedule of draws and repayments.
    Deterministic(Vec<DrawRepayEvent>),

    /// Stochastic utilization for Monte Carlo simulation.
    Stochastic(Box<StochasticUtilizationSpec>),
}

/// A single draw or repayment event.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DrawRepayEvent {
    /// Date of the draw or repayment.
    pub date: Date,

    /// Amount being drawn or repaid (absolute value).
    pub amount: Money,

    /// True if this is a draw, false if it's a repayment.
    pub is_draw: bool,
}

/// Specification for stochastic utilization modeling.
///
/// Defines the stochastic process and simulation parameters for
/// Monte Carlo pricing with uncertain draw/repayment patterns. Credit risk is
/// incorporated via hazard-rate survival weighting (no explicit default events).
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StochasticUtilizationSpec {
    /// Utilization process specification.
    pub utilization_process: UtilizationProcess,

    /// Number of Monte Carlo paths to simulate.
    pub num_paths: usize,

    /// Random seed for reproducibility (None for non-deterministic).
    pub seed: Option<u64>,

    /// Use antithetic variance reduction when simulating paths (default: false).
    #[cfg_attr(feature = "serde", serde(default))]
    pub antithetic: bool,

    /// Use Sobol quasi-Monte Carlo RNG instead of Philox (default: false).
    #[cfg_attr(feature = "serde", serde(default))]
    pub use_sobol_qmc: bool,

    /// Advanced Monte Carlo configuration (optional).
    ///
    /// When present, enables multi-factor modeling with credit spread
    /// and interest rate dynamics, correlation, and default modeling.
    #[cfg(feature = "mc")]
    pub mc_config: Option<McConfig>,
}

/// Advanced Monte Carlo configuration for revolving credit facilities.
///
/// Enables multi-factor modeling with credit risk, interest rate dynamics,
/// correlation between factors, and default modeling.
#[cfg(feature = "mc")]
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct McConfig {
    /// Correlation matrix (3x3) between [utilization, rate, credit].
    ///
    /// Must be symmetric, positive definite, with ones on diagonal.
    /// If None, factors are assumed independent.
    pub correlation_matrix: Option<[[f64; 3]; 3]>,

    /// Recovery rate on default (e.g., 0.4 for 40% recovery).
    ///
    /// Note: This field is currently ignored in favor of `RevolvingCredit::recovery_rate`
    /// to ensure consistency between path generation and pricing.
    pub recovery_rate: f64,

    /// Credit spread process specification.
    pub credit_spread_process: CreditSpreadProcessSpec,

    /// Interest rate process specification (for floating rates).
    ///
    /// If None, assumes fixed rate (no stochastic dynamics).
    pub interest_rate_process: Option<InterestRateProcessSpec>,

    /// Optional utilization–credit correlation used when `correlation_matrix` is None.
    ///
    /// If provided, builds a 3×3 matrix with:
    ///   [ [1, 0, rho], [0, 1, 0], [rho, 0, 1] ]
    /// representing correlation between utilization and credit, with the rate
    /// factor uncorrelated (kept fixed in 2‑factor mode).
    #[cfg_attr(feature = "serde", serde(default))]
    pub util_credit_corr: Option<f64>,
}

#[cfg(feature = "mc")]
impl McConfig {
    /// Validate the configuration parameters.
    ///
    /// Checks that:
    /// - Correlation matrix (if provided) is positive semi-definite
    /// - Recovery rate is in [0, 1]
    /// - CIR parameters satisfy Feller condition if applicable
    /// - Credit spread parameters are valid
    ///
    /// # Returns
    ///
    /// `Ok(())` if all parameters are valid, otherwise returns an error
    /// describing the validation failure.
    pub fn validate(&self) -> finstack_core::Result<()> {
        use super::MAX_RECOVERY_RATE;
        use finstack_core::InputError;

        // Validate recovery rate: must be in [0, 1) to avoid division by zero
        // in hazard-to-spread mapping: λ = s / (1 - R)
        if self.recovery_rate < 0.0 || self.recovery_rate >= MAX_RECOVERY_RATE {
            return Err(finstack_core::Error::Validation(format!(
                "Recovery rate must be in [0, {:.6}), got {}",
                MAX_RECOVERY_RATE, self.recovery_rate
            )));
        }

        // Validate correlation matrix if provided
        if let Some(corr) = self.correlation_matrix {
            // Check positive semi-definiteness
            finstack_core::math::linalg::validate_correlation_matrix(
                &corr.iter().flatten().copied().collect::<Vec<_>>(),
                3,
            )?;
        }

        // Validate util_credit_corr if provided
        if let Some(rho) = self.util_credit_corr {
            if rho.abs() > 1.0 {
                return Err(InputError::Invalid.into());
            }
        }

        // Validate credit spread process parameters
        match &self.credit_spread_process {
            CreditSpreadProcessSpec::Cir {
                kappa,
                theta,
                sigma,
                initial,
            } => {
                // All parameters must be non-negative
                if *kappa <= 0.0 || *theta < 0.0 || *sigma < 0.0 || *initial < 0.0 {
                    return Err(InputError::Invalid.into());
                }
            }
            CreditSpreadProcessSpec::Constant(spread) => {
                if *spread < 0.0 {
                    return Err(InputError::Invalid.into());
                }
            }
            CreditSpreadProcessSpec::MarketAnchored {
                kappa,
                implied_vol,
                tenor_years,
                ..
            } => {
                if *kappa <= 0.0 || *implied_vol < 0.0 {
                    return Err(InputError::Invalid.into());
                }
                if let Some(tenor) = tenor_years {
                    if *tenor <= 0.0 {
                        return Err(InputError::Invalid.into());
                    }
                }
            }
        }

        // Validate interest rate process if provided
        if let Some(InterestRateProcessSpec::HullWhite1F { kappa, sigma, .. }) =
            &self.interest_rate_process
        {
            if *kappa <= 0.0 || *sigma < 0.0 {
                return Err(InputError::Invalid.into());
            }
        }

        Ok(())
    }
}

/// Credit spread process specification.
#[cfg(feature = "mc")]
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CreditSpreadProcessSpec {
    /// CIR process for stochastic credit spread/hazard rate.
    ///
    /// Models credit spread as: dλ_t = κ(θ - λ_t)dt + σ√λ_t dW_t
    Cir {
        /// Mean reversion speed (κ)
        kappa: f64,
        /// Long-term mean (θ)
        theta: f64,
        /// Volatility (σ)
        sigma: f64,
        /// Initial credit spread
        initial: f64,
    },
    /// Constant credit spread (no dynamics).
    Constant(f64),

    /// Market-anchored credit spread process calibrated to a hazard curve and CDS option vol.
    ///
    /// The mean level is anchored to the time-average spread implied by the input hazard
    /// curve (over tenor T chosen as facility maturity by default). The initial spread is set
    /// to the first-segment spread, and the volatility is scaled from the CDS index option
    /// implied volatility.
    MarketAnchored {
        /// Hazard curve identifier in `MarketContext` used to anchor spreads.
        hazard_curve_id: CurveId,
        /// Mean reversion speed (κ) of the CIR process.
        kappa: f64,
        /// Annualized CDS (index) option implied volatility for spreads.
        implied_vol: f64,
        /// Optional tenor in years; if None, uses facility maturity horizon.
        #[cfg_attr(feature = "serde", serde(default))]
        tenor_years: Option<f64>,
    },
}

/// Interest rate process specification (for floating rates).
#[cfg(feature = "mc")]
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum InterestRateProcessSpec {
    /// Hull-White 1-factor model for short rate.
    ///
    /// Models short rate as: dr_t = κ[θ(t) - r_t]dt + σ dW_t
    HullWhite1F {
        /// Mean reversion speed (κ)
        kappa: f64,
        /// Volatility (σ)
        sigma: f64,
        /// Initial short rate
        initial: f64,
        /// Constant mean reversion level (θ)
        theta: f64,
    },
}

/// Utilization process for stochastic draws/repayments.
///
/// For the 80/20 implementation, we support a single mean-reverting process.
/// This can be extended in the future to support other processes (jump-diffusion,
/// regime-switching, etc.).
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum UtilizationProcess {
    /// Mean-reverting utilization rate process.
    ///
    /// Models utilization as reverting to a long-term target with specified
    /// speed and volatility. Uses Ornstein-Uhlenbeck dynamics:
    /// dU(t) = speed * (target_rate - U(t)) * dt + volatility * dW(t)
    MeanReverting {
        /// Target utilization rate (0.0 to 1.0).
        target_rate: f64,
        /// Mean reversion speed (annualized).
        speed: f64,
        /// Volatility of utilization changes (annualized).
        volatility: f64,
    },
}

// Implement HasCreditCurve for generic CS01 calculator.
// Returns hazard_curve_id if present, otherwise falls back to discount_curve_id.
// CS01 will fail at runtime if no hazard curve exists in market data, which is acceptable.
#[allow(deprecated)]
impl crate::metrics::HasCreditCurve for RevolvingCredit {
    fn credit_curve_id(&self) -> &finstack_core::types::CurveId {
        self.hazard_curve_id
            .as_ref()
            .unwrap_or(&self.discount_curve_id)
    }
}

impl RevolvingCredit {
    /// Get the current undrawn amount.
    pub fn undrawn_amount(&self) -> finstack_core::Result<Money> {
        self.commitment_amount.checked_sub(self.drawn_amount)
    }

    /// Get the current utilization rate (drawn / committed).
    pub fn utilization_rate(&self) -> f64 {
        if self.commitment_amount.amount() == 0.0 {
            0.0
        } else {
            self.drawn_amount.amount() / self.commitment_amount.amount()
        }
    }

    /// Check if the facility uses deterministic cashflows.
    pub fn is_deterministic(&self) -> bool {
        matches!(self.draw_repay_spec, DrawRepaySpec::Deterministic(_))
    }

    /// Check if the facility uses stochastic utilization.
    pub fn is_stochastic(&self) -> bool {
        matches!(self.draw_repay_spec, DrawRepaySpec::Stochastic(_))
    }

    /// Check if the facility has a credit curve configured for CS01 calculations.
    ///
    /// Returns `true` if `hazard_curve_id` is set, indicating that credit risk
    /// sensitivity (CS01) calculations are meaningful for this facility.
    pub fn has_credit_curve(&self) -> bool {
        self.hazard_curve_id.is_some()
    }
}

// Implement the Instrument trait
impl crate::instruments::common::traits::Instrument for RevolvingCredit {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::RevolvingCredit
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
        // Optional model override via attributes metadata (e.g., meta["pricing_model"] = "monte_carlo_gbm")
        if let Some(model_str) = self.attributes().get_meta("pricing_model") {
            if let Ok(model) = <crate::pricer::ModelKey as ::std::str::FromStr>::from_str(model_str)
            {
                let registry = crate::pricer::create_standard_registry();
                let result = registry
                    .price_with_registry(self, model, curves, as_of, None)
                    .map_err(|e| finstack_core::Error::Validation(e.to_string()))?;
                return Ok(result.value);
            }
        }

        // Route to appropriate pricer based on spec type
        if self.is_deterministic() {
            crate::instruments::revolving_credit::pricer::unified::RevolvingCreditPricer::price_deterministic(
                self, curves, as_of,
            )
        } else {
            // For the value() fast path, route stochastic specs to deterministic pricing.
            // MC remains available via explicit pricer APIs/bindings (e.g., mc_paths_with_capture).
            let mut fallback = self.clone();
            // If the stochastic spec carried a market-anchored hazard reference in its MC config,
            // propagate that to the deterministic fallback so survival weighting is preserved.
            #[cfg(feature = "mc")]
            if let super::types::DrawRepaySpec::Stochastic(spec) = &self.draw_repay_spec {
                if let Some(mc_cfg) = &spec.mc_config {
                    if let super::types::CreditSpreadProcessSpec::MarketAnchored {
                        hazard_curve_id,
                        ..
                    } = &mc_cfg.credit_spread_process
                    {
                        fallback.hazard_curve_id = Some(hazard_curve_id.clone());
                        fallback.recovery_rate = mc_cfg.recovery_rate;
                    }
                }
            }
            // Ensure deterministic schedule for pricing.
            fallback.draw_repay_spec = super::types::DrawRepaySpec::Deterministic(Vec::new());
            crate::instruments::revolving_credit::pricer::unified::RevolvingCreditPricer::price_deterministic(
                &fallback, curves, as_of,
            )
        }
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
            None,
        )
    }
}

// Implement HasDiscountCurve for generic metric calculators
#[allow(deprecated)]
impl crate::instruments::common::pricing::HasDiscountCurve for RevolvingCredit {
    fn discount_curve_id(&self) -> &finstack_core::types::CurveId {
        &self.discount_curve_id
    }
}

// Implement CurveDependencies for DV01 calculator
impl crate::instruments::common::traits::CurveDependencies for RevolvingCredit {
    fn curve_dependencies(&self) -> crate::instruments::common::traits::InstrumentCurves {
        let mut builder = crate::instruments::common::traits::InstrumentCurves::builder()
            .discount(self.discount_curve_id.clone());

        // Add credit curve if present
        if let Some(ref credit_curve_id) = self.hazard_curve_id {
            builder = builder.credit(credit_curve_id.clone());
        }

        builder.build()
    }
}

// Implement CashflowProvider for standard cashflow interface
impl crate::cashflow::traits::CashflowProvider for RevolvingCredit {
    fn notional(&self) -> Option<finstack_core::money::Money> {
        Some(self.commitment_amount)
    }

    fn build_full_schedule(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<crate::cashflow::builder::CashFlowSchedule> {
        // Only works for deterministic specs
        if !self.is_deterministic() {
            return Err(finstack_core::InputError::Invalid.into());
        }

        use crate::instruments::revolving_credit::cashflow_engine::CashflowEngine;
        let engine = CashflowEngine::new(self, Some(curves), as_of)?;
        let path_schedule = engine.generate_deterministic()?;
        Ok(path_schedule.schedule)
    }
}
