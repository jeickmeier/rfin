//! Simplified waterfall engine for structured credit instruments.
//!
//! This module provides a streamlined sequential waterfall for pricing flows:
//! fees → tranche interest → tranche principal → equity residual.
//! Supports simple OC/IC diversion triggers for coverage test breaches.

use super::coverage_tests::{CoverageTest, TestContext};
use super::AssetPool;
use super::TrancheStructure;
use crate::instruments::structured_credit::config::QUARTERLY_PERIODS_PER_YEAR;
use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::explain::{ExplainOpts, ExplanationTrace, TraceEntry};
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::Result;
use std::collections::HashMap;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

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
    /// Equity/residual distribution
    Equity,
}

/// Type of management fee
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ManagementFeeType {
    /// Senior variant.
    Senior,
    /// Subordinated variant.
    Subordinated,
    /// Incentive variant.
    Incentive,
}

/// How to calculate payment amount
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum PaymentCalculation {
    /// Fixed amount
    FixedAmount {
        /// Amount.
        amount: Money,
    },
    /// Percentage of collateral balance
    PercentageOfCollateral {
        /// Rate.
        rate: f64,
        /// Annualized.
        annualized: bool,
    },
    /// Interest due on tranche
    TrancheInterest {
        /// Tranche id.
        tranche_id: String,
    },
    /// Principal payment to tranche
    TranchePrincipal {
        /// Tranche id.
        tranche_id: String,
        /// Target balance.
        target_balance: Option<Money>,
    },
    /// All remaining cash
    ResidualCash,
}

/// Type of coverage test (simplified to OC/IC only)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum CoverageTestType {
    /// Overcollateralization test
    OC,
    /// Interest coverage test
    IC,
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
        }
    }

    /// Mark as divertible if coverage tests fail
    pub fn divertible(mut self) -> Self {
        self.divertible = true;
        self
    }
}

/// Simple OC/IC trigger for diversion
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CoverageTrigger {
    /// Tranche where test applies
    pub tranche_id: String,
    /// OC trigger level (e.g., 1.15 = 115%)
    pub oc_trigger: Option<f64>,
    /// IC trigger level (e.g., 1.10 = 110%)
    pub ic_trigger: Option<f64>,
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
    /// Whether any diversions occurred
    pub had_diversions: bool,
    /// Diversion reason if applicable
    pub diversion_reason: Option<String>,
    /// Optional explanation trace (enabled via ExplainOpts)
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none")
    )]
    pub explanation: Option<ExplanationTrace>,
}

/// Record of individual payment
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct PaymentRecord {
    /// rule id.
    pub rule_id: String,
    /// priority.
    pub priority: u32,
    /// recipient.
    pub recipient: PaymentRecipient,
    /// requested amount.
    pub requested_amount: Money,
    /// paid amount.
    pub paid_amount: Money,
    /// shortfall.
    pub shortfall: Money,
    /// diverted.
    pub diverted: bool,
}

/// Main waterfall engine (simplified sequential)
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct WaterfallEngine {
    /// Ordered payment rules
    pub payment_rules: Vec<PaymentRule>,
    /// Coverage triggers for OC/IC diversion
    pub coverage_triggers: Vec<CoverageTrigger>,
    /// Base currency
    pub base_currency: Currency,
}

impl WaterfallEngine {
    /// Create new waterfall engine
    pub fn new(base_currency: Currency) -> Self {
        Self {
            payment_rules: Vec::new(),
            coverage_triggers: Vec::new(),
            base_currency,
        }
    }

    /// Add payment rule
    pub fn add_rule(mut self, rule: PaymentRule) -> Self {
        self.payment_rules.push(rule);
        self.payment_rules.sort_by_key(|r| r.priority);
        self
    }

    /// Add coverage trigger for OC/IC diversion
    pub fn add_coverage_trigger(mut self, trigger: CoverageTrigger) -> Self {
        self.coverage_triggers.push(trigger);
        self
    }

