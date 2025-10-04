//! Collateralized Loan Obligation (CLO) instrument built on the shared structured credit core.

use crate::cashflow::traits::{CashflowProvider, DatedFlows};
use crate::instruments::common::structured_credit::{
    AssetPool, CoverageTests, DealType, StructuredCreditWaterfall, TrancheStructure,
    // Prepayment/default frameworks
    PrepaymentBehavior, DefaultBehavior, RecoveryBehavior,
    PrepaymentModelFactory, DefaultModelFactory,
    MarketConditions, CreditFactors,
    // Waterfall engine
    WaterfallEngine,
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
        context: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<DatedFlows> {
        // Generate full tranche-specific cashflows with waterfall distribution
        self.generate_tranche_cashflows(context, as_of)
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
    /// Generate complete tranche-specific cashflows using waterfall engine
    fn generate_tranche_cashflows(
        &self,
        _context: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<DatedFlows> {
        use crate::instruments::common::structured_credit::{
            EnhancedCoverageTests, PaymentRecipient,
        };
        use std::collections::HashMap;

        let base_ccy = self.pool.base_currency();
        let mut pool_outstanding = self.pool.total_balance();
        
        if pool_outstanding.amount() <= 0.0 {
            return Ok(Vec::new());
        }

        // Track tranche balances over time
        let mut tranche_balances: HashMap<String, Money> = self.tranches
            .tranches
            .iter()
            .map(|t| (t.id.to_string(), t.current_balance))
            .collect();

        // Store all tranche cashflows by tranche ID
        let mut tranche_cashflow_map: HashMap<String, Vec<(Date, Money)>> = HashMap::new();
        for tranche in &self.tranches.tranches {
            tranche_cashflow_map.insert(tranche.id.to_string(), Vec::new());
        }

        // Initialize waterfall engine with standard CLO rules
        let mut waterfall_engine = self.create_waterfall_engine();
        
        // Initialize enhanced coverage tests
        let mut _coverage_tests = EnhancedCoverageTests {
            oc_tests: HashMap::new(),
            ic_tests: HashMap::new(),
            par_value_test: None,
            diversity_test: None,
            warf_test: None,
            was_test: None,
        };

        let months_per_period = self.payment_frequency.months().unwrap_or(3) as f64;
        let mut pay_date = self.first_payment_date.max(as_of);

        // Simulate period-by-period
        while pay_date <= self.legal_maturity && pool_outstanding.amount() > 100.0 {
            let seasoning_months = {
                let m = (pay_date.year() - self.closing_date.year()) * 12
                    + (pay_date.month() as i32 - self.closing_date.month() as i32);
                m.max(0) as u32
            };

            // Step 1: Calculate pool collections
            let wac = self.pool.weighted_avg_coupon();
            let period_rate = wac * (months_per_period / 12.0);
            let interest_collections = Money::new(pool_outstanding.amount() * period_rate, base_ccy);

            // Step 2: Apply prepayments and defaults
            let smm = self.premium_smm(pay_date, seasoning_months);
            let mdr = self.premium_mdr(pay_date, seasoning_months);
            
            let prepay_amt = Money::new(pool_outstanding.amount() * smm, base_ccy);
            let default_amt = Money::new(pool_outstanding.amount() * mdr, base_ccy);
            
            let recovery_rate = self.recovery_model.recovery_rate(
                pay_date,
                6,
                None,
                default_amt,
                &crate::instruments::common::structured_credit::MarketFactors::default(),
            );
            let recovery_amt = Money::new(default_amt.amount() * recovery_rate, base_ccy);

            // Total principal available = prepayments + recoveries + scheduled (0 for now)
            let scheduled_prin = Money::new(0.0, base_ccy);
            let total_principal = scheduled_prin
                .checked_add(prepay_amt)?
                .checked_add(recovery_amt)?;

            // Total cash available for distribution
            let total_cash = interest_collections.checked_add(total_principal)?;

            // Step 3: Run waterfall to distribute cash to tranches
            let waterfall_result = waterfall_engine.apply_waterfall(
                total_cash,
                pay_date,
                &self.tranches,
                pool_outstanding,
            )?;

            // Step 4: Record tranche-specific cashflows
            for tranche in &self.tranches.tranches {
                let tranche_id = tranche.id.to_string();
                
                // Get payment to this tranche from waterfall
                if let Some(payment) = waterfall_result.distributions.get(&PaymentRecipient::Tranche(tranche_id.clone())) {
                    if payment.amount() > 0.0 {
                        tranche_cashflow_map
                            .get_mut(&tranche_id)
                            .unwrap()
                            .push((pay_date, *payment));
                    }
                    
                    // Update tranche balance (assuming payments reduce balance)
                    let interest_portion = Money::new(
                        tranche_balances[&tranche_id].amount() * tranche.coupon.current_rate(pay_date) * (months_per_period / 12.0),
                        base_ccy
                    );
                    let principal_payment = payment.checked_sub(interest_portion).unwrap_or(Money::new(0.0, base_ccy));
                    
                    if let Some(current) = tranche_balances.get_mut(&tranche_id) {
                        *current = current.checked_sub(principal_payment).unwrap_or(*current);
                    }
                }
            }

            // Step 5: Update pool balance
            pool_outstanding = pool_outstanding
                .checked_sub(prepay_amt)?
                .checked_sub(default_amt)?;

            // Advance to next period
            pay_date = add_months(pay_date, self.payment_frequency.months().unwrap_or(3) as i32);
        }

        // Aggregate all tranche cashflows into single schedule
        // For now, sum across all tranches; in production would track separately
        let mut all_flows: DatedFlows = Vec::new();
        let mut flow_map: HashMap<Date, Money> = HashMap::new();
        
        for (_tranche_id, flows) in tranche_cashflow_map {
            for (date, amount) in flows {
                *flow_map.entry(date).or_insert(Money::new(0.0, base_ccy)) = 
                    flow_map[&date].checked_add(amount)?;
            }
        }
        
        for (date, amount) in flow_map {
            all_flows.push((date, amount));
        }
        all_flows.sort_by_key(|(d, _)| *d);

        Ok(all_flows)
    }

    /// Create waterfall engine with standard CLO payment rules
    fn create_waterfall_engine(&self) -> WaterfallEngine {
        use crate::instruments::common::structured_credit::{
            PaymentRule, PaymentRecipient, PaymentCalculation, ManagementFeeType,
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
            recipient: PaymentRecipient::Manager(ManagementFeeType::Senior),
            calculation: PaymentCalculation::PercentageOfCollateral {
                rate: 0.01,
                annual: true,
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
        
        // Add principal payments for each tranche
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
