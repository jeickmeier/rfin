//! Collateralized Loan Obligation (CLO) instrument built on the shared structured credit core.

use crate::cashflow::traits::{CashflowProvider, DatedFlows};
use crate::instruments::common::discountable::Discountable;
use crate::instruments::common::structured_credit::{
    AssetPool,
    CoverageTests,
    CreditFactors,
    DealType,
    DefaultBehavior,
    DefaultModelSpec,
    MarketConditions,
    // Prepayment/default frameworks
    PrepaymentBehavior,
    PrepaymentModelSpec,
    RecoveryBehavior,
    RecoveryModelSpec,
    StructuredCreditWaterfall,
    TrancheStructure,
    // Tranche valuation
    TrancheCashflowResult,
    TrancheValuation,
    TrancheValuationExt,
    TrancheSeniority,
};
use crate::instruments::common::traits::{Attributes, Instrument};
use crate::metrics::MetricId;
use crate::results::ValuationResult;
use finstack_core::dates::{Date, Frequency};
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};

use std::any::Any;
use std::sync::Arc;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Primary CLO instrument representation.
#[derive(Clone, finstack_valuations_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Clo {
    /// Unique instrument identifier
    pub id: InstrumentId,

    /// Deal classification (always `DealType::CLO`)
    pub deal_type: DealType,

    /// Asset pool definition
    pub pool: AssetPool,

    /// Tranche structure
    pub tranches: TrancheStructure,

    /// Waterfall distribution rules
    pub waterfall: StructuredCreditWaterfall,

    /// Coverage tests and monitoring
    pub coverage_tests: CoverageTests,

    /// Call provisions (leveraging bond call/put infrastructure)
    #[cfg_attr(feature = "serde", serde(default))]
    pub call_provisions: crate::instruments::common::structured_credit::CallProvisionManager,

    /// Key dates
    pub closing_date: Date,
    pub first_payment_date: Date,
    pub reinvestment_end_date: Option<Date>,
    pub legal_maturity: Date,

    /// Payment frequency for the structure
    pub payment_frequency: Frequency,

    /// Manager/servicer information
    pub manager_id: Option<String>,
    pub servicer_id: Option<String>,

    /// Discount curve for valuation
    pub disc_id: CurveId,

    /// Attributes for scenario selection
    pub attributes: Attributes,

    /// Prepayment model specification
    #[cfg_attr(feature = "serde", serde(default = "Clo::default_prepayment_spec"))]
    pub prepayment_spec: PrepaymentModelSpec,

    #[cfg_attr(feature = "serde", serde(skip))]
    #[builder(skip)]
    prepayment_model_cache: once_cell::sync::OnceCell<Arc<dyn PrepaymentBehavior>>,

    /// Default model specification
    #[cfg_attr(feature = "serde", serde(default = "Clo::default_default_spec"))]
    pub default_spec: DefaultModelSpec,

    #[cfg_attr(feature = "serde", serde(skip))]
    #[builder(skip)]
    default_model_cache: once_cell::sync::OnceCell<Arc<dyn DefaultBehavior>>,

    /// Recovery model specification
    #[cfg_attr(feature = "serde", serde(default = "Clo::default_recovery_spec"))]
    pub recovery_spec: RecoveryModelSpec,

    #[cfg_attr(feature = "serde", serde(skip))]
    #[builder(skip)]
    recovery_model_cache: once_cell::sync::OnceCell<Arc<dyn RecoveryBehavior>>,

    /// Market conditions for prepayment behavior
    pub market_conditions: MarketConditions,

    /// Credit factors for default behavior
    pub credit_factors: CreditFactors,
}

impl Clo {
    #[cfg(feature = "serde")]
    fn default_prepayment_spec() -> PrepaymentModelSpec {
        PrepaymentModelSpec::ConstantCpr { cpr: 0.15 }
    }

    #[cfg(feature = "serde")]
    fn default_default_spec() -> DefaultModelSpec {
        DefaultModelSpec::AssetDefault {
            asset_type: "corporate".to_string(),
        }
    }