    /// Apply waterfall to distribute available cash
    ///
    /// Sequential waterfall: processes payment rules in priority order.
    /// If OC/IC triggers are configured and breached, diverts equity/junior principal
    /// to senior tranches until tests pass.
    #[allow(clippy::too_many_arguments)]
    pub fn apply_waterfall(
        &mut self,
        available_cash: Money,
        interest_collections: Money,
        payment_date: Date,
        tranches: &TrancheStructure,
        pool_balance: Money,
        _pool: &AssetPool,
        market: &MarketContext,
    ) -> Result<WaterfallResult> {
        self.apply_waterfall_with_explanation(
            available_cash,
            interest_collections,
            payment_date,
            tranches,
            pool_balance,
            _pool,
            market,
            ExplainOpts::disabled(),
        )
    }

    /// Apply waterfall with optional explanation trace.
    ///
    /// Returns waterfall result with optional trace containing
    /// step-by-step payment allocations when explanation is enabled.
    #[allow(clippy::too_many_arguments)]
    pub fn apply_waterfall_with_explanation(
        &mut self,
        available_cash: Money,
        interest_collections: Money,
        payment_date: Date,
        tranches: &TrancheStructure,
        pool_balance: Money,
        _pool: &AssetPool,
        market: &MarketContext,
        explain: ExplainOpts,
    ) -> Result<WaterfallResult> {
        let mut remaining = available_cash;
        let mut distributions: HashMap<PaymentRecipient, Money> =
            HashMap::with_capacity(self.payment_rules.len());
        let mut payment_records = Vec::with_capacity(self.payment_rules.len());
        let mut had_diversions = false;
        let mut diversion_reason = None;

        // Initialize explanation trace if requested
        let mut trace = if explain.enabled {
            Some(ExplanationTrace::new("waterfall"))
        } else {
            None
        };

        // Build tranche index for O(1) lookup by id
        let mut tranche_index: HashMap<&str, usize> =
            HashMap::with_capacity(tranches.tranches.len());
        for (i, t) in tranches.tranches.iter().enumerate() {
            tranche_index.insert(t.id.as_str(), i);
        }

        // Check OC/IC triggers (simplified: assumes we can compute on the fly)
        // For now, set triggers as inactive; in production, caller would check tests and set active
        let diversion_active = self.check_diversion_triggers_active(
            tranches,
            _pool,
            payment_date,
            available_cash,
            interest_collections,
        )?;
        if diversion_active {
            had_diversions = true;
            diversion_reason = Some("OC or IC breached".to_string());
        }

        // Process payments in priority order (sequential)
        for rule in &self.payment_rules {
            if remaining.amount() <= 0.0 {
                break;
            }

            // Calculate payment amount
            let requested = self.calculate_payment_amount(
                &rule.calculation,
                remaining,
                tranches,
                &tranche_index,
                pool_balance,
                payment_date,
                market,
            )?;

            // Check for diversion: if divertible and triggers active, skip payment
            let (recipient, diverted) = if rule.divertible && diversion_active {
                // Divert to first senior tranche (priority 1)
                let senior_tranche = tranches
                    .tranches
                    .iter()
                    .min_by_key(|t| t.payment_priority)
                    .map(|t| PaymentRecipient::Tranche(t.id.to_string()))
                    .unwrap_or_else(|| rule.recipient.clone());
                (senior_tranche, true)
            } else {
                (rule.recipient.clone(), false)
            };

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
            match distributions.entry(recipient.clone()) {
                Entry::Occupied(mut e) => {
                    let next = e.get().checked_add(paid)?;
                    e.insert(next);
                }
                Entry::Vacant(e) => {
                    e.insert(paid);
                }
            }

            payment_records.push(PaymentRecord {
                rule_id: rule.id.clone(),
                priority: rule.priority,
                recipient: recipient.clone(),
                requested_amount: requested,
                paid_amount: paid,
                shortfall,
                diverted,
            });

            // Add trace entry if explanation is enabled
            if let Some(ref mut t) = trace {
                let step_name = format!("{} - {:?}", rule.id, recipient);
                t.push(
                    TraceEntry::WaterfallStep {
                        period: 0, // Single period waterfall
                        step_name,
                        cash_in_amount: requested.amount(),
                        cash_in_currency: requested.currency().to_string(),
                        cash_out_amount: paid.amount(),
                        cash_out_currency: paid.currency().to_string(),
                        shortfall_amount: if shortfall.amount() > 0.0 {
                            Some(shortfall.amount())
                        } else {
                            None
                        },
                        shortfall_currency: if shortfall.amount() > 0.0 {
                            Some(shortfall.currency().to_string())
                        } else {
                            None
                        },
                    },
                    explain.max_entries,
                );
            }

            remaining = remaining.checked_sub(paid)?;
        }

        Ok(WaterfallResult {
            payment_date,
            total_available: available_cash,
            distributions,
            payment_records,
            remaining_cash: remaining,
            had_diversions,
            diversion_reason,
            explanation: trace,
        })
    }

