//! Asset-Backed Security (ABS) instrument built on the shared structured credit core.

use crate::cashflow::traits::{CashflowProvider, DatedFlows};
use crate::instruments::common::structured_credit::{
    AssetPool, CoverageTests, CreditFactors, DealType, DefaultBehavior, MarketConditions,
    PrepaymentBehavior, RecoveryBehavior, StructuredCreditWaterfall, TrancheStructure,
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

/// Primary ABS instrument representation.
#[derive(Clone, finstack_valuations_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Abs {
    /// Unique instrument identifier
    pub id: InstrumentId,

    /// Deal classification (always `DealType::ABS`)
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

    /// Servicer/administrator information
    pub servicer_id: Option<String>,
    pub trustee_id: Option<String>,

    /// Discount curve for valuation
    pub disc_id: CurveId,

    /// Attributes for scenario selection
    pub attributes: Attributes,

    /// Prepayment model (SMM) for receivables/loans
    #[cfg_attr(
        feature = "serde",
        serde(
            skip_serializing,
            skip_deserializing,
            default = "Abs::default_prepayment_arc"
        )
    )]
    pub prepayment_model: Arc<dyn PrepaymentBehavior>,

    /// Default model (MDR) for receivables/loans
    #[cfg_attr(
        feature = "serde",
        serde(
            skip_serializing,
            skip_deserializing,
            default = "Abs::default_default_arc"
        )
    )]
    pub default_model: Arc<dyn DefaultBehavior>,

    /// Recovery model for ABS collateral
    #[cfg_attr(
        feature = "serde",
        serde(
            skip_serializing,
            skip_deserializing,
            default = "Abs::default_recovery_arc"
        )
    )]
    pub recovery_model: Arc<dyn RecoveryBehavior>,

    /// Market conditions impacting prepayments
    pub market_conditions: MarketConditions,

    /// Credit factors impacting default behavior
    pub credit_factors: CreditFactors,

    /// Instrument-level knobs
    pub abs_speed: Option<f64>,
    pub cdr_annual: Option<f64>,
}

impl Abs {
    /// Create a new ABS instrument from its building blocks.
    pub fn new(
        id: impl Into<String>,
        pool: AssetPool,
        tranches: TrancheStructure,
        waterfall: StructuredCreditWaterfall,
        legal_maturity: Date,
        disc_id: impl Into<String>,
    ) -> Self {
        let id_str = id.into();
        // Generic ABS defaults (overridable by attributes or asset mix)
        use crate::instruments::common::structured_credit::{prepayment_model_for, default_model_for, recovery_model_for};
        let prepay = prepayment_model_for("auto");
        let dflt = default_model_for("consumer");
        let recv = recovery_model_for("consumer");
        Self {
            id: InstrumentId::new(id_str),
            deal_type: DealType::ABS,
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
            trustee_id: None,
            disc_id: CurveId::new(disc_id.into()),
            attributes: Attributes::new(),
            prepayment_model: Arc::from(prepay),
            default_model: Arc::from(dflt),
            recovery_model: Arc::from(recv),
            market_conditions: MarketConditions::default(),
            credit_factors: CreditFactors::default(),
            abs_speed: None,
            cdr_annual: None,
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
        // Simplified calculation based on pool WAL
        Ok(self.pool.weighted_avg_life(as_of))
    }
}

impl CashflowProvider for Abs {
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

impl Instrument for Abs {
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

impl crate::instruments::common::traits::InstrumentKind for Abs {
    const TYPE: crate::pricer::InstrumentType = crate::pricer::InstrumentType::ABS;
}

impl crate::instruments::common::HasDiscountCurve for Abs {
    fn discount_curve_id(&self) -> &CurveId {
        &self.disc_id
    }
}

// Implement StructuredCreditInstrument trait to use shared waterfall logic
impl crate::instruments::common::structured_credit::StructuredCreditInstrument for Abs {
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
        self.create_abs_waterfall_engine()
    }

    // ABS-specific prepayment override
    fn prepayment_rate_override(&self, _pay_date: Date, _seasoning: u32) -> Option<f64> {
        self.abs_speed // If set, use the ABS speed directly
    }

    // ABS-specific default override
    fn default_rate_override(&self, _pay_date: Date, _seasoning: u32) -> Option<f64> {
        self.cdr_annual.map(|cdr| {
            use crate::instruments::common::structured_credit::cdr_to_mdr;
            cdr_to_mdr(cdr)
        })
    }
}

impl Abs {
    #[cfg(feature = "serde")]
    fn default_prepayment_arc() -> Arc<dyn PrepaymentBehavior> {
        use crate::instruments::common::structured_credit::prepayment_model_for;
        Arc::from(prepayment_model_for("auto"))
    }

    #[cfg(feature = "serde")]
    fn default_default_arc() -> Arc<dyn DefaultBehavior> {
        use crate::instruments::common::structured_credit::default_model_for;
        Arc::from(default_model_for("consumer"))
    }

    #[cfg(feature = "serde")]
    fn default_recovery_arc() -> Arc<dyn RecoveryBehavior> {
        use crate::instruments::common::structured_credit::recovery_model_for;
        Arc::from(recovery_model_for("consumer"))
    }

    /// Create waterfall engine for ABS (called by trait)
    fn create_abs_waterfall_engine(
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
                rate: 0.005, // 50 bps servicing
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

impl core::fmt::Debug for Abs {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Abs")
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
