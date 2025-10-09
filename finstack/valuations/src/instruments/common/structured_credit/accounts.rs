//! Account management system for structured credit deals.
//!
//! Provides a comprehensive account framework supporting:
//! - Reserve accounts with targets/floors/caps
//! - Principal Deficiency Ledgers (PDL) for tracking shortfalls
//! - Collection accounts for cash management
//! - Bank accounts with interest accrual
//! - Liquidity facilities for temporary funding
//!
//! This module centralizes account state management and provides a clean
//! interface for waterfall interactions.

use finstack_core::dates::Date;
use finstack_core::money::Money;
use finstack_core::Result;
use std::collections::HashMap;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Generic account trait for all deal-level accounts
pub trait Account: std::fmt::Debug + Send + Sync {
    /// Get current account balance
    fn balance(&self) -> Money;
    
    /// Deposit money into account
    fn deposit(&mut self, amount: Money) -> Result<Money>;
    
    /// Withdraw money from account (respects floor constraints)
    fn withdraw(&mut self, amount: Money) -> Result<Money>;
    
    /// Get account identifier
    fn id(&self) -> &str;
    
    /// Get account type for reporting
    fn account_type(&self) -> AccountType;
    
    /// Update account for new period (interest accrual, etc.)
    fn update_for_period(&mut self, payment_date: Date, context: &AccountUpdateContext) -> Result<()>;
    
    /// Check account constraints and return violations
    fn check_constraints(&self) -> Vec<String>;
    
    /// Get maximum withdrawal allowed (considering floors)
    fn max_withdrawal(&self) -> Money;
    
    /// Get shortfall from target (for reserve accounts)
    fn shortfall_from_target(&self) -> Option<Money>;

    /// Downcast for serialization support
    fn as_any(&self) -> &dyn std::any::Any;
}

/// Account types for classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum AccountType {
    Reserve,
    Collection,
    PrincipalDeficiencyLedger,
    Bank,
    LiquidityFacility,
    ExcessSpread,
    Cash,
}

/// Context for account updates
#[derive(Debug, Clone)]
pub struct AccountUpdateContext {
    /// Current payment date
    pub payment_date: Date,
    /// Pool balance for percentage-based calculations
    pub pool_balance: Money,
    /// Applicable interest rates
    pub interest_rates: HashMap<String, f64>,
}

/// Reserve account with target, floor, and cap constraints
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ReserveAccount {
    /// Account identifier
    pub id: String,
    /// Current balance
    pub balance: Money,
    /// Target balance to maintain
    pub target_balance: Money,
    /// Floor balance (cannot withdraw below this)
    pub floor_balance: Money,
    /// Optional cap balance (excess gets released)
    pub cap_balance: Option<Money>,
    /// Whether account earns interest
    pub earns_interest: bool,
    /// Interest rate curve ID (if applicable)
    pub interest_curve_id: Option<String>,
}

impl ReserveAccount {
    /// Create new reserve account
    pub fn new(
        id: impl Into<String>, 
        target: Money, 
        floor: Money
    ) -> Self {
        let balance = Money::new(0.0, target.currency());
        Self {
            id: id.into(),
            balance,
            target_balance: target,
            floor_balance: floor,
            cap_balance: None,
            earns_interest: false,
            interest_curve_id: None,
        }
    }
    
    /// With interest earning capability
    pub fn with_interest(mut self, curve_id: impl Into<String>) -> Self {
        self.earns_interest = true;
        self.interest_curve_id = Some(curve_id.into());
        self
    }
    
    /// With cap balance
    pub fn with_cap(mut self, cap: Money) -> Self {
        self.cap_balance = Some(cap);
        self
    }
}

impl Account for ReserveAccount {
    fn balance(&self) -> Money {
        self.balance
    }
    
