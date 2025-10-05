//! Collateralized Loan Obligation (CLO) instrument built on the shared structured credit core.

use crate::cashflow::traits::{CashflowProvider, DatedFlows};
use crate::instruments::common::structured_credit::{
    AssetPool,
    CoverageTests,
    CreditFactors,
    DealType,
    DefaultBehavior,
    MarketConditions,
    // Prepayment/default frameworks
    PrepaymentBehavior,
    RecoveryBehavior,
    StructuredCreditWaterfall,
    TrancheStructure,
    // Waterfall engine
    WaterfallEngine,
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

    /// Prepayment model (SMM) applied to pool cashflows
    #[cfg_attr(
        feature = "serde",
        serde(
            skip_serializing,
            skip_deserializing,
            default = "Clo::default_prepayment_arc"
        )
    )]
    pub prepayment_model: Arc<dyn PrepaymentBehavior>,

    /// Default model (MDR) applied to pool cashflows
    #[cfg_attr(
        feature = "serde",
        serde(
            skip_serializing,
            skip_deserializing,
            default = "Clo::default_default_arc"
        )
    )]
    pub default_model: Arc<dyn DefaultBehavior>,

    /// Recovery model used to convert defaults to recoveries
    #[cfg_attr(
        feature = "serde",
        serde(
            skip_serializing,
            skip_deserializing,
            default = "Clo::default_recovery_arc"
        )
    )]
    pub recovery_model: Arc<dyn RecoveryBehavior>,

    /// Market conditions for prepayment behavior
    pub market_conditions: MarketConditions,

    /// Credit factors for default behavior
    pub credit_factors: CreditFactors,
}

impl Clo {
    #[cfg(feature = "serde")]
    fn default_prepayment_arc() -> Arc<dyn PrepaymentBehavior> {
        use crate::instruments::common::structured_credit::cpr_model;
        Arc::from(cpr_model(0.15))
    }

    #[cfg(feature = "serde")]
    fn default_default_arc() -> Arc<dyn DefaultBehavior> {
        use crate::instruments::common::structured_credit::default_model_for;
        Arc::from(default_model_for("corporate"))
    }

    #[cfg(feature = "serde")]
    fn default_recovery_arc() -> Arc<dyn RecoveryBehavior> {
        use crate::instruments::common::structured_credit::recovery_model_for;
        Arc::from(recovery_model_for("corporate"))
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
        // Default CLO assumptions: corporate loan pool
        use crate::instruments::common::structured_credit::{
            cpr_model, default_model_for, recovery_model_for,
        };
        let prepay = cpr_model(0.15); // 15% CPR default
        let dflt = default_model_for("corporate");
        let recv = recovery_model_for("corporate");
        Self {
            id: InstrumentId::new(id_str),
            deal_type: DealType::CLO,
            pool,
            tranches,
            waterfall,
            coverage_tests: CoverageTests::new(),
            closing_date: Date::from_calendar_date(2025, time::Month::January, 1).unwrap(),
            first_payment_date: Date::from_calendar_date(2025, time::Month::April, 1).unwrap(),
            reinvestment_end_date: None,
            legal_maturity,
            payment_frequency: Frequency::quarterly(),
            manager_id: None,
            servicer_id: None,
            disc_id: CurveId::new(disc_id.into()),
            attributes: Attributes::new(),
            prepayment_model: Arc::from(prepay),
            default_model: Arc::from(dflt),
            recovery_model: Arc::from(recv),
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
        _metrics: &[MetricId],
    ) -> finstack_core::Result<ValuationResult> {
        let base_value = self.value(context, as_of)?;
        Ok(ValuationResult::stamped(
            self.id.as_str(),
            as_of,
            base_value,
        ))
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
        &self.prepayment_model
    }

    fn default_model(
        &self,
    ) -> &Arc<dyn crate::instruments::common::structured_credit::DefaultBehavior> {
        &self.default_model
    }

    fn recovery_model(
        &self,
    ) -> &Arc<dyn crate::instruments::common::structured_credit::RecoveryBehavior> {
        &self.recovery_model
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
            ManagementFeeType, PaymentCalculation, PaymentRecipient, PaymentRule,
        };

        let mut engine = WaterfallEngine::new(self.pool.base_currency());

        // Priority 1: Trustee fees
        engine.payment_rules.push(PaymentRule {
            id: "trustee_fees".to_string(),
            priority: 1,
            recipient: PaymentRecipient::ServiceProvider("Trustee".to_string()),
            calculation: PaymentCalculation::FixedAmount {
                amount: Money::new(50_000.0, self.pool.base_currency()),
            },
            conditions: vec![],
            divertible: false,
        });

        // Priority 2: Senior management fee
        engine.payment_rules.push(PaymentRule {
            id: "senior_mgmt_fee".to_string(),
            priority: 2,
            recipient: PaymentRecipient::ManagerFee(ManagementFeeType::Senior),
            calculation: PaymentCalculation::PercentageOfCollateral {
                rate: 0.01,
                annualized: true,
            },
            conditions: vec![],
            divertible: false,
        });

        // Add interest payments for each tranche in priority order
        let mut sorted_tranches = self.tranches.tranches.clone();
        sorted_tranches.sort_by_key(|t| t.payment_priority);

        let mut priority = 3;
        for tranche in &sorted_tranches {
            // Interest payment
            let mut interest_id = String::with_capacity(tranche.id.len() + 9);
            interest_id.push_str(tranche.id.as_str());
            interest_id.push_str("_interest");

            engine.payment_rules.push(PaymentRule {
                id: interest_id,
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

        // Add principal payments for each tranche
        for tranche in &sorted_tranches {
            let mut principal_id = String::with_capacity(tranche.id.len() + 10);
            principal_id.push_str(tranche.id.as_str());
            principal_id.push_str("_principal");

            engine.payment_rules.push(PaymentRule {
                id: principal_id,
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

        // Add equity distribution
        engine.payment_rules.push(PaymentRule {
            id: "equity_distribution".to_string(),
            priority,
            recipient: PaymentRecipient::Equity,
            calculation: PaymentCalculation::ResidualCash,
            conditions: vec![],
            divertible: false,
        });

        engine
    }
}
