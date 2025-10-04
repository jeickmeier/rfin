//! Commercial Mortgage-Backed Security (CMBS) instrument powered by shared structured credit components.

use crate::cashflow::traits::{CashflowProvider, DatedFlows};
use crate::instruments::common::structured_credit::{
    AssetPool, CoverageTests, DealType, StructuredCreditWaterfall, TrancheStructure,
    PrepaymentBehavior, DefaultBehavior, RecoveryBehavior,
    PrepaymentModelFactory, DefaultModelFactory,
    MarketConditions, CreditFactors,
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
    #[cfg_attr(feature = "serde", serde(skip_serializing, skip_deserializing, default = "Cmbs::default_prepayment_arc"))]
    pub prepayment_model: Arc<dyn PrepaymentBehavior>,

    /// Default model for commercial loans
    #[cfg_attr(feature = "serde", serde(skip_serializing, skip_deserializing, default = "Cmbs::default_default_arc"))]
    pub default_model: Arc<dyn DefaultBehavior>,

    /// Recovery model for commercial collateral
    #[cfg_attr(feature = "serde", serde(skip_serializing, skip_deserializing, default = "Cmbs::default_recovery_arc"))]
    pub recovery_model: Arc<dyn RecoveryBehavior>,

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
        // Defaults for CMBS: commercial prepayment model via factory("cmbs"), commercial defaults
        let prepay = PrepaymentModelFactory::create_default("cmbs");
        let dflt = DefaultModelFactory::create_default_model("commercial");
        let recv = DefaultModelFactory::create_recovery_model("commercial");
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
            prepayment_model: Arc::from(prepay),
            default_model: Arc::from(dflt),
            recovery_model: Arc::from(recv),
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
        Ok(self.pool.weighted_avg_life(as_of))
    }
}

impl CashflowProvider for Cmbs {
    fn build_schedule(
        &self,
        context: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<DatedFlows> {
        // Generate full tranche-specific cashflows with waterfall distribution
        self.generate_tranche_cashflows_cmbs(context, as_of)
    }
}

impl Instrument for Cmbs {
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

impl crate::instruments::common::traits::InstrumentKind for Cmbs {
    const TYPE: crate::pricer::InstrumentType = crate::pricer::InstrumentType::CMBS;
}

impl crate::instruments::common::HasDiscountCurve for Cmbs {
    fn discount_curve_id(&self) -> &CurveId {
        &self.disc_id
    }
}

impl Cmbs {
    #[cfg(feature = "serde")]
    fn default_prepayment_arc() -> Arc<dyn PrepaymentBehavior> {
        Arc::from(PrepaymentModelFactory::create_default("cmbs"))
    }

    #[cfg(feature = "serde")]
    fn default_default_arc() -> Arc<dyn DefaultBehavior> {
        Arc::from(DefaultModelFactory::create_default_model("commercial"))
    }

    #[cfg(feature = "serde")]
    fn default_recovery_arc() -> Arc<dyn RecoveryBehavior> {
        Arc::from(DefaultModelFactory::create_recovery_model("commercial"))
    }

    #[inline]
    pub(super) fn premium_smm(&self, as_of: Date, seasoning_months: u32) -> f64 {
        self.prepayment_model
            .prepayment_rate(as_of, self.closing_date, seasoning_months, &self.market_conditions)
            .max(0.0)
    }

    #[inline]
    pub(super) fn premium_mdr(&self, as_of: Date, seasoning_months: u32) -> f64 {
        self.default_model
            .default_rate(as_of, self.closing_date, seasoning_months, &self.credit_factors)
            .max(0.0)
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
