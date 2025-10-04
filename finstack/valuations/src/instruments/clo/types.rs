//! Collateralized Loan Obligation (CLO) instrument built on the shared structured credit core.

use crate::cashflow::traits::{CashflowProvider, DatedFlows};
use crate::instruments::common::structured_credit::{
    AssetPool, CoverageTests, DealType, StructuredCreditWaterfall, TrancheStructure,
    // Prepayment/default frameworks
    PrepaymentBehavior, DefaultBehavior, RecoveryBehavior,
    PrepaymentModelFactory, DefaultModelFactory,
    MarketConditions, CreditFactors,
};
use crate::instruments::common::traits::{Attributes, Instrument};
use crate::metrics::MetricId;
use crate::results::ValuationResult;
use finstack_core::dates::utils::add_months;
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
    #[cfg_attr(feature = "serde", serde(skip_serializing, skip_deserializing, default = "Clo::default_prepayment_arc"))]
    pub prepayment_model: Arc<dyn PrepaymentBehavior>,

    /// Default model (MDR) applied to pool cashflows
    #[cfg_attr(feature = "serde", serde(skip_serializing, skip_deserializing, default = "Clo::default_default_arc"))]
    pub default_model: Arc<dyn DefaultBehavior>,

    /// Recovery model used to convert defaults to recoveries
    #[cfg_attr(feature = "serde", serde(skip_serializing, skip_deserializing, default = "Clo::default_recovery_arc"))]
    pub recovery_model: Arc<dyn RecoveryBehavior>,

    /// Market conditions for prepayment behavior
    pub market_conditions: MarketConditions,

    /// Credit factors for default behavior
    pub credit_factors: CreditFactors,
}

impl Clo {
    #[cfg(feature = "serde")]
    fn default_prepayment_arc() -> Arc<dyn PrepaymentBehavior> {
        Arc::from(PrepaymentModelFactory::create_cpr(0.15))
    }

    #[cfg(feature = "serde")]
    fn default_default_arc() -> Arc<dyn DefaultBehavior> {
        Arc::from(DefaultModelFactory::create_default_model("corporate"))
    }

    #[cfg(feature = "serde")]
    fn default_recovery_arc() -> Arc<dyn RecoveryBehavior> {
        Arc::from(DefaultModelFactory::create_recovery_model("corporate"))
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
        let prepay = PrepaymentModelFactory::create_cpr(0.15); // 15% CPR default
        let dflt = DefaultModelFactory::create_default_model("corporate");
        let recv = DefaultModelFactory::create_recovery_model("corporate");
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
        // Simplified calculation based on pool WAL
        Ok(self.pool.weighted_avg_life(as_of))
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
        _context: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<DatedFlows> {
        // Simplified pool-level simulation applying SMM (prepay) and MDR (default)
        let mut flows: DatedFlows = Vec::new();

        let base_ccy = self.pool.base_currency();
        let mut outstanding = self.pool.total_balance();
        if outstanding.amount() <= 0.0 {
            return Ok(flows);
        }

        let months_per_period = self.payment_frequency.months().unwrap_or(3) as f64;
        let wac = self.pool.weighted_avg_coupon();
        // Periodic interest rate based on frequency
        let period_rate = wac * (months_per_period / 12.0);

        let mut pay_date = self.first_payment_date.max(as_of);
        while pay_date <= self.legal_maturity && outstanding.amount() > 0.0 {
            // Seasoning relative to closing date
            let seasoning_months = {
                let m = (pay_date.year() - self.closing_date.year()) * 12
                    + (pay_date.month() as i32 - self.closing_date.month() as i32);
                m.max(0) as u32
            };

            // Prepayment and default rates
            let smm = self.premium_smm(pay_date, seasoning_months);
            let mdr = self.premium_mdr(pay_date, seasoning_months);

            // Interest on current outstanding
            let interest_amt = Money::new(outstanding.amount() * period_rate, base_ccy);

            // No scheduled amortization (bullet) in this simplified pass
            let scheduled_prin = Money::new(0.0, base_ccy);

            // Prepayments
            let prepay_amt = Money::new(outstanding.amount() * smm, base_ccy);

            // Defaults and recoveries (recovery received immediately for simplicity)
            let default_amt = Money::new(outstanding.amount() * mdr, base_ccy);
            let recovery_rate = self.recovery_model.recovery_rate(
                pay_date,
                6,
                None,
                default_amt,
                &crate::instruments::common::structured_credit::MarketFactors::default(),
            );
            let recovery_amt = Money::new(default_amt.amount() * recovery_rate, base_ccy);

            // Cash collected this period (interest + principal components)
            let period_cash = interest_amt
                .checked_add(scheduled_prin)?
                .checked_add(prepay_amt)?
                .checked_add(recovery_amt)?;
            flows.push((pay_date, period_cash));

            // Update outstanding balance after scheduled principal, prepay, and default
            outstanding = outstanding
                .checked_sub(scheduled_prin)?
                .checked_sub(prepay_amt)?
                .checked_sub(default_amt)?;

            // Advance to next period
            pay_date = add_months(pay_date, self.payment_frequency.months().unwrap_or(3) as i32);
        }

        Ok(flows)
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

impl Clo {
    #[inline]
    fn premium_smm(&self, as_of: Date, seasoning_months: u32) -> f64 {
        self.prepayment_model
            .prepayment_rate(as_of, self.closing_date, seasoning_months, &self.market_conditions)
            .max(0.0)
    }

    #[inline]
    fn premium_mdr(&self, as_of: Date, seasoning_months: u32) -> f64 {
        self.default_model
            .default_rate(as_of, self.closing_date, seasoning_months, &self.credit_factors)
            .max(0.0)
    }
}
