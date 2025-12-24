//! Type definitions for structured credit instruments.
//!
//! This module contains all data structures for structured credit instruments:
//! - `StructuredCredit` - The main instrument type
//! - Pool and asset types
//! - Tranche structure and coupon types
//! - Waterfall distribution types
//! - Behavioral model specifications
//! - Result types for valuation

// ============================================================================
// TYPE DEFINITION MODULES
// ============================================================================

pub mod constants;
pub mod enums;
pub mod pool;
/// SoA layout for pool assets.
pub mod pool_state;
pub mod results;
pub mod setup;
pub mod tranches;
pub mod waterfall;

// ============================================================================
// INTERNAL MODULES
// ============================================================================

mod constructors;
mod reinvestment;
mod stochastic;

// ============================================================================
// RE-EXPORTS FROM TYPE MODULES
// ============================================================================

// Constants and defaults
pub use constants::*;

// Enums - use the new Seniority name
pub use enums::{AssetType, DealType, PaymentMode, TrancheSeniority, TriggerConsequence};
// Re-export TrancheSeniority as Seniority for cleaner naming
pub use enums::TrancheSeniority as Seniority;

// Pool types - use Pool as the primary name
pub use pool::AssetPool as Pool;
pub use pool::{
    calculate_pool_stats, AssetPool, ConcentrationCheckResult, ConcentrationViolation, PoolAsset,
    PoolStats, ReinvestmentCriteria, ReinvestmentPeriod,
};
pub use pool_state::PoolState;

// Tranche types
pub use tranches::{
    CoverageTrigger, CreditEnhancement, Tranche, TrancheBehaviorType, TrancheBuilder,
    TrancheCoupon, TrancheStructure,
};

// Setup/configuration types
pub use setup::{CoverageTestConfig, DealConfig, DealDates, DealFees, DefaultAssumptions};

// Reinvestment
pub use reinvestment::ReinvestmentManager;

// Waterfall types
pub use waterfall::CoverageTrigger as WaterfallCoverageTrigger;
pub use waterfall::{
    AllocationMode, CoverageTestType, ManagementFeeType, PaymentCalculation, PaymentRecord,
    PaymentType, Recipient, RecipientType, RoundingConvention, Waterfall, WaterfallBuilder,
    WaterfallDistribution, WaterfallTier, WaterfallWorkspace,
};

// Result types
pub use results::{TrancheCashflows, TrancheValuation, TrancheValuationExt};

// Stochastic specs - re-export from pricing
pub use crate::instruments::structured_credit::pricing::{
    CorrelationStructure, StochasticDefaultSpec, StochasticPrepaySpec,
};

// ============================================================================
// BEHAVIORAL MODEL SPECS
// ============================================================================

// Re-export deterministic model specs from cashflow builder
pub use crate::cashflow::builder::{DefaultModelSpec, PrepaymentModelSpec, RecoveryModelSpec};

// ============================================================================
// IMPORTS FOR STRUCTUREDCREDIT
// ============================================================================

use crate::cashflow::traits::{CashflowProvider, DatedFlows};
use crate::constants::DECIMAL_TO_PERCENT;
use crate::instruments::common::traits::{Attributes, Instrument};
use crate::instruments::irs::InterestRateSwap;
use crate::instruments::structured_credit::pricing::stochastic::pricer::{
    PricingMode, StochasticPricer, StochasticPricerConfig, StochasticPricingResult,
};
use crate::instruments::structured_credit::pricing::stochastic::tree::{
    BranchingSpec, ScenarioTreeConfig,
};
use crate::instruments::structured_credit::utils::rates::{cdr_to_mdr, cpr_to_smm};
use crate::metrics::MetricId;
use crate::results::ValuationResult;
use finstack_core::dates::{BusinessDayConvention, Date, DateExt, DayCount, DayCountCtx, Tenor};
use finstack_core::error::Error as CoreError;
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};