    #[cfg(feature = "serde")]
    fn default_recovery_spec() -> RecoveryModelSpec {
        RecoveryModelSpec::AssetDefault {
            asset_type: "corporate".to_string(),
        }
    }
    /// Create a new CLO instrument from its building blocks.
    pub fn new(
        id: impl Into<String>,
        pool: AssetPool,
        tranches: TrancheStructure,
        waterfall: StructuredCreditWaterfall,
        legal_maturity: Date,
        disc_id: impl Into<String>,
    ) -> Self {
        let id_str = id.into();
        Self {
            id: InstrumentId::new(id_str),
            deal_type: DealType::CLO,
            pool,
            tranches,
            waterfall,
            coverage_tests: CoverageTests::new(),
            call_provisions: crate::instruments::common::structured_credit::CallProvisionManager::new(),
            closing_date: Date::from_calendar_date(2025, time::Month::January, 1).unwrap(),
            first_payment_date: Date::from_calendar_date(2025, time::Month::April, 1).unwrap(),
            reinvestment_end_date: None,
            legal_maturity,
            payment_frequency: Frequency::quarterly(),
            manager_id: None,
            servicer_id: None,
            disc_id: CurveId::new(disc_id.into()),
            attributes: Attributes::new(),
            prepayment_spec: PrepaymentModelSpec::ConstantCpr { cpr: 0.15 },
            prepayment_model_cache: once_cell::sync::OnceCell::new(),
            default_spec: DefaultModelSpec::AssetDefault {
                asset_type: "corporate".to_string(),
            },
            default_model_cache: once_cell::sync::OnceCell::new(),
            recovery_spec: RecoveryModelSpec::AssetDefault {
                asset_type: "corporate".to_string(),
            },
            recovery_model_cache: once_cell::sync::OnceCell::new(),
            market_conditions: MarketConditions::default(),
            credit_factors: CreditFactors::default(),
        }
    }

    /// Calculate current loss percentage of the pool.
    pub fn current_loss_percentage(&self) -> f64 {
        let total_balance = self.pool.total_balance().amount();
        if total_balance == 0.0 {
            return 0.0;
        }

        (self.pool.cumulative_defaults.amount() - self.pool.cumulative_recoveries.amount())
            / total_balance
            * 100.0
    }

    /// Get cashflows for a specific tranche.
    pub fn tranche_cashflows(
        &self,
        _tranche_id: &str,
        context: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<DatedFlows> {
        // This would be implemented to extract tranche-specific flows
        // from the overall structure cashflows
        let _all_flows = self.build_schedule(context, as_of)?;

        // Placeholder: return empty flows for now
        Ok(Vec::new())
    }

    /// Calculate expected life of the structure.
    pub fn expected_life(&self, as_of: Date) -> finstack_core::Result<f64> {
        // Simplified calculation based on pool WAM (approximation for WAL)
        Ok(self.pool.weighted_avg_maturity(as_of))
    }

    /// Enhanced valuation using sophisticated Discountable framework
    /// 
    /// This method leverages the existing discounting infrastructure for
    /// more accurate pricing than the basic waterfall implementation.
    pub fn value_with_discountable(
        &self,
        context: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<Money> {
        // Get sophisticated cashflow schedule
        let flows = self.build_schedule(context, as_of)?;
        
        // Use the sophisticated Discountable trait from core
        let disc = context.get_discount_ref(self.disc_id.as_str())?;
        
        // Leverage the existing discounting framework with proper day count
        flows.npv(disc, as_of, finstack_core::dates::DayCount::Act360)
    }

    /// Price individual tranche using Discountable framework
    pub fn price_tranche_with_discountable(
        &self,
        tranche_id: &str,
        context: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<Money> {
        // Get tranche-specific cashflows
        let tranche_flows = self.get_tranche_cashflows(tranche_id, context, as_of)?;
        
        // Use sophisticated discounting
        let disc = context.get_discount_ref(self.disc_id.as_str())?;
        tranche_flows.cashflows.npv(disc, as_of, finstack_core::dates::DayCount::Act360)
    }
}

impl core::fmt::Debug for Clo {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Clo")
            .field("id", &self.id)
            .field("deal_type", &self.deal_type)
            .field("closing_date", &self.closing_date)
            .field("first_payment_date", &self.first_payment_date)
            .field("legal_maturity", &self.legal_maturity)
            .field("payment_frequency", &self.payment_frequency)
            .field("disc_id", &self.disc_id)
            .finish()
    }
}

impl TrancheValuationExt for Clo {
    /// Generate cashflows for a specific tranche after waterfall allocation
    fn get_tranche_cashflows(
        &self,
        tranche_id: &str,
        context: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<TrancheCashflowResult> {
        use crate::instruments::common::structured_credit::StructuredCreditInstrument;
        <Self as StructuredCreditInstrument>::generate_specific_tranche_cashflows(
            self,
            tranche_id,
            context,
            as_of,
        )
    }

    /// Calculate present value for a specific tranche
    fn value_tranche(
        &self,
        tranche_id: &str,
        context: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<Money> {
        let cashflows = self.get_tranche_cashflows(tranche_id, context, as_of)?;
        let disc = context.get_discount(&self.disc_id)?;
        
        let mut pv = Money::new(0.0, self.pool.base_currency());
        for (date, amount) in &cashflows.cashflows {
            if *date > as_of {
                let df = disc.df_on_date_curve(*date);
                let flow_pv = Money::new(amount.amount() * df, amount.currency());
                pv = pv.checked_add(flow_pv)?;
            }
        }
        
        Ok(pv)
    }

    /// Get full valuation with metrics for a specific tranche
    fn value_tranche_with_metrics(
        &self,
        tranche_id: &str,
        context: &MarketContext,
        as_of: Date,
        metrics: &[MetricId],
    ) -> finstack_core::Result<TrancheValuation> {
        use crate::instruments::common::structured_credit::{
            calculate_tranche_cs01, calculate_tranche_duration,
            calculate_tranche_wal, calculate_tranche_z_spread,
        };
        
        // Get tranche-specific cashflows
        let cashflow_result = self.get_tranche_cashflows(tranche_id, context, as_of)?;
        
        // Calculate PV
        let pv = self.value_tranche(tranche_id, context, as_of)?;
        
        // Get tranche for notional
        let tranche = self.tranches
            .tranches
            .iter()
            .find(|t| t.id.as_str() == tranche_id)
            .ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                    id: format!("tranche:{}", tranche_id),
                })
            })?;
        
        let notional = tranche.original_balance.amount();
        
        // Calculate prices
        let dirty_price = if notional > 0.0 {
            (pv.amount() / notional) * 100.0
        } else {
            0.0
        };
        
        // Simple accrued calculation (would be more sophisticated in practice)
        let accrued = Money::new(0.0, pv.currency());
        let clean_price = dirty_price; // Simplified - would subtract accrued
        
        // Calculate metrics
        let wal = calculate_tranche_wal(&cashflow_result, as_of)?;
        
        let disc = context.get_discount(&self.disc_id)?;
        let modified_duration = calculate_tranche_duration(
            &cashflow_result.cashflows,
            &disc,
            as_of,
            pv,
        )?;
        
        let z_spread = calculate_tranche_z_spread(
            &cashflow_result.cashflows,
            &disc,
            pv,
            as_of,
        )?;
        
        let z_spread_decimal = z_spread / 10_000.0; // Convert from bps to decimal
        let cs01 = calculate_tranche_cs01(
            &cashflow_result.cashflows,
            &disc,
            z_spread_decimal,
            as_of,
        )?;
        
        // Simple YTM calculation (would use solver in practice)
        let ytm = 0.05; // Placeholder
        
        // Build metrics map
        let mut metric_values = std::collections::HashMap::new();
        for metric in metrics {
            match metric {
                MetricId::WAL => metric_values.insert(MetricId::WAL, wal),
                MetricId::DurationMod => metric_values.insert(MetricId::DurationMod, modified_duration),
                MetricId::ZSpread => metric_values.insert(MetricId::ZSpread, z_spread),
                MetricId::Cs01 => metric_values.insert(MetricId::Cs01, cs01),
                _ => None,
            };
        }
        
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
            metrics: metric_values,
        })
    }
}

