//! Market-standard waterfall engine for CLOs and other structured products.
//!
//! This module implements the priority of payments (waterfall) logic used in
//! structured credit products, with support for coverage tests, diversions,
//! and reserve accounts.

use crate::instruments::common::structured_credit::TrancheStructure;
use crate::instruments::common::structured_credit::types_extended::TrancheId;
use finstack_core::dates::Date;
use finstack_core::money::Money;
use finstack_core::currency::Currency;
use std::collections::HashMap;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Main waterfall engine that processes cash distributions
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct WaterfallEngine {
    /// Ordered list of payment rules (priority order)
    pub payment_rules: Vec<PaymentRule>,
    /// Diversion triggers that can redirect cash flows
    pub diversion_triggers: Vec<DiversionTrigger>,
    /// Reserve accounts that hold cash
    pub reserve_accounts: HashMap<String, ReserveAccount>,
    /// Base currency for the waterfall
    pub base_currency: Currency,
}

/// Standard payment priorities in a CLO waterfall
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum PaymentPriority {
    /// Priority 1: Senior expenses (trustee, admin, rating agency)
    SeniorExpenses = 1,
    /// Priority 2: Senior management fee
    SeniorMgmtFee = 2,
    /// Priority 3: Class A (AAA) interest
    ClassAInterest = 3,
    /// Priority 4: Class A coverage test cure
    ClassACoverageCure = 4,
    /// Priority 5: Class B (AA) interest
    ClassBInterest = 5,
    /// Priority 6: Class B coverage test cure
    ClassBCoverageCure = 6,
    /// Priority 7: Class C (A) interest
    ClassCInterest = 7,
    /// Priority 8: Class C coverage test cure
    ClassCCoverageCure = 8,
    /// Priority 9: Class D (BBB) interest
    ClassDInterest = 9,
    /// Priority 10: Class D coverage test cure
    ClassDCoverageCure = 10,
    /// Priority 11: Class E (BB) interest
    ClassEInterest = 11,
    /// Priority 12: Class E coverage test cure
    ClassECoverageCure = 12,
    /// Priority 13: Subordinated management fee
    SubMgmtFee = 13,
    /// Priority 14: Other subordinated expenses
    SubExpenses = 14,
    /// Priority 15-20: Principal payments (in order of seniority)
    ClassAPrincipal = 15,
    ClassBPrincipal = 16,
    ClassCPrincipal = 17,
    ClassDPrincipal = 18,
    ClassEPrincipal = 19,
    /// Priority 21: Equity distribution
    EquityDistribution = 21,
}

/// A single payment rule in the waterfall
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct PaymentRule {
    /// Unique identifier for this payment
    pub id: String,
    /// Priority order (lower = higher priority)
    pub priority: u32,
    /// Who receives the payment
    pub recipient: PaymentRecipient,
    /// How to calculate the payment amount
    pub calculation: PaymentCalculation,
    /// Conditions that must be met for payment
    pub conditions: Vec<PaymentCondition>,
    /// Whether this payment can be diverted
    pub divertible: bool,
}

/// Recipients of waterfall payments
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum PaymentRecipient {
    /// Service provider (trustee, admin, etc.)
    ServiceProvider(String),
    /// Manager fee
    Manager(ManagementFeeType),
    /// Tranche holder
    Tranche(TrancheId),
    /// Reserve account
    ReserveAccount(String),
    /// Equity/residual holder
    Equity,
}

/// Types of management fees
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum ManagementFeeType {
    Senior,
    Subordinated,
    Incentive,
}

/// How to calculate payment amounts
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case", tag = "type"))]
pub enum PaymentCalculation {
    /// Fixed amount
    FixedAmount { amount: Money },
    /// Percentage of collateral
    PercentageOfCollateral { rate: f64, annual: bool },
    /// Interest due on tranche
    TrancheInterest { tranche_id: TrancheId },
    /// Principal due on tranche
    TranchePrincipal { 
        tranche_id: TrancheId,
        target_balance: Option<Money>,
    },
    /// Amount needed to cure coverage test
    CoverageTestCure { 
        test_type: CoverageTestType,
        tranche_id: TrancheId,
    },
    /// All remaining cash
    ResidualCash,
    /// Amount to fill reserve to target
    ReserveFill { 
        reserve_id: String,
        target_amount: Money,
    },
}

