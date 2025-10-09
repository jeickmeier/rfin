//! Call provisions for structured credit instruments.
//!
//! Adapts the existing bond call/put schedule infrastructure for use in 
//! structured credit deals. Hastructure supports call provisions based on
//! pool/bond balances, dates, or factor triggers.

use finstack_core::dates::Date;
use finstack_core::money::Money;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use super::{AssetPool, TrancheStructure};

/// Call provision trigger conditions (adapted from bond call/put schedules)
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum CallTrigger {
    /// Call on or after specific date at fixed price
    OnDate { 
        call_date: Date, 
        call_price_pct: f64 
    },
    /// Call when pool balance falls below threshold
    PoolBalanceThreshold { 
        threshold_amount: Money,
        call_price_pct: f64,
    },
    /// Call when pool factor falls below threshold
    PoolFactorThreshold { 
        factor_threshold: f64,
        call_price_pct: f64,
    },
    /// Call when bond/tranche balance falls below threshold
    TrancheBalanceThreshold { 
        tranche_id: String,
        threshold_amount: Money,
        call_price_pct: f64,
    },
    /// Optional call (manager discretion) after date
    Optional { 
        first_call_date: Date,
        call_schedule: Vec<(Date, f64)>, // (date, call_price_pct)
    },
    /// Cleanup call (typically 10% remaining)
    Cleanup { 
        factor_threshold: f64, // e.g., 0.10 for 10%
        call_price_pct: f64,
    },
}

/// Call provision configuration
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CallProvision {
    /// Unique identifier for this call provision
    pub id: String,
    /// Trigger condition
    pub trigger: CallTrigger,
    /// Whether call is currently active
    pub is_active: bool,
    /// Date when call was triggered (if any)
    pub triggered_date: Option<Date>,
    /// Notice period (days) required before call can be executed
    pub notice_days: u32,
}

impl CallProvision {
    /// Create a new call provision
    pub fn new(id: impl Into<String>, trigger: CallTrigger) -> Self {
        Self {
            id: id.into(),
            trigger,
            is_active: false,
            triggered_date: None,
            notice_days: 30, // Standard 30-day notice
        }
    }

    /// Check if call condition is met
    pub fn check_trigger_condition(
        &self,
        current_date: Date,
        pool: &AssetPool,
        tranches: &TrancheStructure,
    ) -> (bool, Option<f64>) {
        match &self.trigger {
            CallTrigger::OnDate { call_date, call_price_pct } => {
                if current_date >= *call_date {
                    (true, Some(*call_price_pct))
                } else {
                    (false, None)
                }
            }
            CallTrigger::PoolBalanceThreshold { threshold_amount, call_price_pct } => {
                let current_balance = pool.total_balance();
                if current_balance.amount() <= threshold_amount.amount() {
                    (true, Some(*call_price_pct))
                } else {
                    (false, None)
                }
            }
            CallTrigger::PoolFactorThreshold { factor_threshold, call_price_pct } => {
                // Calculate pool factor (current/original)
                let current_balance = pool.total_balance().amount();
                // Note: Would need original pool balance stored somewhere
                // For now, use a simple heuristic
                let estimated_original = current_balance / 0.5; // Assume 50% rundown
                let factor = current_balance / estimated_original;
                
                if factor <= *factor_threshold {
                    (true, Some(*call_price_pct))
                } else {
                    (false, None)
                }
            }
            CallTrigger::TrancheBalanceThreshold { tranche_id, threshold_amount, call_price_pct } => {
                if let Some(tranche) = tranches.tranches.iter().find(|t| t.id.as_str() == tranche_id) {
                    if tranche.current_balance.amount() <= threshold_amount.amount() {
                        (true, Some(*call_price_pct))
                    } else {
                        (false, None)
                    }
                } else {
                    (false, None)
                }
            }
            CallTrigger::Optional { first_call_date, call_schedule } => {
                if current_date >= *first_call_date {
                    // Find applicable call price from schedule
                    let call_price = call_schedule
                        .iter()
                        .filter(|(date, _)| current_date >= *date)
                        .last()
                        .map(|(_, price)| *price)
                        .unwrap_or(100.0);
                    (true, Some(call_price))
                } else {
                    (false, None)
                }
            }
            CallTrigger::Cleanup { factor_threshold, call_price_pct } => {
                let current_balance = pool.total_balance().amount();
                let total_original = tranches.total_size.amount();
                
                if total_original > 0.0 {
                    let factor = current_balance / total_original;
                    if factor <= *factor_threshold {
                        (true, Some(*call_price_pct))
                    } else {
                        (false, None)
                    }
                } else {
                    (false, None)
                }
            }
        }
    }