impl CashflowProvider for Clo {
    fn build_schedule(
        &self,
        context: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<DatedFlows> {
        // Use shared waterfall implementation via trait
        use crate::instruments::common::structured_credit::StructuredCreditInstrument;
        <Self as StructuredCreditInstrument>::generate_tranche_cashflows(self, context, as_of)
    }
}

impl Instrument for Clo {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        <Self as crate::instruments::common::traits::InstrumentKind>::TYPE
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
        let disc = context.get_discount_ref(self.disc_id.as_str())?;
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
        // Compute base NPV
        let base_value = self.value(context, as_of)?;
        
        // If no metrics requested, return simple result
        if metrics.is_empty() {
            return Ok(ValuationResult::stamped(self.id.as_str(), as_of, base_value));
        }
        
        // Build cashflows for metrics that need them
        let flows = self.build_schedule(context, as_of)?;
        
        // Create metric context
        let mut metric_context = crate::metrics::MetricContext::new(
            std::sync::Arc::new(self.clone()) as std::sync::Arc<dyn crate::instruments::common::traits::Instrument>,
            std::sync::Arc::new(context.clone()),
            as_of,
            base_value,
        );
        
        // Cache cashflows and discount curve ID for metrics to use
        metric_context.cashflows = Some(flows);
        metric_context.discount_curve_id = Some(self.disc_id.clone());
        
        // Get standard registry
        let registry = crate::metrics::declarative_standard_registry();
        
        // Compute requested metrics
        let computed_metrics = registry.compute(metrics, &mut metric_context)?;
        
        // Build result with computed metrics
        let mut result = ValuationResult::stamped(self.id.as_str(), as_of, base_value);
        
        for (metric_id, value) in computed_metrics {
            result.measures.insert(metric_id.to_string(), value);
        }
        