/// Conditions for payments
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case", tag = "type"))]
pub enum PaymentCondition {
    /// After a specific date
    AfterDate { date: Date },
    /// Before a specific date
    BeforeDate { date: Date },
    /// Coverage test must be passing
    CoverageTestPassing { 
        test_type: CoverageTestType,
        tranche_id: TrancheId,
    },
    /// Coverage test must be failing
    CoverageTestFailing {
        test_type: CoverageTestType,
        tranche_id: TrancheId,
    },
    /// In reinvestment period
    InReinvestmentPeriod,
    /// Not in reinvestment period
    NotInReinvestmentPeriod,
    /// Reserve account has minimum
    ReserveMinimum {
        reserve_id: String,
        minimum: Money,
    },
}

/// Types of coverage tests
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum CoverageTestType {
    /// Overcollateralization test
    OC,
    /// Interest coverage test
    IC,
    /// Par value test
    ParValue,
}

/// Diversion trigger that redirects cash flow
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct DiversionTrigger {
    /// Unique identifier
    pub id: String,
    /// Test that triggers diversion
    pub trigger_test: CoverageTestType,
    /// Tranche level where test applies
    pub tranche_id: TrancheId,
    /// Where to divert cash from
    pub divert_from: PaymentPriority,
    /// Where to divert cash to
    pub divert_to: PaymentRecipient,
    /// Active/inactive flag
    pub active: bool,
}

/// Reserve account for holding cash
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ReserveAccount {
    /// Account identifier
    pub id: String,
    /// Current balance
    pub balance: Money,
    /// Target balance
    pub target_balance: Money,
    /// Floor (minimum) balance
    pub floor_balance: Money,
    /// Cap (maximum) balance
    pub cap_balance: Option<Money>,
}

/// Results from applying the waterfall
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct WaterfallResult {
    /// Distributions made to each recipient
    pub distributions: HashMap<PaymentRecipient, Money>,
    /// Detailed payment records in order
    pub payment_details: Vec<PaymentRecord>,
    /// Triggers that were breached
    pub triggers_breached: Vec<String>,
    /// Remaining undistributed cash
    pub remaining_cash: Money,
    /// Updated reserve balances
    pub reserve_balances: HashMap<String, Money>,
    /// Coverage ratios after distributions
    pub coverage_ratios: CoverageRatios,
}

/// Record of a single payment
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct PaymentRecord {
    /// Payment rule ID
    pub rule_id: String,
    /// Priority order
    pub priority: u32,
    /// Recipient
    pub recipient: PaymentRecipient,
    /// Amount requested
    pub requested_amount: Money,
    /// Amount actually paid
    pub paid_amount: Money,
    /// Shortfall if any
    pub shortfall: Money,
    /// Whether payment was diverted
    pub diverted: bool,
}

/// Coverage ratios calculated during waterfall
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CoverageRatios {
    /// OC ratios by tranche
    pub oc_ratios: HashMap<TrancheId, f64>,
    /// IC ratios by tranche  
    pub ic_ratios: HashMap<TrancheId, f64>,
    /// OC tests passing/failing
    pub oc_tests: HashMap<TrancheId, bool>,
    /// IC tests passing/failing
    pub ic_tests: HashMap<TrancheId, bool>,
    /// Par value ratio
    pub par_value_ratio: Option<f64>,
}

impl WaterfallEngine {
    /// Create a new waterfall engine
    pub fn new(base_currency: Currency) -> Self {
        Self {
            payment_rules: Vec::new(),
            diversion_triggers: Vec::new(),
            reserve_accounts: HashMap::new(),
            base_currency,
        }
    }

    /// Add a payment rule
    pub fn add_rule(mut self, rule: PaymentRule) -> Self {
        self.payment_rules.push(rule);
        // Keep sorted by priority
        self.payment_rules.sort_by_key(|r| r.priority);
        self
    }

    /// Add a diversion trigger
    pub fn add_trigger(mut self, trigger: DiversionTrigger) -> Self {
        self.diversion_triggers.push(trigger);
        self
    }

    /// Add a reserve account
    pub fn add_reserve(mut self, reserve: ReserveAccount) -> Self {
        self.reserve_accounts.insert(reserve.id.clone(), reserve);
        self
    }

