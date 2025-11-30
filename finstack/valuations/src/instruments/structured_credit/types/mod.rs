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
    PaymentType, Recipient, RecipientType, Waterfall, WaterfallBuilder, WaterfallDistribution,
    WaterfallTier, WaterfallWorkspace,
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
use crate::instruments::structured_credit::utils::rates::{cdr_to_mdr, cpr_to_smm};
use crate::metrics::MetricId;
use crate::results::ValuationResult;
use finstack_core::dates::{Date, Frequency};
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};

use std::any::Any;
use std::collections::HashMap;

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
            custom_factors: HashMap::new(),
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

/// Legacy type alias for `Metadata` (backward compatibility).
pub type DealMetadata = Metadata;
/// Legacy type alias for `Overrides` (backward compatibility).
pub type BehaviorOverrides = Overrides;

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
    pub payment_frequency: Frequency,

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
    /// Calculate current loss percentage of the pool.
    pub fn current_loss_percentage(&self) -> f64 {
        let total_balance = self.pool.total_balance().amount();
        if total_balance == 0.0 {
            return 0.0;
        }

        (self.pool.cumulative_defaults.amount() - self.pool.cumulative_recoveries.amount())
            / total_balance
            * DECIMAL_TO_PERCENT
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
        flows.npv(disc, as_of, finstack_core::dates::DayCount::Act360)
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

        let disc_dc = disc.day_count();
        let t_as_of = disc_dc
            .year_fraction(
                disc.base_date(),
                as_of,
                finstack_core::dates::DayCountCtx::default(),
            )
            .unwrap_or(0.0);
        let df_as_of = disc.df(t_as_of);

        let mut pv = Money::new(0.0, self.pool.base_currency());
        for (date, amount) in &cashflows.cashflows {
            if *date > as_of {
                let t_cf = disc_dc
                    .year_fraction(
                        disc.base_date(),
                        *date,
                        finstack_core::dates::DayCountCtx::default(),
                    )
                    .unwrap_or(0.0);
                let df_cf_abs = disc.df(t_cf);
                let df = if df_as_of != 0.0 {
                    df_cf_abs / df_as_of
                } else {
                    1.0
                };
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

        let final_metrics: std::collections::HashMap<MetricId, f64> =
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
        let pool_balance = self.pool.total_balance();
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
