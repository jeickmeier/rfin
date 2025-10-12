//! Unified waterfall engine for structured credit instruments.
//!
//! This module provides a comprehensive, flexible waterfall implementation for
//! distributing cashflows in CLOs, ABS, RMBS, CMBS and other structured products.

use crate::instruments::common::structured_credit::TrancheStructure;
use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::money::Money;
use finstack_core::Result;
use std::collections::HashMap;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use super::enums::PaymentMode;

/// Recipient of waterfall payments
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum PaymentRecipient {
    /// Service provider (trustee, admin, rating agency, etc.)
    ServiceProvider(String),
    /// Manager fee (type indicates senior/subordinated/incentive)
    ManagerFee(ManagementFeeType),
    /// Tranche payment
    Tranche(String),
    /// Reserve account funding
    ReserveAccount(String),
    /// Equity/residual distribution
    Equity,
}

/// Type of management fee
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ManagementFeeType {
    Senior,
    Subordinated,
    Incentive,
}

/// How to calculate payment amount
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum PaymentCalculation {
    /// Fixed amount
    FixedAmount { amount: Money },
    /// Percentage of collateral balance
    PercentageOfCollateral { rate: f64, annualized: bool },
    /// Interest due on tranche
    TrancheInterest { tranche_id: String },
    /// Principal payment to tranche
    TranchePrincipal {
        tranche_id: String,
        target_balance: Option<Money>,
    },
    /// Amount needed to cure coverage test breach
    CoverageTestCure {
        test_type: CoverageTestType,
        tranche_id: String,
    },
    /// All remaining cash
    ResidualCash,
    /// Fill reserve to target amount
    ReserveFill {
        reserve_id: String,
        target_amount: Money,
    },
}

/// Type of coverage test
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum CoverageTestType {
    /// Overcollateralization test
    OC,
    /// Interest coverage test
    IC,
    /// Par value test
    ParValue,
    /// Custom test
    Custom(String),
}

/// Payment rule in the waterfall
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct PaymentRule {
    /// Unique identifier
    pub id: String,
    /// Priority order (lower = higher priority)
    pub priority: u32,
    /// Who receives the payment
    pub recipient: PaymentRecipient,
    /// How to calculate amount
    pub calculation: PaymentCalculation,
    /// Whether payment can be diverted if coverage tests fail
    pub divertible: bool,
    /// Optional conditions for payment
    pub conditions: Vec<PaymentCondition>,
}

impl PaymentRule {
    /// Create a new payment rule
    pub fn new(
        id: impl Into<String>,
        priority: u32,
        recipient: PaymentRecipient,
        calculation: PaymentCalculation,
    ) -> Self {
        Self {
            id: id.into(),
            priority,
            recipient,
            calculation,
            divertible: false,
            conditions: Vec::new(),
        }
    }

    /// Mark as divertible if coverage tests fail
    pub fn divertible(mut self) -> Self {
        self.divertible = true;
        self
    }

    /// Add payment conditions
    pub fn with_conditions(mut self, conditions: Vec<PaymentCondition>) -> Self {
        self.conditions = conditions;
        self
    }
}

/// Conditions that must be met for payment
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum PaymentCondition {
    /// After a specific date
    AfterDate { date: Date },
    /// Before a specific date  
    BeforeDate { date: Date },
    /// Coverage test must be passing
    CoverageTestPassing {
        test_type: CoverageTestType,
        tranche_id: String,
    },
    /// In reinvestment period
    InReinvestmentPeriod,
    /// Not in reinvestment period
    NotInReinvestmentPeriod,
}

/// Trigger that can divert payments
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct DiversionTrigger {
    /// Unique identifier
    pub id: String,
    /// Test that triggers diversion
    pub test_type: CoverageTestType,
    /// Tranche where test applies
    pub tranche_id: String,
    /// Where to divert cash to (typically senior principal)
    pub divert_to: PaymentRecipient,
    /// Currently active
    pub active: bool,
}

/// Reserve account
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ReserveAccount {
    /// Account identifier
    pub id: String,
    /// Current balance
    pub balance: Money,
    /// Target balance
    pub target_balance: Money,
    /// Floor balance
    pub floor_balance: Money,
    /// Cap balance
    pub cap_balance: Option<Money>,
}