use finstack_core::collections::HashMap;
use std::any::Any;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

// ============================================================================
// MARKET CONDITIONS AND CREDIT FACTORS
// ============================================================================

/// Market conditions that affect prepayment behavior.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct MarketConditions {
    /// Current refinancing rate.
    pub refi_rate: f64,
    /// Rate at origination for refinancing incentive calculation.
    pub original_rate: Option<f64>,
    /// Home price appreciation (for mortgages).
    pub hpa: Option<f64>,
    /// Unemployment rate.
    pub unemployment: Option<f64>,
    /// Seasonal adjustment factor.
    pub seasonal_factor: Option<f64>,
    /// Custom market factors.
    pub custom_factors: HashMap<String, f64>,
}

impl Default for MarketConditions {
    fn default() -> Self {
        Self {
            refi_rate: 0.04,
            original_rate: None,
            hpa: None,
            unemployment: None,
            seasonal_factor: Some(1.0),
            custom_factors: HashMap::default(),
        }
    }
}

/// Credit factors affecting default probability.
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CreditFactors {
    /// Current FICO/credit score.
    pub credit_score: Option<u32>,
    /// Debt-to-income ratio.
    pub dti: Option<f64>,
    /// Loan-to-value ratio.
    pub ltv: Option<f64>,
    /// Payment delinquency status (days).
    pub delinquency_days: u32,
    /// Unemployment rate.
    pub unemployment_rate: Option<f64>,
    /// Additional custom factors.
    pub custom_factors: HashMap<String, f64>,
}

// ============================================================================
// DEAL METADATA AND OVERRIDES
// ============================================================================

/// Deal metadata (counterparties and identifiers).
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Metadata {
    /// Manager identifier (for CLO).
    pub manager_id: Option<String>,
    /// Servicer identifier (for ABS/RMBS/CMBS).
    pub servicer_id: Option<String>,
    /// Master servicer identifier (for CMBS/RMBS).
    pub master_servicer_id: Option<String>,
    /// Special servicer identifier (for CMBS).
    pub special_servicer_id: Option<String>,
    /// Trustee identifier (for ABS).
    pub trustee_id: Option<String>,
}

/// Behavioral overrides for prepayment, default, and recovery assumptions.
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Overrides {
    /// Override prepayment with constant annual CPR.
    pub cpr_annual: Option<f64>,
    /// Override prepayment with monthly ABS speed.
    pub abs_speed: Option<f64>,
    /// Override prepayment with PSA multiplier.
    pub psa_speed_multiplier: Option<f64>,
    /// Override default with constant annual CDR.
    pub cdr_annual: Option<f64>,
    /// Override default with SDA multiplier.
    pub sda_speed_multiplier: Option<f64>,
    /// Override recovery with constant rate.
    pub recovery_rate: Option<f64>,
    /// Override recovery lag (months).
    pub recovery_lag_months: Option<u32>,
    /// Reinvestment price constraint (% of par).
    pub reinvestment_price: Option<f64>,
}

// ============================================================================
// STRUCTURED CREDIT INSTRUMENT
// ============================================================================