    fn deposit(&mut self, amount: Money) -> Result<Money> {
        let new_balance = self.balance.checked_add(amount)?;
        
        // Check cap constraint
        let deposited = if let Some(cap) = self.cap_balance {
            if new_balance.amount() > cap.amount() {
                let excess = new_balance.checked_sub(cap)?;
                self.balance = cap;
                amount.checked_sub(excess)?
            } else {
                self.balance = new_balance;
                amount
            }
        } else {
            self.balance = new_balance;
            amount
        };
        
        Ok(deposited)
    }
    
    fn withdraw(&mut self, amount: Money) -> Result<Money> {
        let available = self.max_withdrawal();
        let withdrawn = if amount.amount() <= available.amount() {
            self.balance = self.balance.checked_sub(amount)?;
            amount
        } else {
            self.balance = self.floor_balance;
            available
        };
        
        Ok(withdrawn)
    }
    
    fn id(&self) -> &str {
        &self.id
    }
    
    fn account_type(&self) -> AccountType {
        AccountType::Reserve
    }
    
    fn update_for_period(&mut self, _payment_date: Date, context: &AccountUpdateContext) -> Result<()> {
        if self.earns_interest {
            if let Some(curve_id) = &self.interest_curve_id {
                if let Some(&rate) = context.interest_rates.get(curve_id) {
                    let interest = Money::new(
                        self.balance.amount() * rate / 4.0, // Quarterly
                        self.balance.currency()
                    );
                    self.balance = self.balance.checked_add(interest)?;
                }
            }
        }
        Ok(())
    }
    
    fn check_constraints(&self) -> Vec<String> {
        let mut violations = Vec::new();
        
        if self.balance.amount() < self.floor_balance.amount() {
            violations.push(format!(
                "Reserve {} below floor: {} < {}",
                self.id,
                self.balance.amount(),
                self.floor_balance.amount()
            ));
        }
        
        violations
    }
    
    fn max_withdrawal(&self) -> Money {
        if self.balance.amount() > self.floor_balance.amount() {
            self.balance.checked_sub(self.floor_balance).unwrap_or(
                Money::new(0.0, self.balance.currency())
            )
        } else {
            Money::new(0.0, self.balance.currency())
        }
    }
    
    fn shortfall_from_target(&self) -> Option<Money> {
        if self.balance.amount() < self.target_balance.amount() {
            Some(self.target_balance.checked_sub(self.balance).unwrap_or(
                Money::new(0.0, self.balance.currency())
            ))
        } else {
            None
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Principal Deficiency Ledger - critical for structured credit
/// 
/// Tracks principal shortfalls for each tranche and enables "curing"
/// of shortfalls through excess spread according to waterfall rules.
/// This is a key feature that Hastructure supports.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct PrincipalDeficiencyLedger {
    /// Account identifier
    pub id: String,
    /// Outstanding deficiencies by tranche
    pub deficiencies: HashMap<String, Money>,
    /// Historical cure amounts by period
    pub cure_history: Vec<(Date, String, Money)>, // (date, tranche_id, cured_amount)
}

impl PrincipalDeficiencyLedger {
    /// Create new PDL
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            deficiencies: HashMap::new(),
            cure_history: Vec::new(),
        }
    }
    
    /// Record principal shortfall for a tranche
    pub fn record_deficiency(
        &mut self, 
        tranche_id: impl Into<String>, 
        amount: Money
    ) -> Result<()> {
        let tid = tranche_id.into();
        self.deficiencies
            .entry(tid)
            .and_modify(|existing| {
                *existing = existing.checked_add(amount).unwrap_or(*existing)
            })
            .or_insert(amount);
        Ok(())
    }
    