/// Result of waterfall distribution
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct WaterfallResult {
    /// Payment date
    pub payment_date: Date,
    /// Total available cash at start
    pub total_available: Money,
    /// Distributions by recipient
    pub distributions: HashMap<PaymentRecipient, Money>,
    /// Detailed payment records
    pub payment_records: Vec<PaymentRecord>,
    /// Remaining undistributed cash
    pub remaining_cash: Money,
    /// Updated reserve balances
    pub reserve_balances: HashMap<String, Money>,
    /// Whether any diversions occurred
    pub had_diversions: bool,
}

/// Record of individual payment
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct PaymentRecord {
    pub rule_id: String,
    pub priority: u32,
    pub recipient: PaymentRecipient,
    pub requested_amount: Money,
    pub paid_amount: Money,
    pub shortfall: Money,
    pub diverted: bool,
}

/// Main waterfall engine
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct WaterfallEngine {
    /// Ordered payment rules
    pub payment_rules: Vec<PaymentRule>,
    /// Diversion triggers for coverage test breaches
    pub diversion_triggers: Vec<DiversionTrigger>,
    /// Reserve accounts
    pub reserve_accounts: HashMap<String, ReserveAccount>,
    /// Base currency
    pub base_currency: Currency,
    /// Current payment mode
    pub payment_mode: PaymentMode,
}

impl WaterfallEngine {
    /// Create new waterfall engine
    pub fn new(base_currency: Currency) -> Self {
        Self {
            payment_rules: Vec::new(),
            diversion_triggers: Vec::new(),
            reserve_accounts: HashMap::new(),
            base_currency,
            payment_mode: PaymentMode::ProRata,
        }
    }

    /// Distribute principal pro-rata across all debt tranches
    fn distribute_prorata_principal(
        &self,
        principal_pot: Money,
        tranches: &TrancheStructure,
        distributions: &mut HashMap<PaymentRecipient, Money>,
        payment_records: &mut Vec<PaymentRecord>,
        payment_info: (&str, u32, bool), // (rule_id, priority, diverted)
    ) -> Result<Money> {
        let (rule_id, priority, diverted) = payment_info;
        // Total debt outstanding (exclude equity)
        let total_debt_balance = tranches
            .tranches
            .iter()
            .filter(|t| t.seniority != super::enums::TrancheSeniority::Equity)
            .try_fold(Money::new(0.0, self.base_currency), |acc, t| {
                acc.checked_add(t.current_balance)
            })?;

        if total_debt_balance.amount() == 0.0 || principal_pot.amount() == 0.0 {
            return Ok(Money::new(0.0, self.base_currency));
        }

        // Distribute to each debt tranche
        let mut total_paid = Money::new(0.0, self.base_currency);
        for t in &tranches.tranches {
            if t.seniority == super::enums::TrancheSeniority::Equity {
                continue;
            }
            
            let weight = t.current_balance.amount() / total_debt_balance.amount();
            let pay_amt = Money::new(principal_pot.amount() * weight, self.base_currency);
            let capped = if pay_amt.amount() <= t.current_balance.amount() {
                pay_amt
            } else {
                t.current_balance
            };

            if capped.amount() > 0.0 {
                let recip = PaymentRecipient::Tranche(t.id.to_string());
                use std::collections::hash_map::Entry;
                match distributions.entry(recip.to_owned()) {
                    Entry::Occupied(mut e) => {
                        let next = e.get().checked_add(capped)?;
                        e.insert(next);
                    }
                    Entry::Vacant(e) => {
                        e.insert(capped);
                    }
                }

                payment_records.push(PaymentRecord {
                    rule_id: rule_id.to_owned(),
                    priority,
                    recipient: recip,
                    requested_amount: capped,
                    paid_amount: capped,
                    shortfall: Money::new(0.0, self.base_currency),
                    diverted,
                });

                total_paid = total_paid.checked_add(capped)?;
            }
        }

        Ok(total_paid)
    }

    /// Add payment rule
    pub fn add_rule(mut self, rule: PaymentRule) -> Self {
        self.payment_rules.push(rule);
        self.payment_rules.sort_by_key(|r| r.priority);
        self
    }

    /// Add diversion trigger
    pub fn add_trigger(mut self, trigger: DiversionTrigger) -> Self {
        self.diversion_triggers.push(trigger);
        self
    }

    /// Add reserve account
    pub fn add_reserve(mut self, reserve: ReserveAccount) -> Self {
        self.reserve_accounts.insert(reserve.id.to_owned(), reserve);
        self
    }