/// Unified structured credit instrument representation.
///
/// This single type handles CLO, ABS, CMBS, and RMBS instruments using
/// composition for deal-specific differences.
#[derive(Clone, finstack_valuations_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct StructuredCredit {
    /// Unique instrument identifier.
    pub id: InstrumentId,

    /// Deal classification (ABS/CLO/CMBS/RMBS).
    pub deal_type: DealType,

    /// Asset pool definition.
    pub pool: Pool,

    /// Tranche structure.
    pub tranches: TrancheStructure,

    /// Key dates.
    /// Deal closing date (issuance).
    pub closing_date: Date,
    /// First payment date to tranches.
    pub first_payment_date: Date,
    /// End of reinvestment period (if applicable).
    pub reinvestment_end_date: Option<Date>,
    /// Legal final maturity date.
    pub legal_maturity: Date,

    /// Payment frequency for the structure.
    pub payment_frequency: Tenor,

    /// Optional payment calendar identifier for schedule adjustments.
    #[builder(default)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub payment_calendar_id: Option<String>,

    /// Business day convention for tranche payments (defaults to Following).
    #[builder(default)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub payment_bdc: Option<BusinessDayConvention>,

    /// Discount curve for valuation.
    pub discount_curve_id: CurveId,

    /// Attributes for scenario selection.
    pub attributes: Attributes,

    /// Prepayment model specification.
    #[cfg_attr(
        feature = "serde",
        serde(default = "StructuredCredit::default_prepayment_spec")
    )]
    pub prepayment_spec: PrepaymentModelSpec,

    /// Default model specification.
    #[cfg_attr(
        feature = "serde",
        serde(default = "StructuredCredit::default_default_spec")
    )]
    pub default_spec: DefaultModelSpec,

    /// Recovery model specification.
    #[cfg_attr(
        feature = "serde",
        serde(default = "StructuredCredit::default_recovery_spec")
    )]
    pub recovery_spec: RecoveryModelSpec,

    /// Market conditions impacting behavior.
    pub market_conditions: MarketConditions,

    /// Credit factors impacting default behavior.
    pub credit_factors: CreditFactors,

    /// Deal metadata (counterparties, identifiers).
    #[cfg_attr(feature = "serde", serde(default))]
    pub deal_metadata: Metadata,

    /// Behavioral assumption overrides.
    #[cfg_attr(feature = "serde", serde(default))]
    pub behavior_overrides: Overrides,

    /// Default behavioral assumptions for the deal.
    #[cfg_attr(feature = "serde", serde(default))]
    pub default_assumptions: DefaultAssumptions,

    /// Optional stochastic prepayment model specification.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub stochastic_prepay_spec: Option<StochasticPrepaySpec>,

    /// Optional stochastic default model specification.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub stochastic_default_spec: Option<StochasticDefaultSpec>,

    /// Optional correlation structure for stochastic modeling.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub correlation_structure: Option<CorrelationStructure>,

    /// Interest rate swaps used to hedge basis or interest rate risk.
    #[cfg_attr(feature = "serde", serde(default))]
    pub hedge_swaps: Vec<InterestRateSwap>,
}

impl StructuredCredit {
    /// Set the payment calendar ID for business day adjustments.
    ///
    /// This is required for accurate schedule generation. Structured credit deals
    /// are calendar-specific (e.g., NY, TARGET2), and using the wrong calendar
    /// shifts payment dates around holidays, breaking WAC/WAL and OC tests.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use finstack_core::dates::BusinessDayConvention;
    /// use finstack_valuations::instruments::structured_credit::StructuredCredit;
    ///
    /// let clo = StructuredCredit::example()
    ///     .with_payment_calendar("nyse")
    ///     .with_payment_bdc(BusinessDayConvention::ModifiedFollowing);
    /// # let _ = clo;
    /// ```
    #[must_use]
    pub fn with_payment_calendar(mut self, calendar_id: impl Into<String>) -> Self {
        self.payment_calendar_id = Some(calendar_id.into());
        self
    }

    /// Set the business day convention for payment date adjustments.
    ///
    /// If not specified, defaults to `BusinessDayConvention::Following`.
    #[must_use]
    pub fn with_payment_bdc(mut self, convention: BusinessDayConvention) -> Self {
        self.payment_bdc = Some(convention);
        self
    }

    /// Calculate current loss percentage of the pool.
    pub fn current_loss_percentage(&self) -> finstack_core::Result<f64> {
        let total_balance = self.pool.total_balance()?.amount();
        if total_balance == 0.0 {
            return Ok(0.0);
        }

        Ok(
            (self.pool.cumulative_defaults.amount() - self.pool.cumulative_recoveries.amount())
                / total_balance
                * DECIMAL_TO_PERCENT,
        )
    }

