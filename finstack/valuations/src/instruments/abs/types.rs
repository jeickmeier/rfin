//! Asset-Backed Security (ABS) instrument built on the shared structured credit core.

use crate::cashflow::traits::{CashflowProvider, DatedFlows};
use crate::constants::DECIMAL_TO_PERCENT;
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
    #[cfg_attr(feature = "serde", serde(default = "Abs::default_prepayment_spec"))]
    pub prepayment_spec: PrepaymentModelSpec,

    #[cfg_attr(feature = "serde", serde(skip))]
    #[builder(skip)]
    prepayment_model_cache: once_cell::sync::OnceCell<Arc<dyn PrepaymentBehavior>>,

    /// Default model specification
    #[cfg_attr(feature = "serde", serde(default = "Abs::default_default_spec"))]
    pub default_spec: DefaultModelSpec,

    #[cfg_attr(feature = "serde", serde(skip))]
    #[builder(skip)]
    default_model_cache: once_cell::sync::OnceCell<Arc<dyn DefaultBehavior>>,

    /// Recovery model specification
    #[cfg_attr(feature = "serde", serde(default = "Abs::default_recovery_spec"))]
    pub recovery_spec: RecoveryModelSpec,

    #[cfg_attr(feature = "serde", serde(skip))]
    #[builder(skip)]
    recovery_model_cache: once_cell::sync::OnceCell<Arc<dyn RecoveryBehavior>>,

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
            prepayment_spec: PrepaymentModelSpec::AssetDefault {
                asset_type: "auto".to_string(),
            },
            prepayment_model_cache: once_cell::sync::OnceCell::new(),
            default_spec: DefaultModelSpec::AssetDefault {
                asset_type: "consumer".to_string(),
            },
            default_model_cache: once_cell::sync::OnceCell::new(),
            recovery_spec: RecoveryModelSpec::AssetDefault {
                asset_type: "collateral".to_string(),
            },
            recovery_model_cache: once_cell::sync::OnceCell::new(),
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
            * DECIMAL_TO_PERCENT
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
        crate::pricer::InstrumentType::ABS
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
        metric_context.discount_curve_id = Some(self.disc_id.to_owned());
        
        let registry = crate::metrics::standard_registry();
        let computed_metrics = registry.compute(metrics, &mut metric_context)?;
        
        let mut result = ValuationResult::stamped(self.id.as_str(), as_of, base_value);
        for (metric_id, value) in computed_metrics {
            result.measures.insert(metric_id.to_string(), value);
        }
        
        Ok(result)
    }
}


impl crate::instruments::common::pricing::HasDiscountCurve for Abs {
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
    fn default_prepayment_spec() -> PrepaymentModelSpec {
        PrepaymentModelSpec::AssetDefault {
            asset_type: "auto".to_string(),
        }
    }

    #[cfg(feature = "serde")]
    fn default_default_spec() -> DefaultModelSpec {
        DefaultModelSpec::AssetDefault {
            asset_type: "consumer".to_string(),
        }
    }

    #[cfg(feature = "serde")]
    fn default_recovery_spec() -> RecoveryModelSpec {
        RecoveryModelSpec::AssetDefault {
            asset_type: "collateral".to_string(),
        }
    }

    /// Create waterfall engine for ABS (called by trait)
    fn create_abs_waterfall_engine(
        &self,
    ) -> crate::instruments::common::structured_credit::WaterfallEngine {
        use crate::instruments::common::structured_credit::{
            PaymentCalculation, PaymentRecipient, PaymentRule, WaterfallEngine,
        };

        let base_ccy = self.pool.base_currency();
        
        // Define ABS-specific fees
        let fees = vec![
            PaymentRule::new(
                "servicing_fees",
                1,
                PaymentRecipient::ServiceProvider("Servicer".to_string()),
                PaymentCalculation::PercentageOfCollateral {
                    rate: 0.005, // 50 bps servicing
                    annualized: true,
                },
            ),
        ];
        
        // Use shared waterfall construction
        WaterfallEngine::standard_sequential(base_ccy, &self.tranches, fees)
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