    /// Set payment mode
    pub fn with_payment_mode(mut self, mode: PaymentMode) -> Self {
        self.payment_mode = mode;
        self
    }

    /// Apply waterfall to distribute available cash
    pub fn apply_waterfall(
        &mut self,
        available_cash: Money,
        payment_date: Date,
        tranches: &TrancheStructure,
        pool_balance: Money,
    ) -> Result<WaterfallResult> {
        let mut remaining = available_cash;
        let mut distributions: HashMap<PaymentRecipient, Money> =
            HashMap::with_capacity(self.payment_rules.len());
        let mut payment_records = Vec::with_capacity(self.payment_rules.len());
        let mut had_diversions = false;

        // Check which diversion triggers are active
        let mut active_triggers: Vec<String> = Vec::with_capacity(self.diversion_triggers.len());
        for t in &self.diversion_triggers {
            if t.active {
                active_triggers.push(t.id.to_owned());
            }
        }

        // Build tranche index for O(1) lookup by id
        let mut tranche_index: HashMap<&str, usize> =
            HashMap::with_capacity(tranches.tranches.len());
        for (i, t) in tranches.tranches.iter().enumerate() {
            tranche_index.insert(t.id.as_str(), i);
        }

        // Process payments in priority order
        let mut pro_rata_done = false;

        for rule in &self.payment_rules {
            if remaining.amount() <= 0.0 {
                break;
            }

            // Check conditions
            if !self.check_conditions(&rule.conditions, payment_date) {
                continue;
            }

            // Calculate payment amount
            let requested = self.calculate_payment_amount(
                &rule.calculation,
                remaining,
                tranches,
                &tranche_index,
                pool_balance,
                payment_date,
            )?;

            // Check for diversion
            let (recipient, diverted) = if rule.divertible && !active_triggers.is_empty() {
                // Divert to senior principal if triggers active
                if let Some(trigger) = self.diversion_triggers.iter().find(|t| t.active) {
                    had_diversions = true;
                    (trigger.divert_to.clone(), true)
                } else {
                    (rule.recipient.to_owned(), false)
                }
            } else {
                (rule.recipient.clone(), false)
            };

            // Handle pro-rata principal distributions in a single batch
            if self.payment_mode == PaymentMode::ProRata {
                if let PaymentCalculation::TranchePrincipal { .. } = &rule.calculation {
                    if !pro_rata_done {
                        pro_rata_done = true;
                        let total_paid = self.distribute_prorata_principal(
                            remaining,
                            tranches,
                            &mut distributions,
                            &mut payment_records,
                            (&rule.id, rule.priority, diverted),
                        )?;
                        remaining = remaining.checked_sub(total_paid)?;
                    }
                    // Skip sequential payment logic for any individual principal rules once batched
                    continue;
                }
            }

            // Make payment (sequential logic)
            let paid = if requested.amount() <= remaining.amount() {
                requested
            } else {
                remaining
            };
            let shortfall = requested
                .checked_sub(paid)
                .unwrap_or(Money::new(0.0, self.base_currency));

            use std::collections::hash_map::Entry;
            match distributions.entry(recipient.to_owned()) {
                Entry::Occupied(mut e) => {
                    let next = e.get().checked_add(paid)?;
                    e.insert(next);
                }
                Entry::Vacant(e) => {
                    e.insert(paid);
                }
            }

            payment_records.push(PaymentRecord {
                rule_id: rule.id.to_owned(),
                priority: rule.priority,
                recipient,
                requested_amount: requested,
                paid_amount: paid,
                shortfall,
                diverted,
            });

            remaining = remaining.checked_sub(paid)?;
        }

        // Update reserve balances
        let reserve_balances = self.update_reserve_balances(&distributions)?;

        Ok(WaterfallResult {
            payment_date,
            total_available: available_cash,
            distributions,
            payment_records,
            remaining_cash: remaining,
            reserve_balances,
            had_diversions,
        })
    }

    fn check_conditions(&self, conditions: &[PaymentCondition], date: Date) -> bool {
        for condition in conditions {
            match condition {
                PaymentCondition::AfterDate { date: after } => {
                    if date < *after {
                        return false;
                    }
                }
                PaymentCondition::BeforeDate { date: before } => {
                    if date > *before {
                        return false;
                    }
                }
                // Other conditions would need additional context
                _ => {}
            }
        }
        true
    }