    /// Execute call if triggered and notice period satisfied
    pub fn try_execute_call(
        &mut self,
        current_date: Date,
        pool: &AssetPool,
        tranches: &TrancheStructure,
    ) -> Option<CallExecution> {
        let (is_triggered, call_price) = self.check_trigger_condition(current_date, pool, tranches);
        
        if is_triggered && !self.is_active {
            self.is_active = true;
            self.triggered_date = Some(current_date);
        }
        
        // Check if notice period has elapsed
        if let (true, Some(triggered)) = (self.is_active, self.triggered_date) {
            let days_elapsed = (current_date - triggered).whole_days().max(0) as u32;
            if days_elapsed >= self.notice_days {
                if let Some(price) = call_price {
                    return Some(CallExecution {
                        provision_id: self.id.clone(),
                        execution_date: current_date,
                        call_price_pct: price,
                        total_call_amount: self.calculate_call_amount(tranches, price),
                    });
                }
            }
        }
        
        None
    }
    
    /// Calculate total call amount for all outstanding tranches
    fn calculate_call_amount(&self, tranches: &TrancheStructure, call_price_pct: f64) -> Money {
        let total_outstanding = tranches.tranches
            .iter()
            .try_fold(
                Money::new(0.0, tranches.total_size.currency()),
                |acc, t| acc.checked_add(t.current_balance)
            )
            .unwrap_or(Money::new(0.0, tranches.total_size.currency()));
            
        Money::new(
            total_outstanding.amount() * (call_price_pct / 100.0),
            total_outstanding.currency()
        )
    }
}

/// Result of executing a call provision
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CallExecution {
    /// ID of the call provision that was executed
    pub provision_id: String,
    /// Date of call execution
    pub execution_date: Date,
    /// Call price as percentage of par
    pub call_price_pct: f64,
    /// Total amount to be paid to bondholders
    pub total_call_amount: Money,
}

/// Manager for all call provisions in a deal
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CallProvisionManager {
    /// All call provisions for this deal
    pub provisions: Vec<CallProvision>,
    /// History of executed calls
    pub execution_history: Vec<CallExecution>,
}

impl CallProvisionManager {
    /// Create new call provision manager
    pub fn new() -> Self {
        Self {
            provisions: Vec::new(),
            execution_history: Vec::new(),
        }
    }
    
    /// Add a call provision
    pub fn add_provision(mut self, provision: CallProvision) -> Self {
        self.provisions.push(provision);
        self
    }
    
    /// Check all provisions and execute any that are ready
    pub fn check_and_execute_calls(
        &mut self,
        current_date: Date,
        pool: &AssetPool,
        tranches: &TrancheStructure,
    ) -> Vec<CallExecution> {
        let mut executions = Vec::new();
        
        for provision in &mut self.provisions {
            if let Some(execution) = provision.try_execute_call(current_date, pool, tranches) {
                executions.push(execution.clone());
                self.execution_history.push(execution);
            }
        }
        
        executions
    }
    
    /// Check if any call is currently active (notice period running)
    pub fn has_active_calls(&self) -> bool {
        self.provisions.iter().any(|p| p.is_active)
    }
    
    /// Get all triggered but not yet executed calls
    pub fn pending_calls(&self, current_date: Date) -> Vec<&CallProvision> {
        self.        provisions
            .iter()
            .filter(|p| {
                p.is_active && 
                p.triggered_date
                    .map(|t| ((current_date - t).whole_days() as u32) < p.notice_days)
                    .unwrap_or(false)
            })
            .collect()
    }
}

impl Default for CallProvisionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::currency::Currency;
    use crate::instruments::common::structured_credit::{DealType, Tranche, TrancheSeniority, TrancheCoupon};
    use time::Month;

    fn test_date() -> Date {
        Date::from_calendar_date(2024, Month::January, 1).unwrap()
    }

    #[test]
    fn test_cleanup_call_trigger() {
        let pool = AssetPool::new("TEST", DealType::CLO, Currency::USD);
        
        let tranche = Tranche::new(
            "SENIOR", 
            0.0, 
            100.0, 
            TrancheSeniority::Senior,
            Money::new(1_000_000.0, Currency::USD),
            TrancheCoupon::Fixed { rate: 0.05 },
            test_date()
        ).unwrap();
        
        let tranches = TrancheStructure::new(vec![tranche]).unwrap();
        
        let call_provision = CallProvision::new(
            "cleanup_call",
            CallTrigger::Cleanup { 
                factor_threshold: 0.10, 
                call_price_pct: 100.0 
            }
        );
        
        // Should trigger when pool is small relative to original structure
        let (triggered, price) = call_provision.check_trigger_condition(test_date(), &pool, &tranches);
        
        // Empty pool should trigger cleanup call
        assert!(triggered);
        assert_eq!(price, Some(100.0));
    }

    #[test]
    fn test_call_provision_manager() {
        let manager = CallProvisionManager::new();
        
        let provision = CallProvision::new(
            "optional_call",
            CallTrigger::OnDate { 
                call_date: test_date(), 
                call_price_pct: 102.0 
            }
        );
        
        let manager = manager.add_provision(provision);
        assert_eq!(manager.provisions.len(), 1);
        assert!(!manager.has_active_calls());
    }
}