    /// Cure deficiency using available cash (excess spread)
    pub fn cure_deficiency(
        &mut self,
        tranche_id: &str,
        cure_amount: Money,
        payment_date: Date,
    ) -> Result<Money> {
        if let Some(deficiency) = self.deficiencies.get_mut(tranche_id) {
            let actual_cure = if cure_amount.amount() >= deficiency.amount() {
                let full_cure = *deficiency;
                *deficiency = Money::new(0.0, deficiency.currency());
                self.deficiencies.remove(tranche_id);
                full_cure
            } else {
                *deficiency = deficiency.checked_sub(cure_amount)?;
                cure_amount
            };
            
            self.cure_history.push((payment_date, tranche_id.to_string(), actual_cure));
            Ok(actual_cure)
        } else {
            Ok(Money::new(0.0, cure_amount.currency()))
        }
    }
    
    /// Get total deficiencies across all tranches
    pub fn total_deficiencies(&self) -> Money {
        if let Some((_, first_deficiency)) = self.deficiencies.iter().next() {
            let base_currency = first_deficiency.currency();
            self.deficiencies.values().try_fold(
                Money::new(0.0, base_currency),
                |acc, amount| acc.checked_add(*amount)
            ).unwrap_or(Money::new(0.0, base_currency))
        } else {
            Money::new(0.0, finstack_core::currency::Currency::USD) // Default fallback
        }
    }
}

impl Account for PrincipalDeficiencyLedger {
    fn balance(&self) -> Money {
        self.total_deficiencies()
    }
    
    fn deposit(&mut self, _amount: Money) -> Result<Money> {
        // PDL doesn't accept deposits directly - use record_deficiency
        Ok(Money::new(0.0, _amount.currency()))
    }
    
    fn withdraw(&mut self, _amount: Money) -> Result<Money> {
        // PDL doesn't allow withdrawals - use cure_deficiency
        Ok(Money::new(0.0, _amount.currency()))
    }
    
    fn id(&self) -> &str {
        &self.id
    }
    
    fn account_type(&self) -> AccountType {
        AccountType::PrincipalDeficiencyLedger
    }
    
    fn update_for_period(&mut self, _payment_date: Date, _context: &AccountUpdateContext) -> Result<()> {
        // PDL doesn't accrue interest - deficiencies remain until cured
        Ok(())
    }
    
    fn check_constraints(&self) -> Vec<String> {
        // PDL constraints would be deal-specific
        Vec::new()
    }
    
    fn max_withdrawal(&self) -> Money {
        Money::new(0.0, finstack_core::currency::Currency::USD)
    }
    