    /// Calculate expected life of the structure.
    pub fn expected_life(&self, as_of: Date) -> finstack_core::Result<f64> {
        Ok(self.pool.weighted_avg_maturity(as_of))
    }

    /// Create waterfall from instrument configuration.
    pub fn create_waterfall(&self) -> Waterfall {
        self.create_waterfall_internal()
    }

    /// Internal waterfall creation (called by constructors).
    fn create_waterfall_internal(&self) -> Waterfall {
        // Use the standard sequential waterfall as default
        Waterfall::standard_sequential(self.pool.base_currency(), &self.tranches, vec![])
    }

    /// Calculate prepayment rate (SMM) for a given period.
    pub fn calculate_prepayment_rate(&self, pay_date: Date, seasoning_months: u32) -> f64 {
        if let Some(override_rate) = self.prepayment_rate_override(pay_date, seasoning_months) {
            return override_rate;
        }
        self.prepayment_spec.smm(seasoning_months).max(0.0)
    }

    /// Calculate default rate (MDR) for a given period.
    pub fn calculate_default_rate(&self, pay_date: Date, seasoning_months: u32) -> f64 {
        if let Some(override_rate) = self.default_rate_override(pay_date, seasoning_months) {
            return override_rate;
        }
        self.default_spec.mdr(seasoning_months).max(0.0)
    }