    /// Apply the waterfall to available cash
    pub fn apply_waterfall(
        &mut self,
        available_cash: Money,
        period_date: Date,
        tranches: &TrancheStructure,
        pool_balance: Money,
    ) -> finstack_core::Result<WaterfallResult> {
        // Validate currency
        if available_cash.currency() != self.base_currency {
            return Err(finstack_core::Error::CurrencyMismatch {
                expected: self.base_currency,
                actual: available_cash.currency(),
            });
        }

        let mut remaining = available_cash;
        let mut distributions = HashMap::new();
        let mut payment_details = Vec::new();
        let mut triggers_breached = Vec::new();

        // Calculate coverage ratios before distributions
        let coverage_ratios = self.calculate_coverage_ratios(tranches, pool_balance)?;

        // Check for breached triggers
        for trigger in &self.diversion_triggers {
            if trigger.active && self.is_trigger_breached(trigger, &coverage_ratios) {
                triggers_breached.push(trigger.id.clone());
            }
        }

        // Process payments in priority order
        for rule in &self.payment_rules {
            // Check conditions
            if !self.check_conditions(&rule.conditions, period_date, &coverage_ratios) {
                continue;
            }

            // Check if payment should be diverted
            let diverted = rule.divertible && self.should_divert(rule, &triggers_breached);
            
            // Calculate payment amount
            let requested = self.calculate_payment_amount(
                &rule.calculation,
                remaining,
                tranches,
                pool_balance,
                &coverage_ratios,
            )?;

            // Determine actual payment (limited by available cash)
            let paid = if requested.amount() <= remaining.amount() {
                requested
            } else {
                remaining
            };

            // Determine recipient (may be diverted)
            let actual_recipient = if diverted {
                self.get_diversion_recipient(&rule.recipient, &triggers_breached)
                    .unwrap_or_else(|| rule.recipient.clone())
            } else {
                rule.recipient.clone()
            };

            // Make payment
            if paid.amount() > 0.0 {
                *distributions.entry(actual_recipient.clone()).or_insert(
                    Money::new(0.0, self.base_currency)
                ) = distributions.get(&actual_recipient)
                    .unwrap_or(&Money::new(0.0, self.base_currency))
                    .checked_add(paid)?;
                
                remaining = remaining.checked_sub(paid)?;
            }

            // Record payment details
            payment_details.push(PaymentRecord {
                rule_id: rule.id.clone(),
                priority: rule.priority,
                recipient: actual_recipient,
                requested_amount: requested,
                paid_amount: paid,
                shortfall: requested.checked_sub(paid).unwrap_or(
                    Money::new(0.0, self.base_currency)
                ),
                diverted,
            });
        }

        // Update reserve account balances
        let reserve_balances = self.update_reserve_balances(&distributions)?;

        Ok(WaterfallResult {
            distributions,
            payment_details,
            triggers_breached,
            remaining_cash: remaining,
            reserve_balances,
            coverage_ratios,
        })
    }

    /// Calculate coverage ratios
    fn calculate_coverage_ratios(
        &self,
        tranches: &TrancheStructure,
        pool_balance: Money,
    ) -> finstack_core::Result<CoverageRatios> {
        let mut oc_ratios = HashMap::new();
        let mut ic_ratios = HashMap::new();
        let mut oc_tests = HashMap::new();
        let mut ic_tests = HashMap::new();

        for tranche in &tranches.tranches {
            // Calculate OC ratio: (Pool Balance) / (Tranche Balance + Senior Tranches)
            let tranche_and_senior = self.calculate_tranche_and_senior_balance(
                tranche.id.as_ref(),
                tranches,
            )?;
            
            let oc_ratio = if tranche_and_senior.amount() > 0.0 {
                pool_balance.amount() / tranche_and_senior.amount()
            } else {
                f64::INFINITY
            };
            
            oc_ratios.insert(tranche.id.to_string(), oc_ratio);
            
            // Check against required OC level
            let oc_required = tranche.oc_trigger
                .as_ref()
                .map(|t| t.trigger_level)
                .unwrap_or(1.0);
            oc_tests.insert(tranche.id.to_string(), oc_ratio >= oc_required);

            // IC ratio calculation would require interest amounts
            // For now, using placeholder
            ic_ratios.insert(tranche.id.to_string(), 1.5);
            ic_tests.insert(tranche.id.to_string(), true);
        }

        Ok(CoverageRatios {
            oc_ratios,
            ic_ratios,
            oc_tests,
            ic_tests,
            par_value_ratio: Some(1.0), // Placeholder
        })
    }

