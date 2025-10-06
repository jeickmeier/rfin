//! Residential Mortgage-Backed Security (RMBS) instrument leveraging shared structured credit components.

use crate::cashflow::traits::{CashflowProvider, DatedFlows};
use crate::instruments::common::structured_credit::{
    AssetPool, CoverageTests, CreditFactors, DealType, DefaultBehavior, DefaultModelSpec,
    MarketConditions, PrepaymentBehavior, PrepaymentModelSpec, RecoveryBehavior,
    RecoveryModelSpec, StructuredCreditWaterfall, TrancheStructure,
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
use time::Month;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Primary RMBS instrument representation.
#[derive(Clone, finstack_valuations_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Rmbs {
    /// Unique instrument identifier
    pub id: InstrumentId,

    /// Deal classification (always `DealType::RMBS`)
    pub deal_type: DealType,

    /// Asset pool definition
    pub pool: AssetPool,

    /// Tranche structure
    pub tranches: TrancheStructure,

    /// Waterfall distribution rules
    pub waterfall: StructuredCreditWaterfall,

    /// Coverage tests and monitoring
    pub coverage_tests: CoverageTests,

    /// Key dates
    pub closing_date: Date,
    pub first_payment_date: Date,
    pub reinvestment_end_date: Option<Date>,
    pub legal_maturity: Date,

    /// Payment frequency for the structure
    pub payment_frequency: Frequency,

    /// Servicing parties
    pub servicer_id: Option<String>,
    pub master_servicer_id: Option<String>,

    /// Discount curve for valuation
    pub disc_id: CurveId,

    /// Attributes for scenario selection
    pub attributes: Attributes,

    /// Prepayment model specification
    #[cfg_attr(feature = "serde", serde(default = "Rmbs::default_prepayment_spec"))]
    pub prepayment_spec: PrepaymentModelSpec,

    /// Cached prepayment model (lazily initialized from spec)
    #[cfg_attr(feature = "serde", serde(skip))]
    #[builder(skip)]
    prepayment_model_cache: once_cell::sync::OnceCell<Arc<dyn PrepaymentBehavior>>,

    /// Default model specification
    #[cfg_attr(feature = "serde", serde(default = "Rmbs::default_default_spec"))]
    pub default_spec: DefaultModelSpec,

    /// Cached default model (lazily initialized from spec)
    #[cfg_attr(feature = "serde", serde(skip))]
    #[builder(skip)]
    default_model_cache: once_cell::sync::OnceCell<Arc<dyn DefaultBehavior>>,

    /// Recovery model specification
    #[cfg_attr(feature = "serde", serde(default = "Rmbs::default_recovery_spec"))]
    pub recovery_spec: RecoveryModelSpec,

    /// Cached recovery model (lazily initialized from spec)
    #[cfg_attr(feature = "serde", serde(skip))]
    #[builder(skip)]
    recovery_model_cache: once_cell::sync::OnceCell<Arc<dyn RecoveryBehavior>>,

    /// Market conditions (refi rate, HPA, seasonality)
    pub market_conditions: MarketConditions,

    /// Credit factors (LTV, FICO)
    pub credit_factors: CreditFactors,

    /// Instrument-level knobs
    pub psa_speed: f64,
    pub sda_speed: f64,
}

impl Rmbs {
    /// Create a new RMBS instrument from its building blocks.
    pub fn new(
        id: impl Into<String>,
        pool: AssetPool,
        tranches: TrancheStructure,
        waterfall: StructuredCreditWaterfall,
        legal_maturity: Date,
        disc_id: impl Into<String>,
    ) -> Self {
        let id_str = id.into();
        let credit_factors = CreditFactors {
            ltv: Some(0.80),
            ..Default::default()
        };
        Self {
            id: InstrumentId::new(id_str),
            deal_type: DealType::RMBS,
            pool,
            tranches,
            waterfall,
            coverage_tests: CoverageTests::new(),
            closing_date: Date::from_calendar_date(2025, Month::January, 1).unwrap(),
            first_payment_date: Date::from_calendar_date(2025, Month::February, 1).unwrap(),
            reinvestment_end_date: None,
            legal_maturity,
            payment_frequency: Frequency::monthly(),
            servicer_id: None,
            master_servicer_id: None,
            disc_id: CurveId::new(disc_id.into()),
            attributes: Attributes::new(),
            prepayment_spec: PrepaymentModelSpec::Psa { multiplier: 1.0 },
            prepayment_model_cache: once_cell::sync::OnceCell::new(),
            default_spec: DefaultModelSpec::AssetDefault {
                asset_type: "rmbs".to_string(),
            },
            default_model_cache: once_cell::sync::OnceCell::new(),
            recovery_spec: RecoveryModelSpec::AssetDefault {
                asset_type: "mortgage".to_string(),
            },
            recovery_model_cache: once_cell::sync::OnceCell::new(),
            market_conditions: MarketConditions::default(),
            credit_factors,
            psa_speed: 1.0,
            sda_speed: 1.0,
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
        let _all_flows = self.build_schedule(context, as_of)?;
        Ok(Vec::new())
    }

    /// Calculate expected life of the structure.
    pub fn expected_life(&self, as_of: Date) -> finstack_core::Result<f64> {
        // Use WAM as approximation for WAL
        Ok(self.pool.weighted_avg_maturity(as_of))
    }
}

impl CashflowProvider for Rmbs {
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

impl Instrument for Rmbs {
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
        let base_value = self.value(context, as_of)?;
        