    /// Stochastic pricing convenience that defaults to the tree-based engine.
    pub fn price_stochastic(
        &self,
        context: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<StochasticPricingResult> {
        self.price_stochastic_with_mode(context, as_of, PricingMode::Tree)
    }

    /// Stochastic pricing with an explicit mode (tree, Monte Carlo, or hybrid).
    pub fn price_stochastic_with_mode(
        &self,
        context: &MarketContext,
        as_of: Date,
        pricing_mode: PricingMode,
    ) -> finstack_core::Result<StochasticPricingResult> {
        let tree_config = self.build_scenario_tree_config(as_of)?;
        let discount_curve = context.get_discount(self.discount_curve_id.as_str())?;
        let config = StochasticPricerConfig::new(as_of, discount_curve, tree_config)
            .with_pricing_mode(pricing_mode);
        self.run_stochastic_pricer(config)
    }

    fn run_stochastic_pricer(
        &self,
        config: StochasticPricerConfig,
    ) -> finstack_core::Result<StochasticPricingResult> {
        let currency = self.pool.base_currency();
        let notional = self.pool.total_balance()?.amount();

        if notional.abs() <= f64::EPSILON {
            return Ok(StochasticPricingResult::new(
                Money::new(0.0, currency),
                Money::new(0.0, currency),
                0,
            ));
        }

        let pricer = StochasticPricer::new(config);
        let mut result = pricer
            .price(notional, currency)
            .map_err(CoreError::Validation)?;

        let tranche_specs: Vec<(String, String, f64, f64)> = self
            .tranches
            .tranches
            .iter()
            .map(|t| {
                (
                    t.id.to_string(),
                    format!("{:?}", t.seniority),
                    t.attachment_point / 100.0,
                    t.detachment_point / 100.0,
                )
            })
            .collect();

        if !tranche_specs.is_empty() {
            if let Ok(tranche_results) = pricer.price_tranches(&tranche_specs, notional, currency) {
                result = result.with_tranche_results(tranche_results);
            }
        }

        Ok(result)
    }

    fn build_scenario_tree_config(&self, as_of: Date) -> finstack_core::Result<ScenarioTreeConfig> {
        let months_to_maturity = as_of.months_until(self.legal_maturity).max(1) as usize;
        let horizon_years = DayCount::Act365F
            .year_fraction(as_of, self.legal_maturity, DayCountCtx::default())?
            .abs()
            .max(0.25);

        let mut tree_config =
            ScenarioTreeConfig::new(months_to_maturity, horizon_years, BranchingSpec::fixed(3));

        let (prepay, default, correlation) = self.effective_stochastic_specs();
        tree_config.prepay_spec = prepay;
        tree_config.default_spec = default;
        tree_config.correlation = correlation;
        tree_config.pool_coupon = self.pool.weighted_avg_coupon();
        tree_config.initial_balance = self.pool.total_balance()?.amount().max(1.0);
        let seasoning = if as_of > self.closing_date {
            self.closing_date.months_until(as_of)
        } else {
            0
        };
        tree_config.initial_seasoning = seasoning;
        tree_config = tree_config.with_seed(self.derive_seed(as_of));
        Ok(tree_config)
    }

    fn derive_seed(&self, as_of: Date) -> u64 {
        // Use a simple deterministic mixing of the ID and date bytes to ensure reproducibility
        // across different Rust versions/platforms (unlike DefaultHasher).
        let mut seed: u64 = 0xcbf29ce484222325; // FNV offset basis

        for byte in self.id.as_bytes() {
            seed ^= *byte as u64;
            seed = seed.wrapping_mul(0x100000001b3); // FNV prime
        }

        // Mix in date
        let date_val = as_of.to_julian_day() as u64;
        seed ^= date_val;
        seed = seed.wrapping_mul(0x100000001b3);

        seed
    }

    fn effective_stochastic_specs(
        &self,
    ) -> (
        StochasticPrepaySpec,
        StochasticDefaultSpec,
        CorrelationStructure,
    ) {
        let prepay = self
            .stochastic_prepay_spec
            .clone()
            .unwrap_or_else(|| StochasticPrepaySpec::deterministic(self.prepayment_spec.clone()));

        let default = self
            .stochastic_default_spec
            .clone()
            .unwrap_or_else(|| StochasticDefaultSpec::deterministic(self.default_spec.clone()));

        let correlation =
            self.correlation_structure
                .clone()
                .unwrap_or_else(|| match self.deal_type {
                    DealType::RMBS => CorrelationStructure::rmbs_standard(),
                    DealType::CLO | DealType::CBO => CorrelationStructure::clo_standard(),
                    DealType::CMBS => CorrelationStructure::cmbs_standard(),
                    _ => CorrelationStructure::abs_auto_standard(),
                });

        (prepay, default, correlation)
    }

    /// Calculate Option-Adjusted Spread (OAS) given a market price.
    ///
    /// This solves for the spread over the discount curve that equates the
    /// present value of cashflows to the market price.
    ///
    /// Note: This currently uses deterministic cashflows (Z-Spread equivalent).
    /// For true OAS with stochastic prepayment, use `StochasticPricer`.
    pub fn calculate_oas(
        &self,
        context: &MarketContext,
        as_of: Date,
        market_price: f64,
    ) -> finstack_core::Result<f64> {
        use finstack_core::math::solver::{BrentSolver, Solver};

        let flows = self.build_schedule(context, as_of)?;
        let discount_curve = context.get_discount(&self.discount_curve_id)?;

        let price_fn = |spread: f64| -> f64 {
            let mut pv = finstack_core::math::summation::NeumaierAccumulator::new();
            for (date, amount) in &flows {
                // Calculate discount factor with spread
                // DF = exp(-(r + s) * t)
                // We assume continuous compounding for the spread application

                let t = match DayCount::Act365F.year_fraction(as_of, *date, DayCountCtx::default())
                {
                    Ok(t) => t,
                    Err(_) => return f64::NAN, // Solver handles NAN/Inf usually by erroring, but Brent might need finite
                };

                if t <= 0.0 {
                    // Flow is today or past, assume full value or ignore?
                    // Usually ignore past flows, but build_schedule might return future only.
                    // If today, DF=1.
                    pv.add(amount.amount());
                    continue;
                }

                let df_base = match discount_curve.try_df_on_date_curve(*date) {
                    Ok(df) => df,
                    Err(_) => return f64::NAN,
                };

                // Adjustment: df_spread = exp(-spread * t)
                let df_spread = (-spread * t).exp();
                let df = df_base * df_spread;

                pv.add(amount.amount() * df);
            }
            pv.total() - market_price
        };

        // Solve for spread
        // Initial guess: 100 bps (0.01)
        // Bracket: -10% to +50%?
        // BrentSolver finds bracket automatically if not provided.
        let solver = BrentSolver::new().with_tolerance(1e-6);
        solver.solve(price_fn, 0.01)
    }

    fn prepayment_rate_override(&self, _pay_date: Date, seasoning: u32) -> Option<f64> {
        if let Some(abs_speed) = self.behavior_overrides.abs_speed {
            return Some(abs_speed);
        }

        if let Some(cpr) = self.behavior_overrides.cpr_annual {
            return Some(cpr_to_smm(cpr));
        }

        if let Some(psa_mult) = self.behavior_overrides.psa_speed_multiplier {
            let base_cpr = if seasoning <= constants::PSA_RAMP_MONTHS {
                (seasoning as f64 / constants::PSA_RAMP_MONTHS as f64) * constants::PSA_TERMINAL_CPR
            } else {
                constants::PSA_TERMINAL_CPR
            };
            let cpr = base_cpr * psa_mult;
            return Some(cpr_to_smm(cpr));
        }

        None
    }

    fn default_rate_override(&self, _pay_date: Date, seasoning: u32) -> Option<f64> {
        if let Some(cdr) = self.behavior_overrides.cdr_annual {
            return Some(cdr_to_mdr(cdr));
        }

        if let Some(sda_mult) = self.behavior_overrides.sda_speed_multiplier {
            let decline_period = (constants::SDA_PEAK_MONTH * 2 - constants::SDA_PEAK_MONTH) as f64;

            let cdr = if seasoning <= constants::SDA_PEAK_MONTH {
                (seasoning as f64 / constants::SDA_PEAK_MONTH as f64) * constants::SDA_PEAK_CDR
            } else if seasoning <= constants::SDA_PEAK_MONTH * 2 {
                let months_past_peak = (seasoning - constants::SDA_PEAK_MONTH) as f64;
                constants::SDA_PEAK_CDR
                    - (months_past_peak / decline_period)
                        * (constants::SDA_PEAK_CDR - constants::SDA_TERMINAL_CDR)
            } else {
                constants::SDA_TERMINAL_CDR
            } * sda_mult;

            return Some(1.0 - (1.0 - cdr).powf(1.0 / 12.0));
        }

        None
    }

    #[cfg(feature = "serde")]
    fn default_prepayment_spec() -> PrepaymentModelSpec {
        PrepaymentModelSpec::constant_cpr(0.10)
    }

    #[cfg(feature = "serde")]
    fn default_default_spec() -> DefaultModelSpec {
        DefaultModelSpec::constant_cdr(0.02)
    }

    #[cfg(feature = "serde")]
    fn default_recovery_spec() -> RecoveryModelSpec {
        RecoveryModelSpec::with_lag(0.40, 12)
    }
}

// ============================================================================
// TRAIT IMPLEMENTATIONS
// ============================================================================

impl CashflowProvider for StructuredCredit {
    fn notional(&self) -> Option<Money> {
        // Return total pool balance as the notional
        self.pool.total_balance().ok()
    }