        Ok(result)
    }
}

impl crate::instruments::common::traits::InstrumentKind for Clo {
    const TYPE: crate::pricer::InstrumentType = crate::pricer::InstrumentType::CLO;
}

impl crate::instruments::common::HasDiscountCurve for Clo {
    fn discount_curve_id(&self) -> &CurveId {
        &self.disc_id
    }
}

// Implement StructuredCreditInstrument trait to use shared waterfall logic
impl crate::instruments::common::structured_credit::StructuredCreditInstrument for Clo {
    fn pool(&self) -> &crate::instruments::common::structured_credit::AssetPool {
        &self.pool
    }

    fn tranches(&self) -> &crate::instruments::common::structured_credit::TrancheStructure {
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

    fn prepayment_model(
        &self,
    ) -> &Arc<dyn crate::instruments::common::structured_credit::PrepaymentBehavior> {
        self.prepayment_model_cache
            .get_or_init(|| self.prepayment_spec.to_arc())
    }

    fn default_model(
        &self,
    ) -> &Arc<dyn crate::instruments::common::structured_credit::DefaultBehavior> {
        self.default_model_cache
            .get_or_init(|| self.default_spec.to_arc())
    }

    fn recovery_model(
        &self,
    ) -> &Arc<dyn crate::instruments::common::structured_credit::RecoveryBehavior> {
        self.recovery_model_cache
            .get_or_init(|| self.recovery_spec.to_arc())
    }

    fn market_conditions(
        &self,
    ) -> &crate::instruments::common::structured_credit::MarketConditions {
        &self.market_conditions
    }

    fn credit_factors(&self) -> &crate::instruments::common::structured_credit::CreditFactors {
        &self.credit_factors
    }

    fn create_waterfall_engine(
        &self,
    ) -> crate::instruments::common::structured_credit::WaterfallEngine {
        self.create_clo_waterfall_engine()
    }
}

impl Clo {
    /// Create waterfall engine with standard CLO payment rules
    fn create_clo_waterfall_engine(
        &self,
    ) -> crate::instruments::common::structured_credit::WaterfallEngine {
        use crate::instruments::common::structured_credit::{
            ManagementFeeType, PaymentCalculation, PaymentMode, PaymentRecipient, PaymentRule,
            WaterfallEngine,
        };

        let mut engine = WaterfallEngine::new(self.pool.base_currency());
        let mut priority = 1;

        // Priority 1: Trustee fees
        engine = engine.add_rule(PaymentRule::new(
            "trustee_fees",
            priority,
            PaymentRecipient::ServiceProvider("Trustee".to_string()),
            PaymentCalculation::FixedAmount {
                amount: Money::new(50_000.0, self.pool.base_currency()),
            },
        ));
        priority += 1;

        // Priority 2: Senior management fee
        engine = engine.add_rule(PaymentRule::new(
            "senior_mgmt_fee",
            priority,
            PaymentRecipient::ManagerFee(ManagementFeeType::Senior),
            PaymentCalculation::PercentageOfCollateral {
                rate: 0.01,
                annualized: true,
            },
        ));
        priority += 1;

        // Add interest payments for each tranche
        let mut sorted_tranches = self.tranches.tranches.clone();
        sorted_tranches.sort_by_key(|t| t.payment_priority);
        for tranche in &sorted_tranches {
            // Do not schedule interest for Equity; equity receives residual only
            if tranche.seniority == TrancheSeniority::Equity {
                continue;
            }
            engine = engine.add_rule(PaymentRule::new(
                format!("{}_interest", tranche.id.as_str()),
                priority,
                PaymentRecipient::Tranche(tranche.id.to_string()),
                PaymentCalculation::TrancheInterest {
                    tranche_id: tranche.id.to_string(),
                },
            ));
            priority += 1;
        }

        // Add principal payments for debt tranches
        for tranche in &sorted_tranches {
            if tranche.seniority != TrancheSeniority::Equity {
                engine = engine.add_rule(
                    PaymentRule::new(
                        format!("{}_principal", tranche.id.as_str()),
                        priority,
                        PaymentRecipient::Tranche(tranche.id.to_string()),
                        PaymentCalculation::TranchePrincipal {
                            tranche_id: tranche.id.to_string(),
                            target_balance: None,
                        },
                    )
                    .divertible(),
                );
            }
        }
        priority += 1;
        
        // Add equity distribution
        engine = engine.add_rule(PaymentRule::new(
            "equity_distribution",
            priority,
            PaymentRecipient::Equity,
            PaymentCalculation::ResidualCash,
        ));

        engine.payment_mode = PaymentMode::ProRata;
        engine
    }
}