    /// Calculate tranche balance plus all senior tranches
    fn calculate_tranche_and_senior_balance(
        &self,
        tranche_id: &str,
        tranches: &TrancheStructure,
    ) -> finstack_core::Result<Money> {
        let mut total = Money::new(0.0, self.base_currency);
        
        for tranche in &tranches.tranches {
            total = total.checked_add(tranche.current_balance)?;
            if tranche.id.as_str() == tranche_id {
                break;
            }
        }
        
        Ok(total)
    }

    /// Check if payment conditions are met
    fn check_conditions(
        &self,
        conditions: &[PaymentCondition],
        period_date: Date,
        coverage_ratios: &CoverageRatios,
    ) -> bool {
        for condition in conditions {
            match condition {
                PaymentCondition::AfterDate { date } => {
                    if period_date < *date {
                        return false;
                    }
                }
                PaymentCondition::BeforeDate { date } => {
                    if period_date > *date {
                        return false;
                    }
                }
                PaymentCondition::CoverageTestPassing { test_type, tranche_id } => {
                    let passing = match test_type {
                        CoverageTestType::OC => {
                            coverage_ratios.oc_tests.get(tranche_id).copied().unwrap_or(false)
                        }
                        CoverageTestType::IC => {
                            coverage_ratios.ic_tests.get(tranche_id).copied().unwrap_or(false)
                        }
                        _ => true,
                    };
                    if !passing {
                        return false;
                    }
                }
                PaymentCondition::CoverageTestFailing { test_type, tranche_id } => {
                    let failing = match test_type {
                        CoverageTestType::OC => {
                            !coverage_ratios.oc_tests.get(tranche_id).copied().unwrap_or(true)
                        }
                        CoverageTestType::IC => {
                            !coverage_ratios.ic_tests.get(tranche_id).copied().unwrap_or(true)
                        }
                        _ => false,
                    };
                    if !failing {
                        return false;
                    }
                }
                // Other conditions would be implemented similarly
                _ => {}
            }
        }
        true
    }

    /// Calculate payment amount based on calculation method
    fn calculate_payment_amount(
        &self,
        calculation: &PaymentCalculation,
        available_cash: Money,
        tranches: &TrancheStructure,
        pool_balance: Money,
        _coverage_ratios: &CoverageRatios,
    ) -> finstack_core::Result<Money> {
        match calculation {
            PaymentCalculation::FixedAmount { amount } => Ok(*amount),
            
            PaymentCalculation::PercentageOfCollateral { rate, annual } => {
                let period_rate = if *annual { rate / 12.0 } else { *rate };
                Ok(Money::new(pool_balance.amount() * period_rate, self.base_currency))
            }
            
            PaymentCalculation::TrancheInterest { tranche_id } => {
                // Find tranche and calculate interest
                let tranche = tranches.tranches.iter()
                    .find(|t| t.id.as_str() == tranche_id)
                    .ok_or_else(|| finstack_core::Error::Input(
                        finstack_core::error::InputError::NotFound { 
                            id: tranche_id.to_string() 
                        }
                    ))?;
                
                // Monthly interest = annual rate / 12 * balance
                let current_date = finstack_core::dates::Date::from_calendar_date(2025, time::Month::January, 1)
                    .unwrap_or(finstack_core::dates::Date::MIN);
                let monthly_rate = tranche.coupon.current_rate(current_date) / 12.0;
                Ok(Money::new(
                    tranche.current_balance.amount() * monthly_rate,
                    self.base_currency
                ))
            }
            
            PaymentCalculation::TranchePrincipal { tranche_id, target_balance } => {
                let tranche = tranches.tranches.iter()
                    .find(|t| t.id.as_str() == tranche_id)
                    .ok_or_else(|| finstack_core::Error::Input(
                        finstack_core::error::InputError::NotFound { 
                            id: tranche_id.to_string() 
                        }
                    ))?;
                
                let target = target_balance.unwrap_or(Money::new(0.0, self.base_currency));
                let payment = if tranche.current_balance.amount() > target.amount() {
                    tranche.current_balance.checked_sub(target)?
                } else {
                    Money::new(0.0, self.base_currency)
                };
                
                Ok(payment)
            }
            
            PaymentCalculation::ResidualCash => Ok(available_cash),
            
            _ => Ok(Money::new(0.0, self.base_currency)), // Other calculations TBD
        }
    }