    fn calculate_payment_amount(
        &self,
        calculation: &PaymentCalculation,
        available: Money,
        tranches: &TrancheStructure,
        tranche_index: &HashMap<&str, usize>,
        pool_balance: Money,
        payment_date: Date,
    ) -> Result<Money> {
        match calculation {
            PaymentCalculation::FixedAmount { amount } => Ok(*amount),

            PaymentCalculation::PercentageOfCollateral { rate, annualized } => {
                let period_rate = if *annualized {
                    rate / super::constants::QUARTERLY_PERIODS_PER_YEAR
                } else {
                    *rate
                };
                Ok(Money::new(
                    pool_balance.amount() * period_rate,
                    self.base_currency,
                ))
            }

            PaymentCalculation::TrancheInterest { tranche_id } => {
                if let Some(&idx) = tranche_index.get(tranche_id.as_str()) {
                    let tranche = &tranches.tranches[idx];
                    let rate = tranche.coupon.current_rate(payment_date);
                    let period_rate = rate / super::constants::QUARTERLY_PERIODS_PER_YEAR;
                    Ok(Money::new(
                        tranche.current_balance.amount() * period_rate,
                        self.base_currency,
                    ))
                } else {
                    Ok(Money::new(0.0, self.base_currency))
                }
            }

            PaymentCalculation::TranchePrincipal {
                tranche_id,
                target_balance,
            } => {
                if let Some(&idx) = tranche_index.get(tranche_id.as_str()) {
                    let tranche = &tranches.tranches[idx];
                    if let Some(target) = target_balance {
                        let payment = tranche
                            .current_balance
                            .checked_sub(*target)
                            .unwrap_or(Money::new(0.0, self.base_currency));
                        Ok(if payment.amount() <= available.amount() {
                            payment
                        } else {
                            available
                        })
                    } else {
                        Ok(if tranche.current_balance.amount() <= available.amount() {
                            tranche.current_balance
                        } else {
                            available
                        })
                    }
                } else {
                    Ok(Money::new(0.0, self.base_currency))
                }
            }

            PaymentCalculation::CoverageTestCure {
                test_type: _,
                tranche_id: _,
            } => {
                // Simplified: would need coverage test results
                Ok(Money::new(0.0, self.base_currency))
            }

            PaymentCalculation::ResidualCash => Ok(available),

            PaymentCalculation::ReserveFill {
                reserve_id,
                target_amount,
            } => {
                if let Some(reserve) = self.reserve_accounts.get(reserve_id) {
                    let needed = target_amount
                        .checked_sub(reserve.balance)
                        .unwrap_or(Money::new(0.0, self.base_currency));
                    Ok(if needed.amount() <= available.amount() {
                        needed
                    } else {
                        available
                    })
                } else {
                    Ok(Money::new(0.0, self.base_currency))
                }
            }
        }
    }

    fn update_reserve_balances(
        &mut self,
        distributions: &HashMap<PaymentRecipient, Money>,
    ) -> Result<HashMap<String, Money>> {
        let mut balances = HashMap::new();

        for (id, reserve) in &mut self.reserve_accounts {
            if let Some(amount) = distributions.get(&PaymentRecipient::ReserveAccount(id.clone())) {
                reserve.balance = reserve.balance.checked_add(*amount)?;
                if let Some(cap) = reserve.cap_balance {
                    reserve.balance = if reserve.balance.amount() <= cap.amount() {
                        reserve.balance
                    } else {
                        cap
                    };
                }
            }
            balances.insert(id.clone(), reserve.balance);
        }

        Ok(balances)
    }

