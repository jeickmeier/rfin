//! Unified structured credit instrument (ABS, CLO, CMBS, RMBS).
//!
//! This module consolidates four nearly-identical instrument types into a single
//! clean implementation using composition for deal-specific differences.

mod constructors;
mod reinvestment;
mod stochastic;

pub use reinvestment::ReinvestmentManager;

use crate::cashflow::traits::{CashflowProvider, DatedFlows};
use crate::constants::DECIMAL_TO_PERCENT;
use crate::instruments::common::traits::{Attributes, Instrument};
use crate::instruments::irs::InterestRateSwap;
use crate::instruments::structured_credit::components::{
    cdr_to_mdr, cpr_to_smm, AssetPool, CorrelationStructure, CreditFactors, DealType,
    DefaultModelSpec, MarketConditions, PrepaymentModelSpec, RecoveryModelSpec,
    StochasticDefaultSpec, StochasticPrepaySpec, TrancheCashflowResult, TrancheStructure,
    TrancheValuation, TrancheValuationExt, WaterfallEngine,
};
use crate::metrics::MetricId;
use crate::results::ValuationResult;
use finstack_core::dates::{Date, Frequency};
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};

use crate::instruments::structured_credit::config::{
    DefaultAssumptions, PSA_RAMP_MONTHS, PSA_TERMINAL_CPR, SDA_PEAK_CDR, SDA_PEAK_MONTH,
    SDA_TERMINAL_CDR,
};
use std::any::Any;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Deal metadata (counterparties and identifiers).
///
/// This struct captures operational metadata about the deal's service providers
/// and external parties, separate from behavioral/pricing assumptions.
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct DealMetadata {
    /// Manager identifier (for CLO)
    pub manager_id: Option<String>,
    /// Servicer identifier (for ABS/RMBS/CMBS)
    pub servicer_id: Option<String>,
    /// Master servicer identifier (for CMBS/RMBS)
    pub master_servicer_id: Option<String>,
    /// Special servicer identifier (for CMBS)
    pub special_servicer_id: Option<String>,
    /// Trustee identifier (for ABS)
    pub trustee_id: Option<String>,
}

/// Behavioral overrides for prepayment, default, and recovery assumptions.
///
/// These fields allow instrument-level overrides of the model specifications.
/// If set, they take precedence over the model specs for specific parameters.
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct BehaviorOverrides {
    // Prepayment overrides
    /// Override prepayment with constant annual CPR
    pub cpr_annual: Option<f64>,
    /// Override prepayment with monthly ABS speed
    pub abs_speed: Option<f64>,
    /// Override prepayment with PSA multiplier
    pub psa_speed_multiplier: Option<f64>,

    // Default overrides
    /// Override default with constant annual CDR
    pub cdr_annual: Option<f64>,
    /// Override default with SDA multiplier
    pub sda_speed_multiplier: Option<f64>,

    // Recovery overrides
    /// Override recovery with constant rate
    pub recovery_rate: Option<f64>,
    /// Override recovery lag (months)
    pub recovery_lag_months: Option<u32>,

    // Trading overrides
    /// Reinvestment price constraint (% of par)
    pub reinvestment_price: Option<f64>,
}