    fn build_schedule(
        &self,
        context: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<DatedFlows> {
        crate::instruments::structured_credit::pricing::generate_cashflows(self, context, as_of)
    }
}

impl Instrument for StructuredCredit {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::StructuredCredit
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn attributes(&self) -> &Attributes {
        &self.attributes
    }

    fn attributes_mut(&mut self) -> &mut Attributes {
        &mut self.attributes
    }

    fn clone_box(&self) -> Box<dyn Instrument> {
        Box::new(self.clone())
    }

    fn value(&self, context: &MarketContext, as_of: Date) -> finstack_core::Result<Money> {
        let disc = context.get_discount_ref(self.discount_curve_id.as_str())?;
        let flows = self.build_schedule(context, as_of)?;

        use crate::instruments::common::discountable::Discountable;
        let curve_day_count = disc.day_count();
        flows.npv(disc, as_of, curve_day_count)
    }

    fn price_with_metrics(
        &self,
        context: &MarketContext,
        as_of: Date,
        metrics: &[MetricId],
    ) -> finstack_core::Result<ValuationResult> {
        let base_value = self.value(context, as_of)?;

        if metrics.is_empty() {
            return Ok(ValuationResult::stamped(
                self.id.as_str(),
                as_of,
                base_value,
            ));
        }

        let flows = self.build_schedule(context, as_of)?;
        let mut metric_context = crate::metrics::MetricContext::new(
            std::sync::Arc::new(self.clone())
                as std::sync::Arc<dyn crate::instruments::common::traits::Instrument>,
            std::sync::Arc::new(context.clone()),
            as_of,
            base_value,
        );
        metric_context.cashflows = Some(flows);
        metric_context.discount_curve_id = Some(self.discount_curve_id.to_owned());

        let registry = crate::metrics::standard_registry();
        let computed_metrics = registry.compute(metrics, &mut metric_context)?;

        let mut result = ValuationResult::stamped(self.id.as_str(), as_of, base_value);
        for (metric_id, value) in computed_metrics {
            result.measures.insert(metric_id.to_string(), value);
        }

        Ok(result)
    }