    fn shortfall_from_target(&self) -> Option<Money> {
        let total = self.total_deficiencies();
        if total.amount() > 0.0 {
            Some(total)
        } else {
            None
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Collection account for pooling cash before waterfall distribution
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CollectionAccount {
    /// Account identifier
    pub id: String,
    /// Current balance
    pub balance: Money,
    /// Whether account earns overnight interest
    pub earns_overnight_interest: bool,
    /// Overnight interest rate
    pub overnight_rate: f64,
}

impl CollectionAccount {
    /// Create new collection account
    pub fn new(id: impl Into<String>, base_currency: finstack_core::currency::Currency) -> Self {
        Self {
            id: id.into(),
            balance: Money::new(0.0, base_currency),
            earns_overnight_interest: false,
            overnight_rate: 0.0,
        }
    }
    
    /// With overnight interest earning
    pub fn with_overnight_interest(mut self, rate: f64) -> Self {
        self.earns_overnight_interest = true;
        self.overnight_rate = rate;
        self
    }
}

impl Account for CollectionAccount {
    fn balance(&self) -> Money {
        self.balance
    }
    
    fn deposit(&mut self, amount: Money) -> Result<Money> {
        self.balance = self.balance.checked_add(amount)?;
        Ok(amount)
    }
    
    fn withdraw(&mut self, amount: Money) -> Result<Money> {
        if amount.amount() <= self.balance.amount() {
            self.balance = self.balance.checked_sub(amount)?;
            Ok(amount)
        } else {
            let available = self.balance;
            self.balance = Money::new(0.0, self.balance.currency());
            Ok(available)
        }
    }
    
    fn id(&self) -> &str {
        &self.id
    }
    
    fn account_type(&self) -> AccountType {
        AccountType::Collection
    }
    
    fn update_for_period(&mut self, _payment_date: Date, _context: &AccountUpdateContext) -> Result<()> {
        if self.earns_overnight_interest && self.balance.amount() > 0.0 {
            let interest = Money::new(
                self.balance.amount() * self.overnight_rate / 4.0, // Quarterly
                self.balance.currency()
            );
            self.balance = self.balance.checked_add(interest)?;
        }
        Ok(())
    }
    
    fn check_constraints(&self) -> Vec<String> {
        Vec::new() // Collection accounts typically have no constraints
    }
    
    fn max_withdrawal(&self) -> Money {
        self.balance
    }
    
    fn shortfall_from_target(&self) -> Option<Money> {
        None // Collection accounts don't have targets
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Liquidity facility for covering temporary shortfalls
/// 
/// Inspired by Hastructure's liquidity provider support
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct LiquidityFacility {
    /// Facility identifier
    pub id: String,
    /// Total committed capacity
    pub capacity: Money,
    /// Currently drawn amount
    pub drawn_amount: Money,
    /// Interest rate on drawn amounts
    pub drawn_rate: f64,
    /// Commitment fee on undrawn capacity
    pub commitment_fee_rate: f64,
    /// Maturity date of the facility
    pub maturity_date: Date,
    /// Whether facility is currently available
    pub is_available: bool,
}

impl LiquidityFacility {
    /// Create new liquidity facility
    pub fn new(
        id: impl Into<String>,
        capacity: Money,
        drawn_rate: f64,
        commitment_fee_rate: f64,
        maturity_date: Date,
    ) -> Self {
        Self {
            id: id.into(),
            capacity,
            drawn_amount: Money::new(0.0, capacity.currency()),
            drawn_rate,
            commitment_fee_rate,
            maturity_date,
            is_available: true,
        }
    }
    
    /// Available capacity for drawing
    pub fn available_capacity(&self) -> Money {
        self.capacity.checked_sub(self.drawn_amount).unwrap_or(
            Money::new(0.0, self.capacity.currency())
        )
    }
    
    /// Draw from facility
    pub fn draw(&mut self, requested_amount: Money) -> Result<Money> {
        if !self.is_available {
            return Ok(Money::new(0.0, requested_amount.currency()));
        }
        
        let available = self.available_capacity();
        let drawn = if requested_amount.amount() <= available.amount() {
            self.drawn_amount = self.drawn_amount.checked_add(requested_amount)?;
            requested_amount
        } else {
            self.drawn_amount = self.capacity;
            available
        };
        
        Ok(drawn)
    }
    
    /// Repay drawn amount
    pub fn repay(&mut self, repayment_amount: Money) -> Result<Money> {
        let actual_repay = if repayment_amount.amount() >= self.drawn_amount.amount() {
            let full_repay = self.drawn_amount;
            self.drawn_amount = Money::new(0.0, self.drawn_amount.currency());
            full_repay
        } else {
            self.drawn_amount = self.drawn_amount.checked_sub(repayment_amount)?;
            repayment_amount
        };
        
        Ok(actual_repay)
    }
    
    /// Calculate commitment fee for the period
    pub fn commitment_fee(&self) -> Money {
        let undrawn = self.available_capacity();
        Money::new(
            undrawn.amount() * self.commitment_fee_rate / 4.0, // Quarterly
            undrawn.currency()
        )
    }
    
    /// Calculate interest on drawn amount for the period
    pub fn drawn_interest(&self) -> Money {
        Money::new(
            self.drawn_amount.amount() * self.drawn_rate / 4.0, // Quarterly
            self.drawn_amount.currency()
        )
    }
}

impl Account for LiquidityFacility {
    fn balance(&self) -> Money {
        // For liquidity facilities, "balance" represents available capacity
        self.available_capacity()
    }
    
    fn deposit(&mut self, amount: Money) -> Result<Money> {
        // "Deposit" to liquidity facility means repayment of drawn amounts
        self.repay(amount)
    }
    
    fn withdraw(&mut self, amount: Money) -> Result<Money> {
        // "Withdraw" from liquidity facility means drawing funds
        self.draw(amount)
    }
    
    fn id(&self) -> &str {
        &self.id
    }
    
    fn account_type(&self) -> AccountType {
        AccountType::LiquidityFacility
    }
    
    fn update_for_period(&mut self, payment_date: Date, _context: &AccountUpdateContext) -> Result<()> {
        // Check if facility has expired
        if payment_date >= self.maturity_date {
            self.is_available = false;
        }
        Ok(())
    }
    
    fn check_constraints(&self) -> Vec<String> {
        let mut violations = Vec::new();
        
        if !self.is_available {
            violations.push(format!("Liquidity facility {} is no longer available", self.id));
        }
        
        if self.drawn_amount.amount() > self.capacity.amount() {
            violations.push(format!(
                "Liquidity facility {} overdrawn: {} > {}",
                self.id,
                self.drawn_amount.amount(),
                self.capacity.amount()
            ));
        }
        
        violations
    }
    
    fn max_withdrawal(&self) -> Money {
        self.available_capacity()
    }
    
    fn shortfall_from_target(&self) -> Option<Money> {
        None // Liquidity facilities don't have balance targets
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Unified account manager for all deal-level accounts
#[derive(Debug)]
pub struct AccountManager {
    /// All accounts by ID
    accounts: HashMap<String, Box<dyn Account>>,
    /// Account balances snapshot for faster access
    balances_cache: HashMap<String, Money>,
}

impl AccountManager {
    /// Create new account manager
    pub fn new() -> Self {
        Self {
            accounts: HashMap::new(),
            balances_cache: HashMap::new(),
        }
    }
    
    /// Add account to manager
    pub fn add_account(&mut self, account: Box<dyn Account>) {
        let id = account.id().to_string();
        let balance = account.balance();
        self.balances_cache.insert(id.clone(), balance);
        self.accounts.insert(id, account);
    }
    
    /// Get account balance
    pub fn get_balance(&self, account_id: &str) -> Option<Money> {
        self.balances_cache.get(account_id).copied()
    }
    
    /// Deposit to account
    pub fn deposit(&mut self, account_id: &str, amount: Money) -> Result<Money> {
        if let Some(account) = self.accounts.get_mut(account_id) {
            let deposited = account.deposit(amount)?;
            self.balances_cache.insert(account_id.to_string(), account.balance());
            Ok(deposited)
        } else {
            Err(finstack_core::error::InputError::NotFound {
                id: account_id.to_string(),
            }.into())
        }
    }
    
    /// Withdraw from account
    pub fn withdraw(&mut self, account_id: &str, amount: Money) -> Result<Money> {
        if let Some(account) = self.accounts.get_mut(account_id) {
            let withdrawn = account.withdraw(amount)?;
            self.balances_cache.insert(account_id.to_string(), account.balance());
            Ok(withdrawn)
        } else {
            Err(finstack_core::error::InputError::NotFound {
                id: account_id.to_string(),
            }.into())
        }
    }
    
    /// Update all accounts for new payment period
    pub fn update_all_accounts(&mut self, context: &AccountUpdateContext) -> Result<()> {
        for account in self.accounts.values_mut() {
            account.update_for_period(context.payment_date, context)?;
        }
        
        // Refresh balance cache
        for (id, account) in &self.accounts {
            self.balances_cache.insert(id.clone(), account.balance());
        }
        
        Ok(())
    }
    
    /// Get all constraint violations across accounts
    pub fn check_all_constraints(&self) -> Vec<String> {
        self.accounts.values().flat_map(|a| a.check_constraints()).collect()
    }
    
    /// Get PDL reference for deficiency tracking (immutable)
    pub fn get_pdl(&self, pdl_id: &str) -> Option<&PrincipalDeficiencyLedger> {
        self.accounts.get(pdl_id)?.as_any().downcast_ref::<PrincipalDeficiencyLedger>()
    }
    
    /// Record deficiency in PDL (alternative to mutable access)
    pub fn record_pdl_deficiency(
        &mut self, 
        pdl_id: &str, 
        tranche_id: String, 
        amount: Money
    ) -> Result<()> {
        if let Some(account) = self.accounts.get_mut(pdl_id) {
            if let Some(pdl) = account.as_any().downcast_ref::<PrincipalDeficiencyLedger>() {
                // Create a copy, modify it, and replace
                let mut pdl_copy = pdl.clone();
                pdl_copy.record_deficiency(tranche_id, amount)?;
                // Replace with updated version (would need a more sophisticated approach in production)
                self.accounts.insert(pdl_id.to_string(), Box::new(pdl_copy));
            }
        }
        Ok(())
    }
}

impl Default for AccountManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::currency::Currency;
    use time::Month;

    fn test_date() -> Date {
        Date::from_calendar_date(2024, Month::January, 1).unwrap()
    }

    #[test]
    fn test_reserve_account_operations() {
        let mut reserve = ReserveAccount::new(
            "RESERVE_A", 
            Money::new(1_000_000.0, Currency::USD), // target
            Money::new(500_000.0, Currency::USD)     // floor
        );
        
        // Deposit
        let deposited = reserve.deposit(Money::new(750_000.0, Currency::USD)).unwrap();
        assert_eq!(deposited.amount(), 750_000.0);
        assert_eq!(reserve.balance().amount(), 750_000.0);
        
        // Try to withdraw below floor - should be limited
        let withdrawn = reserve.withdraw(Money::new(500_000.0, Currency::USD)).unwrap();
        assert_eq!(withdrawn.amount(), 250_000.0); // Only withdrew down to floor
        assert_eq!(reserve.balance().amount(), 500_000.0); // At floor
    }

    #[test]
    fn test_pdl_operations() {
        let mut pdl = PrincipalDeficiencyLedger::new("PDL_MAIN");
        
        // Record deficiencies
        pdl.record_deficiency("TRANCHE_A", Money::new(100_000.0, Currency::USD)).unwrap();
        pdl.record_deficiency("TRANCHE_B", Money::new(50_000.0, Currency::USD)).unwrap();
        
        assert_eq!(pdl.total_deficiencies().amount(), 150_000.0);
        
        // Cure part of deficiency
        let cured = pdl.cure_deficiency(
            "TRANCHE_A", 
            Money::new(60_000.0, Currency::USD),
            test_date()
        ).unwrap();
        
        assert_eq!(cured.amount(), 60_000.0);
        assert_eq!(pdl.total_deficiencies().amount(), 90_000.0);
        assert_eq!(pdl.cure_history.len(), 1);
    }

    #[test]
    fn test_liquidity_facility() {
        let mut facility = LiquidityFacility::new(
            "LIQUIDITY_A",
            Money::new(10_000_000.0, Currency::USD),
            0.03, // 3% on drawn
            0.005, // 0.5% commitment fee on undrawn
            test_date()
        );
        
        // Draw funds
        let drawn = facility.draw(Money::new(2_000_000.0, Currency::USD)).unwrap();
        assert_eq!(drawn.amount(), 2_000_000.0);
        assert_eq!(facility.available_capacity().amount(), 8_000_000.0);
        
        // Calculate fees
        let commitment_fee = facility.commitment_fee();
        let interest = facility.drawn_interest();
        
        assert!(commitment_fee.amount() > 0.0);
        assert!(interest.amount() > 0.0);
    }
}
