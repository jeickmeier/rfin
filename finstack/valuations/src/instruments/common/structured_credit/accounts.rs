//! Simplified account management for structured credit deals.
//!
//! Provides basic account tracking for reserve accounts and collection accounts
//! needed in standard CLO/ABS/CMBS/RMBS waterfall execution.

use finstack_core::money::Money;
use std::collections::HashMap;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Simplified account manager for structured credit deals
/// 
/// Tracks account balances using a simple HashMap. For standard CLO valuation,
/// this provides sufficient functionality without the overhead of a complex
/// account system with traits and downcasting.
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct AccountManager {
    /// Account balances by account ID
    balances: HashMap<String, Money>,
}

impl AccountManager {
    /// Create new account manager
    pub fn new() -> Self {
        Self {
            balances: HashMap::new(),
        }
    }

    /// Get account balance
    pub fn get_balance(&self, account_id: &str) -> Option<Money> {
        self.balances.get(account_id).copied()
    }

    /// Set account balance
    pub fn set_balance(&mut self, account_id: impl Into<String>, balance: Money) {
        self.balances.insert(account_id.into(), balance);
    }

    /// Deposit to account
    pub fn deposit(&mut self, account_id: &str, amount: Money) -> finstack_core::Result<Money> {
        let current = self.balances.get(account_id).copied().unwrap_or_else(|| {
            Money::new(0.0, amount.currency())
        });
        let new_balance = current.checked_add(amount)?;
        self.balances.insert(account_id.to_string(), new_balance);
        Ok(amount)
    }

    /// Withdraw from account (up to available balance)
    pub fn withdraw(&mut self, account_id: &str, amount: Money) -> finstack_core::Result<Money> {
        let current = self.balances.get(account_id).copied().unwrap_or_else(|| {
            Money::new(0.0, amount.currency())
        });

        let withdrawn = if amount.amount() <= current.amount() {
            self.balances.insert(account_id.to_string(), current.checked_sub(amount)?);
            amount
        } else {
            self.balances.insert(account_id.to_string(), Money::new(0.0, current.currency()));
            current
        };

        Ok(withdrawn)
    }

    /// Clear all account balances
    pub fn clear(&mut self) {
        self.balances.clear();
    }

    /// Get all account IDs
    pub fn account_ids(&self) -> Vec<String> {
        self.balances.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::currency::Currency;

    #[test]
    fn test_account_operations() {
        let mut mgr = AccountManager::new();

        // Deposit
        let deposited = mgr.deposit("reserve", Money::new(100_000.0, Currency::USD)).unwrap();
        assert_eq!(deposited.amount(), 100_000.0);
        assert_eq!(mgr.get_balance("reserve").unwrap().amount(), 100_000.0);

        // Withdraw
        let withdrawn = mgr.withdraw("reserve", Money::new(30_000.0, Currency::USD)).unwrap();
        assert_eq!(withdrawn.amount(), 30_000.0);
        assert_eq!(mgr.get_balance("reserve").unwrap().amount(), 70_000.0);

        // Withdraw more than available
        let partial = mgr.withdraw("reserve", Money::new(100_000.0, Currency::USD)).unwrap();
        assert_eq!(partial.amount(), 70_000.0);
        assert_eq!(mgr.get_balance("reserve").unwrap().amount(), 0.0);
    }

    #[test]
    fn test_set_balance() {
        let mut mgr = AccountManager::new();
        mgr.set_balance("collection", Money::new(50_000.0, Currency::USD));
        assert_eq!(mgr.get_balance("collection").unwrap().amount(), 50_000.0);
    }
}