        if metrics.is_empty() {
            return Ok(ValuationResult::stamped(self.id.as_str(), as_of, base_value));
        }
        
        let flows = self.build_schedule(context, as_of)?;
        let mut metric_context = crate::metrics::MetricContext::new(
            std::sync::Arc::new(self.clone()) as std::sync::Arc<dyn crate::instruments::common::traits::Instrument>,
            std::sync::Arc::new(context.clone()),
            as_of,
            base_value,
        );
        metric_context.cashflows = Some(flows);
        metric_context.discount_curve_id = Some(self.disc_id.clone());
        
        let registry = crate::metrics::declarative_standard_registry();
        let computed_metrics = registry.compute(metrics, &mut metric_context)?;
        
        let mut result = ValuationResult::stamped(self.id.as_str(), as_of, base_value);
        for (metric_id, value) in computed_metrics {
            result.measures.insert(metric_id.to_string(), value);
        }
        
        Ok(result)
    }
}

impl crate::instruments::common::traits::InstrumentKind for Rmbs {
    const TYPE: crate::pricer::InstrumentType = crate::pricer::InstrumentType::RMBS;
}

impl crate::instruments::common::HasDiscountCurve for Rmbs {
    fn discount_curve_id(&self) -> &CurveId {
        &self.disc_id
    }
}

// Implement StructuredCreditInstrument trait to use shared waterfall logic
impl crate::instruments::common::structured_credit::StructuredCreditInstrument for Rmbs {
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
        self.create_rmbs_waterfall_engine()
    }

    // RMBS-specific prepayment override (PSA speed)
    fn prepayment_rate_override(&self, _pay_date: Date, seasoning_months: u32) -> Option<f64> {
        if self.psa_speed != 1.0 {
            use crate::instruments::common::structured_credit::{cpr_to_smm, PSAModel};
            let psa = PSAModel::new(self.psa_speed);
            let cpr = psa.cpr_at_month(seasoning_months);
            Some(cpr_to_smm(cpr))
        } else {
            None
        }
    }

    // RMBS-specific default override (SDA speed)
    fn default_rate_override(&self, pay_date: Date, seasoning_months: u32) -> Option<f64> {
        if self.sda_speed != 1.0 {
            use crate::instruments::common::structured_credit::{DefaultBehavior, SDAModel};
            let sda = SDAModel {
                speed: self.sda_speed,
                ..Default::default()
            };
            Some(sda.default_rate(
                pay_date,
                self.closing_date,
                seasoning_months,
                &self.credit_factors,
            ))
        } else {
            None
        }
    }
}

impl Rmbs {
    #[cfg(feature = "serde")]
    fn default_prepayment_spec() -> PrepaymentModelSpec {
        PrepaymentModelSpec::Psa { multiplier: 1.0 }
    }

    #[cfg(feature = "serde")]
    fn default_default_spec() -> DefaultModelSpec {
        DefaultModelSpec::AssetDefault {
            asset_type: "rmbs".to_string(),
        }
    }

    #[cfg(feature = "serde")]
    fn default_recovery_spec() -> RecoveryModelSpec {
        RecoveryModelSpec::AssetDefault {
            asset_type: "mortgage".to_string(),
        }
    }

    /// Create waterfall engine for RMBS (called by trait)
    fn create_rmbs_waterfall_engine(
        &self,
    ) -> crate::instruments::common::structured_credit::WaterfallEngine {
        use crate::instruments::common::structured_credit::{
            PaymentCalculation, PaymentRecipient, PaymentRule, WaterfallEngine,
        };

        let mut engine = WaterfallEngine::new(self.pool.base_currency());

        engine.payment_rules.push(PaymentRule {
            id: "servicing_fees".to_string(),
            priority: 1,
            recipient: PaymentRecipient::ServiceProvider("Servicer".to_string()),
            calculation: PaymentCalculation::PercentageOfCollateral {
                rate: 0.0025, // 25 bps servicing
                annualized: true,
            },
            conditions: vec![],
            divertible: false,
        });

        let mut sorted_tranches = self.tranches.tranches.clone();
        sorted_tranches.sort_by_key(|t| t.payment_priority);

        let mut priority = 2;
        for tranche in &sorted_tranches {
            engine.payment_rules.push(PaymentRule {
                id: format!("{}_interest", tranche.id.as_str()),
                priority,
                recipient: PaymentRecipient::Tranche(tranche.id.to_string()),
                calculation: PaymentCalculation::TrancheInterest {
                    tranche_id: tranche.id.to_string(),
                },
                conditions: vec![],
                divertible: false,
            });
            priority += 1;
        }

        for tranche in &sorted_tranches {
            engine.payment_rules.push(PaymentRule {
                id: format!("{}_principal", tranche.id.as_str()),
                priority,
                recipient: PaymentRecipient::Tranche(tranche.id.to_string()),
                calculation: PaymentCalculation::TranchePrincipal {
                    tranche_id: tranche.id.to_string(),
                    target_balance: Some(Money::new(0.0, self.pool.base_currency())),
                },
                conditions: vec![],
                divertible: true,
            });
            priority += 1;
        }

        engine
    }
}

impl core::fmt::Debug for Rmbs {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Rmbs")
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