    /// Check if any OC/IC triggers are breached
    ///
    /// Computes OC and IC ratios for each configured trigger and returns true
    /// if any test is below its threshold.
    fn check_diversion_triggers_active(
        &self,
        tranches: &TrancheStructure,
        pool: &AssetPool,
        as_of: Date,
        available_cash: Money,
        interest_collections: Money,
    ) -> Result<bool> {
        if self.coverage_triggers.is_empty() {
            return Ok(false);
        }

        for trigger in &self.coverage_triggers {
            // Find the tranche
            let _ = tranches
                .tranches
                .iter()
                .find(|t| t.id.as_str() == trigger.tranche_id)
                .ok_or_else(|| {
                    finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                        id: format!("tranche:{}", trigger.tranche_id),
                    })
                })?;

            // Calculate cumulative senior balance (all tranches with higher priority)
            let _ = tranches.senior_balance(&trigger.tranche_id);

            if let Some(oc_trigger_level) = trigger.oc_trigger {
                // Reuse unified coverage test logic with performing balance and cash
                let ctx = TestContext {
                    pool,
                    tranches,
                    tranche_id: &trigger.tranche_id,
                    as_of,
                    cash_balance: available_cash,
                    interest_collections,
                };

                let oc_test = CoverageTest::new_oc(oc_trigger_level);
                let result = oc_test.calculate(&ctx);
                if !result.is_passing {
                    return Ok(true);
                }
            }

            if let Some(ic_trigger_level) = trigger.ic_trigger {
                let ctx = TestContext {
                    pool,
                    tranches,
                    tranche_id: &trigger.tranche_id,
                    as_of,
                    cash_balance: available_cash, // Not used by IC, but required by context
                    interest_collections,
                };

                let ic_test = CoverageTest::new_ic(ic_trigger_level);
                let result = ic_test.calculate(&ctx);
                if !result.is_passing {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    #[allow(clippy::too_many_arguments)]
    fn calculate_payment_amount(
        &self,
        calculation: &PaymentCalculation,
        available: Money,
        tranches: &TrancheStructure,
        tranche_index: &HashMap<&str, usize>,
        pool_balance: Money,
        payment_date: Date,
        market: &MarketContext,
    ) -> Result<Money> {
        match calculation {
            PaymentCalculation::FixedAmount { amount } => Ok(*amount),

            PaymentCalculation::PercentageOfCollateral { rate, annualized } => {
                let period_rate = if *annualized {
                    rate / QUARTERLY_PERIODS_PER_YEAR
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
                    let rate = tranche.coupon.current_rate_with_index(payment_date, market);
                    let period_rate = rate / QUARTERLY_PERIODS_PER_YEAR;
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

            PaymentCalculation::ResidualCash => Ok(available),
        }
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

    /// Add OC/IC coverage trigger for diversion
    pub fn add_oc_ic_trigger(
        mut self,
        tranche_id: &str,
        oc_trigger: Option<f64>,
        ic_trigger: Option<f64>,
    ) -> Self {
        self.engine = self.engine.add_coverage_trigger(CoverageTrigger {
            tranche_id: tranche_id.into(),
            oc_trigger,
            ic_trigger,
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