/// Unified structured credit instrument representation.
///
/// This single type replaces the previous separate `Abs`, `Clo`, `Cmbs`, and `Rmbs`
/// types, consolidating ~1,400 lines of near-duplicate code into a clean, composable design.
#[derive(Clone, finstack_valuations_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct StructuredCredit {
    /// Unique instrument identifier
    pub id: InstrumentId,

    /// Deal classification (ABS/CLO/CMBS/RMBS)
    pub deal_type: DealType,

    /// Asset pool definition
    pub pool: AssetPool,

    /// Tranche structure
    pub tranches: TrancheStructure,

    /// Waterfall distribution rules
    pub waterfall: WaterfallEngine,

    /// Key dates
    /// Deal closing date (issuance)
    pub closing_date: Date,
    /// First payment date to tranches
    pub first_payment_date: Date,
    /// End of reinvestment period (if applicable)
    pub reinvestment_end_date: Option<Date>,
    /// Legal final maturity date
    pub legal_maturity: Date,

    /// Payment frequency for the structure
    pub payment_frequency: Frequency,

    /// Discount curve for valuation
    pub discount_curve_id: CurveId,

    /// Attributes for scenario selection
    pub attributes: Attributes,

    /// Prepayment model specification
    #[cfg_attr(
        feature = "serde",
        serde(default = "StructuredCredit::default_prepayment_spec")
    )]
    pub prepayment_spec: PrepaymentModelSpec,

    /// Default model specification
    #[cfg_attr(
        feature = "serde",
        serde(default = "StructuredCredit::default_default_spec")
    )]
    pub default_spec: DefaultModelSpec,

    /// Recovery model specification
    #[cfg_attr(
        feature = "serde",
        serde(default = "StructuredCredit::default_recovery_spec")
    )]
    pub recovery_spec: RecoveryModelSpec,

    /// Market conditions impacting behavior
    pub market_conditions: MarketConditions,

    /// Credit factors impacting default behavior
    pub credit_factors: CreditFactors,

    /// Deal metadata (counterparties, identifiers)
    #[cfg_attr(feature = "serde", serde(default))]
    pub deal_metadata: DealMetadata,

    /// Behavioral assumption overrides
    #[cfg_attr(feature = "serde", serde(default))]
    pub behavior_overrides: BehaviorOverrides,

    /// Default behavioral assumptions for the deal.
    #[cfg_attr(feature = "serde", serde(default))]
    pub default_assumptions: DefaultAssumptions,

    // =========================================================================
    // Stochastic modeling (optional)
    // =========================================================================
    /// Optional stochastic prepayment model specification.
    ///
    /// When set, enables stochastic (factor-correlated) prepayment modeling
    /// with scenario trees or Monte Carlo simulation.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub stochastic_prepay_spec: Option<StochasticPrepaySpec>,

    /// Optional stochastic default model specification.
    ///
    /// When set, enables copula-based or intensity-process default modeling
    /// with correlation-driven scenario analysis.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub stochastic_default_spec: Option<StochasticDefaultSpec>,

    /// Optional correlation structure for stochastic modeling.
    ///
    /// Defines asset correlation, prepay-default correlation, and
    /// sector structure for multi-factor models.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub correlation_structure: Option<CorrelationStructure>,

    // =========================================================================
    // Hedge instruments
    // =========================================================================
    /// Interest rate swaps used to hedge basis or interest rate risk.
    ///
    /// These swaps are valued alongside the deal to provide hedged NPV.
    /// Common uses:
    /// - **Basis swaps**: Hedge mismatch between asset index (e.g., Prime) and liability index (e.g., SOFR)
    /// - **Rate swaps**: Fixed-for-floating to manage duration
    /// - **Cap protection**: Embedded in floating-rate tranche structures
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

    #[cfg(feature = "serde")]
    fn default_prepayment_spec() -> PrepaymentModelSpec {
        PrepaymentModelSpec::constant_cpr(0.10) // Generic 10% CPR
    }

    #[cfg(feature = "serde")]
    fn default_default_spec() -> DefaultModelSpec {
        DefaultModelSpec::constant_cdr(0.02) // Generic 2% CDR
    }

    #[cfg(feature = "serde")]
    fn default_recovery_spec() -> RecoveryModelSpec {
        RecoveryModelSpec::with_lag(0.40, 12) // Generic 40% recovery, 12 month lag
    }
}