    /// Create standard sequential waterfall with fees + tranche interest + tranche principal
    /// 
    /// This is the common pattern across CLO, ABS, CMBS, and RMBS instruments.
    /// Each instrument provides instrument-specific fees, then this method adds
    /// standard interest and principal payments in priority order.
    pub fn standard_sequential(
        base_currency: Currency,
        tranches: &TrancheStructure,
        fees: Vec<PaymentRule>,
    ) -> Self {
        let mut engine = Self::new(base_currency);
        let mut priority = 1;
        
        // Add fee rules
        for mut fee in fees {
            fee.priority = priority;
            engine.payment_rules.push(fee);
            priority += 1;
        }
        
        // Add interest payments for each tranche (in priority order)
        let mut sorted_tranches = tranches.tranches.clone();
        sorted_tranches.sort_by_key(|t| t.payment_priority);
        
        for tranche in &sorted_tranches {
            // Skip equity tranches for interest (they get residual cash)
            if tranche.seniority == super::enums::TrancheSeniority::Equity {
                continue;
            }
            
            engine.payment_rules.push(PaymentRule::new(
                format!("{}_interest", tranche.id.as_str()),
                priority,
                PaymentRecipient::Tranche(tranche.id.to_string()),
                PaymentCalculation::TrancheInterest {
                    tranche_id: tranche.id.to_string(),
                },
            ));
            priority += 1;
        }
        
        // Add principal payments for each debt tranche
        for tranche in &sorted_tranches {
            if tranche.seniority != super::enums::TrancheSeniority::Equity {
                engine.payment_rules.push(
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
            priority += 1;
        }
        
        // Add equity distribution (residual cash)
        engine.payment_rules.push(PaymentRule::new(
            "equity_distribution",
            priority,
            PaymentRecipient::Equity,
            PaymentCalculation::ResidualCash,
        ));
        
        engine.payment_mode = PaymentMode::ProRata;
        engine
    }

    /// Create standard CLO waterfall
    pub fn standard_clo(base_currency: Currency) -> Self {
        let mut engine = Self::new(base_currency);
        let mut priority = 1;

        // Senior expenses
        engine = engine.add_rule(PaymentRule::new(
            "senior_expenses",
            priority,
            PaymentRecipient::ServiceProvider("Trustee".into()),
            PaymentCalculation::FixedAmount {
                amount: Money::new(
                    50000.0 / super::constants::QUARTERLY_PERIODS_PER_YEAR,
                    base_currency,
                ),
            },
        ));
        priority += 1;

        // Senior management fee
        engine = engine.add_rule(PaymentRule::new(
            "senior_mgmt_fee",
            priority,
            PaymentRecipient::ManagerFee(ManagementFeeType::Senior),
            PaymentCalculation::PercentageOfCollateral {
                rate: 0.0015,
                annualized: true,
            },
        ));
        priority += 1;

        // Class A-E interest (would be generated from tranches)
        for class in ["A", "B", "C", "D", "E"] {
            let tranche_id = format!("CLASS_{}", class);
            engine = engine.add_rule(
                PaymentRule::new(
                    format!("{}_interest", tranche_id.to_lowercase()),
                    priority,
                    PaymentRecipient::Tranche(tranche_id.clone()),
                    PaymentCalculation::TrancheInterest { tranche_id },
                )
                .divertible(),
            );
            priority += 1;
        }

        // Subordinated management fee
        engine = engine.add_rule(PaymentRule::new(
            "sub_mgmt_fee",
            priority,
            PaymentRecipient::ManagerFee(ManagementFeeType::Subordinated),
            PaymentCalculation::PercentageOfCollateral {
                rate: 0.0005,
                annualized: true,
            },
        ));
        priority += 1;

        // Equity distribution
        engine = engine.add_rule(PaymentRule::new(
            "equity",
            priority,
            PaymentRecipient::Equity,
            PaymentCalculation::ResidualCash,
        ));

        engine
    }
}

/// Builder for waterfall engine
pub struct WaterfallBuilder {
    engine: WaterfallEngine,
    next_priority: u32,
}

impl WaterfallBuilder {
    /// Create new builder
    pub fn new(base_currency: Currency) -> Self {
        Self {
            engine: WaterfallEngine::new(base_currency),
            next_priority: 1,
        }
    }

    /// Add senior expenses
    pub fn add_senior_expenses(mut self, amount: Money, provider: &str) -> Self {
        self.engine = self.engine.add_rule(PaymentRule::new(
            format!("expense_{}", provider.to_lowercase()),
            self.next_priority,
            PaymentRecipient::ServiceProvider(provider.into()),
            PaymentCalculation::FixedAmount { amount },
        ));
        self.next_priority += 1;
        self
    }

    /// Add management fee
    pub fn add_management_fee(mut self, rate: f64, fee_type: ManagementFeeType) -> Self {
        let fee_name = match fee_type {
            ManagementFeeType::Senior => "senior",
            ManagementFeeType::Subordinated => "sub",
            ManagementFeeType::Incentive => "incentive",
        };

        self.engine = self.engine.add_rule(PaymentRule::new(
            format!("{}_mgmt_fee", fee_name),
            self.next_priority,
            PaymentRecipient::ManagerFee(fee_type),
            PaymentCalculation::PercentageOfCollateral {
                rate,
                annualized: true,
            },
        ));
        self.next_priority += 1;
        self
    }

    /// Add tranche interest payment
    pub fn add_tranche_interest(mut self, tranche_id: &str, divertible: bool) -> Self {
        let mut rule = PaymentRule::new(
            format!("{}_interest", tranche_id.to_lowercase()),
            self.next_priority,
            PaymentRecipient::Tranche(tranche_id.into()),
            PaymentCalculation::TrancheInterest {
                tranche_id: tranche_id.into(),
            },
        );

        if divertible {
            rule = rule.divertible();
        }

        self.engine = self.engine.add_rule(rule);
        self.next_priority += 1;
        self
    }

    /// Add tranche principal payment
    pub fn add_tranche_principal(mut self, tranche_id: &str) -> Self {
        self.engine = self.engine.add_rule(PaymentRule::new(
            format!("{}_principal", tranche_id.to_lowercase()),
            self.next_priority,
            PaymentRecipient::Tranche(tranche_id.into()),
            PaymentCalculation::TranchePrincipal {
                tranche_id: tranche_id.into(),
                target_balance: None,
            },
        ));
        self.next_priority += 1;
        self
    }

    /// Add equity distribution
    pub fn add_equity_distribution(mut self) -> Self {
        self.engine = self.engine.add_rule(PaymentRule::new(
            "equity_distribution",
            self.next_priority,
            PaymentRecipient::Equity,
            PaymentCalculation::ResidualCash,
        ));
        self.next_priority += 1;
        self
    }

    /// Add coverage test diversion trigger
    pub fn add_coverage_trigger(mut self, test_type: CoverageTestType, tranche_id: &str) -> Self {
        self.engine = self.engine.add_trigger(DiversionTrigger {
            id: format!(
                "{}_{}_trigger",
                tranche_id,
                match test_type {
                    CoverageTestType::OC => "oc",
                    CoverageTestType::IC => "ic",
                    _ => "test",
                }
            ),
            test_type,
            tranche_id: tranche_id.into(),
            divert_to: PaymentRecipient::Tranche("CLASS_A".to_string()), // Divert to senior
            active: false,
        });
        self
    }

    /// Add reserve account
    pub fn add_reserve_account(mut self, id: &str, target: Money, floor: Money) -> Self {
        let base_currency = self.engine.base_currency;
        self.engine = self.engine.add_reserve(ReserveAccount {
            id: id.into(),
            balance: Money::new(0.0, base_currency),
            target_balance: target,
            floor_balance: floor,
            cap_balance: None,
        });
        self
    }

    /// Build the waterfall engine
    pub fn build(self) -> WaterfallEngine {
        self.engine
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::currency::Currency;

    #[test]
    fn test_waterfall_builder() {
        let waterfall = WaterfallBuilder::new(Currency::USD)
            .add_senior_expenses(Money::new(25000.0, Currency::USD), "Trustee")
            .add_management_fee(0.004, ManagementFeeType::Senior)
            .add_tranche_interest("CLASS_A", true)
            .add_tranche_principal("CLASS_A")
            .add_equity_distribution()
            .build();

        assert_eq!(waterfall.payment_rules.len(), 5);
        assert_eq!(waterfall.payment_rules[0].priority, 1);
        assert_eq!(waterfall.payment_rules[4].priority, 5);
    }

    #[test]
    fn test_payment_priority_ordering() {
        let mut waterfall = WaterfallEngine::new(Currency::USD);

        waterfall = waterfall.add_rule(PaymentRule::new(
            "third",
            3,
            PaymentRecipient::Equity,
            PaymentCalculation::ResidualCash,
        ));
        waterfall = waterfall.add_rule(PaymentRule::new(
            "first",
            1,
            PaymentRecipient::Equity,
            PaymentCalculation::ResidualCash,
        ));
        waterfall = waterfall.add_rule(PaymentRule::new(
            "second",
            2,
            PaymentRecipient::Equity,
            PaymentCalculation::ResidualCash,
        ));

        assert_eq!(waterfall.payment_rules[0].id, "first");
        assert_eq!(waterfall.payment_rules[1].id, "second");
        assert_eq!(waterfall.payment_rules[2].id, "third");
    }
}