    fn as_cashflow_provider(&self) -> Option<&dyn CashflowProvider> {
        Some(self)
    }
}

impl crate::instruments::common::pricing::HasDiscountCurve for StructuredCredit {
    fn discount_curve_id(&self) -> &CurveId {
        &self.discount_curve_id
    }
}

impl crate::instruments::common::traits::CurveDependencies for StructuredCredit {
    fn curve_dependencies(&self) -> crate::instruments::common::traits::InstrumentCurves {
        crate::instruments::common::traits::InstrumentCurves::builder()
            .discount(self.discount_curve_id.clone())
            .build()
    }
}

impl TrancheValuationExt for StructuredCredit {
    fn get_tranche_cashflows(
        &self,
        tranche_id: &str,
        context: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<TrancheCashflows> {
        crate::instruments::structured_credit::pricing::generate_tranche_cashflows(
            self, tranche_id, context, as_of,
        )
    }

    fn value_tranche(
        &self,
        tranche_id: &str,
        context: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<Money> {
        let cashflows = self.get_tranche_cashflows(tranche_id, context, as_of)?;
        let disc = context.get_discount(&self.discount_curve_id)?;

        let mut pv = Money::new(0.0, self.pool.base_currency());
        for (date, amount) in &cashflows.cashflows {
            if *date > as_of {
                let df = disc.try_df_between_dates(as_of, *date)?;
                let flow_pv = Money::new(amount.amount() * df, amount.currency());
                pv = pv.checked_add(flow_pv)?;
            }
        }

        Ok(pv)
    }

    fn value_tranche_with_metrics(
        &self,
        tranche_id: &str,
        context: &MarketContext,
        as_of: Date,
        metrics: &[MetricId],
    ) -> finstack_core::Result<TrancheValuation> {
        use crate::instruments::structured_credit::metrics::{
            calculate_tranche_cs01, calculate_tranche_duration, calculate_tranche_wal,
            calculate_tranche_z_spread,
        };

        let cashflow_result = self.get_tranche_cashflows(tranche_id, context, as_of)?;
        let pv = self.value_tranche(tranche_id, context, as_of)?;

        let mut metric_context = crate::metrics::MetricContext::new(
            std::sync::Arc::new(self.clone())
                as std::sync::Arc<dyn crate::instruments::common::traits::Instrument>,
            std::sync::Arc::new(context.clone()),
            as_of,
            pv,
        );
        metric_context.cashflows = Some(cashflow_result.cashflows.clone());
        metric_context.discount_curve_id = Some(self.discount_curve_id.to_owned());

        let registry = crate::metrics::standard_registry();
        let computed_metrics = registry.compute(metrics, &mut metric_context)?;

        let tranche = self
            .tranches
            .tranches
            .iter()
            .find(|t| t.id.as_str() == tranche_id)
            .ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                    id: format!("tranche:{}", tranche_id),
                })
            })?;