    /// Check if a trigger is breached
    fn is_trigger_breached(
        &self,
        trigger: &DiversionTrigger,
        coverage_ratios: &CoverageRatios,
    ) -> bool {
        match trigger.trigger_test {
            CoverageTestType::OC => {
                !coverage_ratios.oc_tests.get(&trigger.tranche_id)
                    .copied()
                    .unwrap_or(true)
            }
            CoverageTestType::IC => {
                !coverage_ratios.ic_tests.get(&trigger.tranche_id)
                    .copied()
                    .unwrap_or(true)
            }
            _ => false,
        }
    }

    /// Check if payment should be diverted
    fn should_divert(&self, rule: &PaymentRule, breached_triggers: &[String]) -> bool {
        rule.divertible && !breached_triggers.is_empty()
    }

    /// Get diversion recipient
    fn get_diversion_recipient(
        &self,
        _original: &PaymentRecipient,
        breached_triggers: &[String],
    ) -> Option<PaymentRecipient> {
        // Find first active diversion trigger
        for trigger_id in breached_triggers {
            if let Some(trigger) = self.diversion_triggers.iter()
                .find(|t| t.id == *trigger_id) {
                return Some(trigger.divert_to.clone());
            }
        }
        None
    }

    /// Update reserve account balances
    fn update_reserve_balances(
        &mut self,
        distributions: &HashMap<PaymentRecipient, Money>,
    ) -> finstack_core::Result<HashMap<String, Money>> {
        let mut balances = HashMap::new();
        
        for (recipient, amount) in distributions {
            if let PaymentRecipient::ReserveAccount(reserve_id) = recipient {
                if let Some(reserve) = self.reserve_accounts.get_mut(reserve_id) {
                    reserve.balance = reserve.balance.checked_add(*amount)?;
                    
                    // Apply cap if exists
                    if let Some(cap) = reserve.cap_balance {
                        if reserve.balance.amount() > cap.amount() {
                            reserve.balance = cap;
                        }
                    }
                    
                    balances.insert(reserve_id.clone(), reserve.balance);
                }
            }
        }
        
        Ok(balances)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_waterfall() -> WaterfallEngine {
        let mut engine = WaterfallEngine::new(Currency::USD);
        
        // Add standard CLO payment rules
        engine = engine
            .add_rule(PaymentRule {
                id: "senior_expenses".to_string(),
                priority: 1,
                recipient: PaymentRecipient::ServiceProvider("Trustee".to_string()),
                calculation: PaymentCalculation::FixedAmount {
                    amount: Money::new(50_000.0, Currency::USD),
                },
                conditions: vec![],
                divertible: false,
            })
            .add_rule(PaymentRule {
                id: "senior_mgmt_fee".to_string(),
                priority: 2,
                recipient: PaymentRecipient::Manager(ManagementFeeType::Senior),
                calculation: PaymentCalculation::PercentageOfCollateral {
                    rate: 0.01,
                    annual: true,
                },
                conditions: vec![],
                divertible: false,
            })
            .add_rule(PaymentRule {
                id: "class_a_interest".to_string(),
                priority: 3,
                recipient: PaymentRecipient::Tranche("CLASS_A".into()),
                calculation: PaymentCalculation::TrancheInterest {
                    tranche_id: "CLASS_A".into(),
                },
                conditions: vec![],
                divertible: false,
            });
        
        engine
    }

    #[test]
    fn test_waterfall_creation() {
        let waterfall = create_test_waterfall();
        assert_eq!(waterfall.payment_rules.len(), 3);
        assert_eq!(waterfall.base_currency, Currency::USD);
    }

    #[test]
    fn test_payment_priority_ordering() {
        let waterfall = create_test_waterfall();
        
        // Verify rules are sorted by priority
        for i in 1..waterfall.payment_rules.len() {
            assert!(
                waterfall.payment_rules[i].priority >= waterfall.payment_rules[i - 1].priority
            );
        }
    }
}
