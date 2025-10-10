//! Commercial Mortgage-Backed Security (CMBS) instrument powered by shared structured credit components.

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

/// Primary CMBS instrument representation.
#[derive(Clone, finstack_valuations_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Cmbs {
    /// Unique instrument identifier
    pub id: InstrumentId,

    /// Deal classification (always `DealType::CMBS`)
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
    pub master_servicer_id: Option<String>,
    pub special_servicer_id: Option<String>,

    /// Discount curve for valuation
    pub disc_id: CurveId,

    /// Attributes for scenario selection
    pub attributes: Attributes,

    /// Prepayment model (commercial prepayment behavior)
    #[cfg_attr(feature = "serde", serde(default = "Cmbs::default_prepayment_spec"))]
    pub prepayment_spec: PrepaymentModelSpec,

    #[cfg_attr(feature = "serde", serde(skip))]
    #[builder(skip)]
    prepayment_model_cache: once_cell::sync::OnceCell<Arc<dyn PrepaymentBehavior>>,

    /// Default model specification
    #[cfg_attr(feature = "serde", serde(default = "Cmbs::default_default_spec"))]
    pub default_spec: DefaultModelSpec,

    #[cfg_attr(feature = "serde", serde(skip))]
    #[builder(skip)]
    default_model_cache: once_cell::sync::OnceCell<Arc<dyn DefaultBehavior>>,

    /// Recovery model specification
    #[cfg_attr(feature = "serde", serde(default = "Cmbs::default_recovery_spec"))]
    pub recovery_spec: RecoveryModelSpec,

    #[cfg_attr(feature = "serde", serde(skip))]
    #[builder(skip)]
    recovery_model_cache: once_cell::sync::OnceCell<Arc<dyn RecoveryBehavior>>,

    /// Market conditions (e.g., refi markets less relevant; seasonality)
    pub market_conditions: MarketConditions,

    /// Credit factors (DSCR/LTV proxies via `credit_factors` if extended later)
    pub credit_factors: CreditFactors,

    /// Instrument-level knobs
    pub open_cpr: Option<f64>,
    pub cdr_annual: Option<f64>,
}

impl Cmbs {
    /// Create a new CMBS instrument from its building blocks.
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
            deal_type: DealType::CMBS,
            pool,
            tranches,
            waterfall,
            coverage_tests: CoverageTests::new(),
            closing_date: Date::from_calendar_date(2025, Month::January, 1).unwrap(),
            first_payment_date: Date::from_calendar_date(2025, Month::February, 1).unwrap(),
            reinvestment_end_date: None,
            legal_maturity,
            payment_frequency: Frequency::monthly(),
            master_servicer_id: None,
            special_servicer_id: None,
            disc_id: CurveId::new(disc_id.into()),
            attributes: Attributes::new(),
            prepayment_spec: PrepaymentModelSpec::AssetDefault {
                asset_type: "cmbs".to_string(),
            },
            prepayment_model_cache: once_cell::sync::OnceCell::new(),
            default_spec: DefaultModelSpec::AssetDefault {
                asset_type: "commercial".to_string(),
            },
            default_model_cache: once_cell::sync::OnceCell::new(),
            recovery_spec: RecoveryModelSpec::AssetDefault {
                asset_type: "commercial".to_string(),
            },
            recovery_model_cache: once_cell::sync::OnceCell::new(),
            market_conditions: MarketConditions::default(),
            credit_factors: CreditFactors::default(),
            open_cpr: None,
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
        let _all_flows = self.build_schedule(context, as_of)?;
        Ok(Vec::new())
    }

    /// Calculate expected life of the structure.
    pub fn expected_life(&self, as_of: Date) -> finstack_core::Result<f64> {
        // Use WAM as approximation for WAL
        Ok(self.pool.weighted_avg_maturity(as_of))
    }
}

impl CashflowProvider for Cmbs {
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

impl Instrument for Cmbs {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::CMBS
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
        
        let registry = crate::metrics::standard_registry();
        let computed_metrics = registry.compute(metrics, &mut metric_context)?;
        
        let mut result = ValuationResult::stamped(self.id.as_str(), as_of, base_value);
        for (metric_id, value) in computed_metrics {
            result.measures.insert(metric_id.to_string(), value);
        }
        
        Ok(result)
    }
}


impl crate::instruments::common::pricing::HasDiscountCurve for Cmbs {
    fn discount_curve_id(&self) -> &CurveId {
        &self.disc_id
    }
}

impl Cmbs {
    #[cfg(feature = "serde")]
    fn default_prepayment_spec() -> PrepaymentModelSpec {
        PrepaymentModelSpec::AssetDefault {
            asset_type: "cmbs".to_string(),
        }
    }

    #[cfg(feature = "serde")]
    fn default_default_spec() -> DefaultModelSpec {
        DefaultModelSpec::AssetDefault {
            asset_type: "commercial".to_string(),
        }
    }

    #[cfg(feature = "serde")]
    fn default_recovery_spec() -> RecoveryModelSpec {
        RecoveryModelSpec::AssetDefault {
            asset_type: "commercial".to_string(),
        }
    }

    /// Create waterfall engine for CMBS (called by trait)
    fn create_cmbs_waterfall_engine(
        &self,
    ) -> crate::instruments::common::structured_credit::WaterfallEngine {
        use crate::instruments::common::structured_credit::{
            PaymentCalculation, PaymentRecipient, PaymentRule, WaterfallEngine,
        };

        let base_ccy = self.pool.base_currency();
        
        // Define CMBS-specific fees
        let fees = vec![
            PaymentRule::new(
                "master_servicing",
                1,
                PaymentRecipient::ServiceProvider("MasterServicer".to_string()),
                PaymentCalculation::PercentageOfCollateral {
                    rate: 0.0025, // 25 bps
                    annualized: true,
                },
            ),
        ];
        
        // Use shared waterfall construction
        WaterfallEngine::standard_sequential(base_ccy, &self.tranches, fees)
    }
}

// Implement StructuredCreditInstrument trait to use shared waterfall logic
impl crate::instruments::common::structured_credit::StructuredCreditInstrument for Cmbs {
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
        self.create_cmbs_waterfall_engine()
    }

    // CMBS-specific prepayment override
    fn prepayment_rate_override(&self, _pay_date: Date, _seasoning: u32) -> Option<f64> {
        self.open_cpr.map(|cpr| {
            use crate::instruments::common::structured_credit::cpr_to_smm;
            cpr_to_smm(cpr)
        })
    }

    // CMBS-specific default override
    fn default_rate_override(&self, _pay_date: Date, _seasoning: u32) -> Option<f64> {
        self.cdr_annual.map(|cdr| {
            use crate::instruments::common::structured_credit::cdr_to_mdr;
            cdr_to_mdr(cdr)
        })
    }
}

impl core::fmt::Debug for Cmbs {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Cmbs")
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