        let notional = tranche.original_balance.amount();

        let dirty_price = if notional > 0.0 {
            (pv.amount() / notional) * 100.0
        } else {
            0.0
        };

        let accrued_value = computed_metrics
            .get(&MetricId::Accrued)
            .copied()
            .unwrap_or(0.0);
        let accrued = Money::new(accrued_value, pv.currency());

        let clean_price = if notional > 0.0 {
            dirty_price - (accrued.amount() / notional) * 100.0
        } else {
            dirty_price
        };

        let wal = match computed_metrics.get(&MetricId::WAL) {
            Some(v) => *v,
            None => calculate_tranche_wal(&cashflow_result, as_of)?,
        };

        let disc = context.get_discount(&self.discount_curve_id)?;
        let modified_duration = computed_metrics
            .get(&MetricId::DurationMod)
            .copied()
            .unwrap_or_else(|| {
                calculate_tranche_duration(&cashflow_result.cashflows, &disc, as_of, pv)
                    .unwrap_or(0.0)
            });

        let z_spread = computed_metrics
            .get(&MetricId::ZSpread)
            .copied()
            .unwrap_or_else(|| {
                calculate_tranche_z_spread(&cashflow_result.cashflows, &disc, pv, as_of)
                    .unwrap_or(0.0)
            });

        let z_spread_decimal = z_spread / 10_000.0;
        let cs01 = computed_metrics
            .get(&MetricId::Cs01)
            .copied()
            .unwrap_or_else(|| {
                calculate_tranche_cs01(&cashflow_result.cashflows, &disc, z_spread_decimal, as_of)
                    .unwrap_or(0.0)
            });

        let ytm = computed_metrics
            .get(&MetricId::Ytm)
            .copied()
            .unwrap_or(0.05);

        let final_metrics: finstack_core::collections::HashMap<MetricId, f64> =
            computed_metrics.into_iter().collect();

        Ok(TrancheValuation {
            tranche_id: tranche_id.to_string(),
            pv,
            clean_price,
            dirty_price,
            accrued,
            wal,
            modified_duration,
            z_spread_bps: z_spread,
            cs01,
            ytm,
            metrics: final_metrics,
        })
    }
}

impl core::fmt::Debug for StructuredCredit {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("StructuredCredit")
            .field("id", &self.id)
            .field("deal_type", &self.deal_type)
            .field("closing_date", &self.closing_date)
            .field("first_payment_date", &self.first_payment_date)
            .field("legal_maturity", &self.legal_maturity)
            .field("payment_frequency", &self.payment_frequency)
            .field("discount_curve_id", &self.discount_curve_id)
            .finish()
    }
}

impl core::fmt::Display for StructuredCredit {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let pool_balance = self
            .pool
            .total_balance()
            .unwrap_or(Money::new(0.0, self.pool.base_currency()));
        let tranche_count = self.tranches.tranches.len();

        write!(
            f,
            "{} {:?} | Pool: {} {} | {} tranches | {} -> {}",
            self.id.as_str(),
            self.deal_type,
            pool_balance.amount(),
            pool_balance.currency(),
            tranche_count,
            self.closing_date,
            self.legal_maturity,
        )
    }
}
