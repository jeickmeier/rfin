//! Revolving credit facility types and instrument trait implementations.
//!
//! Defines the `RevolvingCredit` instrument with support for deterministic and
//! stochastic cashflow modeling. Supports standard fee structures (upfront,
//! commitment, usage, and facility fees) and both fixed and floating rate bases.

use finstack_core::dates::{Date, DayCount, Frequency};
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};

use crate::instruments::common::traits::Attributes;

/// Revolving credit facility instrument.
///
/// Models a credit facility with draws/repayments, interest payments on drawn
/// amounts, and fees (commitment, usage, facility, upfront). Supports both
/// deterministic schedules and stochastic utilization via Monte Carlo.
///
/// See unit tests and `examples/` for usage.
#[derive(Clone, Debug, finstack_valuations_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
    pub payment_frequency: Frequency,

    /// Fee structure for the facility.
    pub fees: RevolvingCreditFees,

    /// Draw and repayment schedule (deterministic or stochastic).
    pub draw_repay_spec: DrawRepaySpec,

    /// Discount curve identifier for pricing.
    pub discount_curve_id: CurveId,

    /// Attributes for scenario selection and tagging.
    pub attributes: Attributes,
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

    /// Floating rate linked to an index.
    Floating {
        /// Forward curve identifier for the floating index (e.g., USD-SOFR-3M).
        index_id: CurveId,
        /// Margin over the index in basis points.
        margin_bp: f64,
        /// Reset frequency for rate fixings.
        reset_freq: Frequency,
        /// Optional floor on the base rate in basis points (applies to base only, before margin).
        /// E.g., floor_bp = Some(0.0) enforces a 0% floor on the index rate.
        #[cfg_attr(feature = "serde", serde(default))]
        floor_bp: Option<f64>,
    },
}

/// Fee tier for utilization-based fee structures.
///
/// Tiers are evaluated in order: the first tier where utilization >= threshold applies.
/// Tiers should be sorted by threshold (ascending).
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FeeTier {
    /// Utilization threshold (0.0 to 1.0). Fee applies when utilization >= this threshold.
    pub threshold: f64,
    /// Fee rate in basis points for this tier.
    pub bps: f64,
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
                    threshold: 0.0,
                    bps,
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

    /// Evaluate fee tiers to find the applicable rate for a given utilization.
    ///
    /// Returns the fee rate from the highest tier where utilization >= threshold.
    /// Tiers should be sorted by threshold ascending.
    /// If no tiers match or tiers are empty, returns 0.0.
    fn fee_bps_for_tier(tiers: &[FeeTier], utilization: f64) -> f64 {
        tiers
            .iter()
            .rev()
            .find(|tier| utilization >= tier.threshold)
            .map(|tier| tier.bps)
            .unwrap_or(0.0)
    }

    /// Get commitment fee bps for given utilization (evaluates tiers).
    ///
    /// Returns the fee rate from the highest tier where utilization >= threshold.
    /// Tiers should be sorted by threshold ascending.
    /// If no tiers match or tiers are empty, returns 0.0.
    pub fn commitment_fee_bps(&self, utilization: f64) -> f64 {
        Self::fee_bps_for_tier(&self.commitment_fee_tiers, utilization)
    }

    /// Get usage fee bps for given utilization (evaluates tiers).
    ///
    /// Returns the fee rate from the highest tier where utilization >= threshold.
    /// Tiers should be sorted by threshold ascending.
    /// If no tiers match or tiers are empty, returns 0.0.
    pub fn usage_fee_bps(&self, utilization: f64) -> f64 {
        Self::fee_bps_for_tier(&self.usage_fee_tiers, utilization)
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
/// Monte Carlo pricing with uncertain draw/repayment patterns.
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

    /// Optional simple default model used when `mc_config` is None.
    /// If provided, integrates a constant hazard/spread with recovery into simple MC.
    #[cfg_attr(feature = "serde", serde(default))]
    pub default_model: Option<SimpleDefaultSpec>,

    /// Advanced Monte Carlo configuration (optional).
    ///
    /// When present, enables multi-factor modeling with credit spread
    /// and interest rate dynamics, correlation, and default modeling.
    #[cfg(feature = "mc")]
    pub mc_config: Option<McConfig>,
}

/// Simple default model for utilization-only Monte Carlo.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SimpleDefaultSpec {
    /// Annual hazard rate (e.g., 0.02). If None, derived from `annual_spread` and `recovery_rate`.
    #[cfg_attr(feature = "serde", serde(default))]
    pub annual_hazard: Option<f64>,
    /// Annual credit spread (decimal, e.g., 0.012). Used if `annual_hazard` is None.
    #[cfg_attr(feature = "serde", serde(default))]
    pub annual_spread: Option<f64>,
    /// Recovery rate (e.g., 0.40).
    pub recovery_rate: f64,
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
        curves: &finstack_core::market_data::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        // Route to appropriate pricer based on spec type
        if self.is_deterministic() {
            crate::instruments::revolving_credit::pricer::RevolvingCreditDiscountingPricer::price_deterministic(
                self, curves, as_of,
            )
        } else {
            #[cfg(feature = "mc")]
            {
                crate::instruments::revolving_credit::pricer::RevolvingCreditMcPricer::price_stochastic(
                    self, curves, as_of,
                )
            }
            #[cfg(not(feature = "mc"))]
            {
                Err(finstack_core::error::InputError::Invalid.into())
            }
        }
    }

    fn price_with_metrics(
        &self,
        curves: &finstack_core::market_data::MarketContext,
        as_of: finstack_core::dates::Date,
        metrics: &[crate::metrics::MetricId],
    ) -> finstack_core::Result<crate::results::ValuationResult> {
        let base_value = self.value(curves, as_of)?;
        crate::instruments::common::helpers::build_with_metrics_dyn(
            self, curves, as_of, base_value, metrics,
        )
    }
}

// Implement HasDiscountCurve for generic metric calculators
impl crate::instruments::common::pricing::HasDiscountCurve for RevolvingCredit {
    fn discount_curve_id(&self) -> &finstack_core::types::CurveId {
        &self.discount_curve_id
    }
}

// Implement CashflowProvider for standard cashflow interface
impl crate::cashflow::traits::CashflowProvider for RevolvingCredit {
    fn build_schedule(
        &self,
        curves: &finstack_core::market_data::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<crate::cashflow::DatedFlows> {
        // Only works for deterministic specs
        if !self.is_deterministic() {
            return Err(finstack_core::error::InputError::Invalid.into());
        }

        let schedule = crate::instruments::revolving_credit::cashflows::generate_deterministic_cashflows_with_curves(
            self, curves, as_of,
        )?;

        Ok(schedule
            .flows
            .into_iter()
            .map(|cf| (cf.date, cf.amount))
            .collect())
    }

    fn build_full_schedule(
        &self,
        curves: &finstack_core::market_data::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<crate::cashflow::builder::CashFlowSchedule> {
        // Only works for deterministic specs
        if !self.is_deterministic() {
            return Err(finstack_core::error::InputError::Invalid.into());
        }

        crate::instruments::revolving_credit::cashflows::generate_deterministic_cashflows_with_curves(
            self, curves, as_of,
        )
    }
}