impl CashflowProvider for StructuredCredit {
    fn build_schedule(
        &self,
        context: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<DatedFlows> {
        use crate::instruments::structured_credit::instrument_trait::StructuredCreditInstrument;
        <Self as StructuredCreditInstrument>::generate_tranche_cashflows(self, context, as_of)
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

// Implement CurveDependencies for DV01 calculator
impl crate::instruments::common::traits::CurveDependencies for StructuredCredit {
    fn curve_dependencies(&self) -> crate::instruments::common::traits::InstrumentCurves {
        crate::instruments::common::traits::InstrumentCurves::builder()
            .discount(self.discount_curve_id.clone())
            .build()
    }
}

impl crate::instruments::structured_credit::instrument_trait::StructuredCreditInstrument
    for StructuredCredit
{
    fn pool(&self) -> &AssetPool {
        &self.pool
    }

    fn tranches(&self) -> &TrancheStructure {
        &self.tranches
    }

    fn closing_date(&self) -> Date {
        self.closing_date
    }

    fn first_payment_date(&self) -> Date {
        self.first_payment_date
    }

    fn legal_maturity(&self) -> Date {
        self.legal_maturity
    }

    fn payment_frequency(&self) -> Frequency {
        self.payment_frequency
    }

    fn prepayment_spec(&self) -> &PrepaymentModelSpec {
        &self.prepayment_spec
    }

    fn default_spec_ref(&self) -> &DefaultModelSpec {
        &self.default_spec
    }

    fn recovery_spec_ref(&self) -> &RecoveryModelSpec {
        &self.recovery_spec
    }

    fn default_assumptions(&self) -> &DefaultAssumptions {
        &self.default_assumptions
    }

    fn market_conditions(&self) -> &MarketConditions {
        &self.market_conditions
    }

    fn credit_factors(&self) -> &CreditFactors {
        &self.credit_factors
    }

    fn create_waterfall_engine(&self) -> WaterfallEngine {
        self.create_waterfall_engine_internal()
    }

    fn prepayment_rate_override(&self, _pay_date: Date, seasoning: u32) -> Option<f64> {
        // Check overrides in priority order
        if let Some(abs_speed) = self.behavior_overrides.abs_speed {
            return Some(abs_speed);
        }

        if let Some(cpr) = self.behavior_overrides.cpr_annual {
            return Some(cpr_to_smm(cpr));
        }

        if let Some(psa_mult) = self.behavior_overrides.psa_speed_multiplier {
            // PSA calculation using standard constants
            let base_cpr = if seasoning <= PSA_RAMP_MONTHS {
                (seasoning as f64 / PSA_RAMP_MONTHS as f64) * PSA_TERMINAL_CPR
            } else {
                PSA_TERMINAL_CPR
            };
            let cpr = base_cpr * psa_mult;
            return Some(cpr_to_smm(cpr));
        }

        None
    }

    fn default_rate_override(&self, _pay_date: Date, seasoning: u32) -> Option<f64> {
        // Check overrides in priority order
        if let Some(cdr) = self.behavior_overrides.cdr_annual {
            return Some(cdr_to_mdr(cdr));
        }

        if let Some(sda_mult) = self.behavior_overrides.sda_speed_multiplier {
            // SDA calculation using standard constants
            let decline_period = (SDA_PEAK_MONTH * 2 - SDA_PEAK_MONTH) as f64; // 30 months decline

            let cdr = if seasoning <= SDA_PEAK_MONTH {
                // Ramp up to peak
                (seasoning as f64 / SDA_PEAK_MONTH as f64) * SDA_PEAK_CDR
            } else if seasoning <= SDA_PEAK_MONTH * 2 {
                // Decline from peak to terminal
                let months_past_peak = (seasoning - SDA_PEAK_MONTH) as f64;
                SDA_PEAK_CDR
                    - (months_past_peak / decline_period) * (SDA_PEAK_CDR - SDA_TERMINAL_CDR)
            } else {
                // Terminal rate
                SDA_TERMINAL_CDR
            } * sda_mult;

            // Convert CDR to MDR
            return Some(1.0 - (1.0 - cdr).powf(1.0 / 12.0));
        }

        None
    }
}

impl TrancheValuationExt for StructuredCredit {
    fn get_tranche_cashflows(
        &self,
        tranche_id: &str,
        context: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<TrancheCashflowResult> {
        use crate::instruments::structured_credit::instrument_trait::StructuredCreditInstrument;
        <Self as StructuredCreditInstrument>::generate_specific_tranche_cashflows(
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

        // Pre-compute as_of discount factor for correct theta
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
                // Discount from as_of for correct theta
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

        // Most metrics are calculated via the generic metrics registry.
        // We create a context and pass the detailed cashflow result to it.
        let mut metric_context = crate::metrics::MetricContext::new(
            std::sync::Arc::new(self.clone())
                as std::sync::Arc<dyn crate::instruments::common::traits::Instrument>,
            std::sync::Arc::new(context.clone()),
            as_of,
            pv,
        );
        metric_context.cashflows = Some(cashflow_result.cashflows.clone());
        metric_context.detailed_tranche_cashflows = Some(cashflow_result.clone());
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

        // Ensure WAL is calculated if requested, as it's a primary output field
        let wal = match computed_metrics.get(&MetricId::WAL) {
            Some(v) => *v,
            None => calculate_tranche_wal(&cashflow_result, as_of)?,
        };

        // Fallback calculations for metrics not handled by the registry or if not requested
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
            .unwrap_or(0.05); // Default guess

        // Convert computed metrics to std::collections::HashMap for the TrancheValuation struct
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

#[cfg(all(test, feature = "serde"))]
mod serde_tests {
    use super::*;
    use crate::instruments::structured_credit::components::{
        Tranche, TrancheCoupon, TrancheSeniority, TrancheStructure,
    };
    use finstack_core::currency::Currency;
    use finstack_core::dates::Date;
    use finstack_core::money::Money;
    use time::Month;

    #[test]
    fn test_structured_credit_json_roundtrip() {
        let pool = AssetPool::new("TEST_POOL", DealType::CLO, Currency::USD);

        let tranche = Tranche::new(
            "EQUITY",
            0.0,
            100.0,
            TrancheSeniority::Equity,
            Money::new(1_000_000.0, Currency::USD),
            TrancheCoupon::Fixed { rate: 0.12 },
            Date::from_calendar_date(2030, Month::January, 1).expect("Valid example date"),
        )
        .expect("Tranche build should succeed in test");

        let tranches =
            TrancheStructure::new(vec![tranche]).expect("TrancheStructure should build in test");
        let waterfall = WaterfallEngine::new(Currency::USD);

        let original = StructuredCredit::new_clo(
            "TEST_CLO",
            pool,
            tranches,
            waterfall,
            Date::from_calendar_date(2024, Month::January, 1).expect("Valid example date"),
            Date::from_calendar_date(2030, Month::January, 1).expect("Valid example date"),
            "USD-OIS",
        );

        // Serialize to JSON
        let json = serde_json::to_string(&original).expect("Failed to serialize");

        // Deserialize from JSON
        let deserialized: StructuredCredit =
            serde_json::from_str(&json).expect("Failed to deserialize");

        // Verify key fields match
        assert_eq!(original.id.as_str(), deserialized.id.as_str());
        assert_eq!(original.deal_type, deserialized.deal_type);
        assert_eq!(original.prepayment_spec, deserialized.prepayment_spec);
        assert_eq!(original.default_spec, deserialized.default_spec);
        assert_eq!(original.recovery_spec, deserialized.recovery_spec);
    }

    #[test]
    fn test_behavior_overrides_serialization() {
        let pool = AssetPool::new("TEST_POOL", DealType::RMBS, Currency::USD);

        let tranche = Tranche::new(
            "AAA",
            0.0,
            100.0,
            TrancheSeniority::Senior,
            Money::new(10_000_000.0, Currency::USD),
            TrancheCoupon::Fixed { rate: 0.05 },
            Date::from_calendar_date(2035, Month::January, 1).expect("Valid example date"),
        )
        .expect("Tranche build should succeed in test");

        let tranches =
            TrancheStructure::new(vec![tranche]).expect("TrancheStructure should build in test");
        let waterfall = WaterfallEngine::new(Currency::USD);

        let mut rmbs = StructuredCredit::new_rmbs(
            "TEST_RMBS",
            pool,
            tranches,
            waterfall,
            Date::from_calendar_date(2024, Month::January, 1).expect("Valid example date"),
            Date::from_calendar_date(2035, Month::January, 1).expect("Valid example date"),
            "USD-OIS",
        );

        // Set behavior overrides
        rmbs.behavior_overrides.psa_speed_multiplier = Some(1.5);
        rmbs.behavior_overrides.cdr_annual = Some(0.01);

        // Serialize
        let json = serde_json::to_string(&rmbs).expect("Failed to serialize");

        // Deserialize
        let deserialized: StructuredCredit =
            serde_json::from_str(&json).expect("Failed to deserialize");

        // Verify overrides are preserved
        assert_eq!(
            deserialized.behavior_overrides.psa_speed_multiplier,
            Some(1.5)
        );
        assert_eq!(deserialized.behavior_overrides.cdr_annual, Some(0.01));
    }

    #[test]
    fn test_deal_metadata_serialization() {
        let pool = AssetPool::new("TEST_POOL", DealType::CLO, Currency::USD);

        let tranche = Tranche::new(
            "AAA",
            0.0,
            100.0,
            TrancheSeniority::Senior,
            Money::new(10_000_000.0, Currency::USD),
            TrancheCoupon::Fixed { rate: 0.05 },
            Date::from_calendar_date(2030, Month::January, 1).expect("Valid example date"),
        )
        .expect("Tranche build should succeed in test");

        let tranches =
            TrancheStructure::new(vec![tranche]).expect("TrancheStructure should build in test");
        let waterfall = WaterfallEngine::new(Currency::USD);

        let mut clo = StructuredCredit::new_clo(
            "TEST_CLO",
            pool,
            tranches,
            waterfall,
            Date::from_calendar_date(2024, Month::January, 1).expect("Valid example date"),
            Date::from_calendar_date(2030, Month::January, 1).expect("Valid example date"),
            "USD-OIS",
        );

        // Set deal metadata
        clo.deal_metadata.manager_id = Some("Apollo".to_string());
        clo.deal_metadata.servicer_id = Some("BNY Mellon".to_string());

        // Serialize
        let json = serde_json::to_string(&clo).expect("Failed to serialize");

        // Deserialize
        let deserialized: StructuredCredit =
            serde_json::from_str(&json).expect("Failed to deserialize");

        // Verify metadata is preserved
        assert_eq!(
            deserialized.deal_metadata.manager_id,
            Some("Apollo".to_string())
        );
        assert_eq!(
            deserialized.deal_metadata.servicer_id,
            Some("BNY Mellon".to_string())
        );
    }
}
