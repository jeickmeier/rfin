//! Capital Structure Types
//!
//! This module defines the types used for aggregated cashflow storage.
//! Instrument types (Bond, InterestRateSwap) are re-exported from finstack-valuations.

use finstack_core::dates::PeriodId;
use indexmap::IndexMap;

/// Aggregated cashflows from capital structure instruments by period.
#[derive(Debug, Clone, Default)]
pub struct CapitalStructureCashflows {
    /// Map of instrument_id → (period_id → cashflow_type → amount)
    pub by_instrument: IndexMap<String, IndexMap<PeriodId, CashflowBreakdown>>,

    /// Total cashflows across all instruments by period
    pub totals: IndexMap<PeriodId, CashflowBreakdown>,
}

/// Breakdown of cashflows by type for a single period.
#[derive(Debug, Clone, Default)]
pub struct CashflowBreakdown {
    /// Interest payments (coupons, floating resets)
    pub interest_expense: f64,

    /// Principal repayments (amortization, maturity)
    pub principal_payment: f64,

    /// Fees (commitment fees, etc.)
    pub fees: f64,

    /// Outstanding debt balance at period end
    pub debt_balance: f64,
}

impl CapitalStructureCashflows {
    /// Create empty capital structure cashflows.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get interest expense for a specific instrument in a specific period.
    pub fn get_interest(&self, instrument_id: &str, period_id: &PeriodId) -> Option<f64> {
        self.by_instrument
            .get(instrument_id)?
            .get(period_id)
            .map(|cf| cf.interest_expense)
    }

    /// Get principal payment for a specific instrument in a specific period.
    pub fn get_principal(&self, instrument_id: &str, period_id: &PeriodId) -> Option<f64> {
        self.by_instrument
            .get(instrument_id)?
            .get(period_id)
            .map(|cf| cf.principal_payment)
    }

    /// Get debt balance for a specific instrument in a specific period.
    pub fn get_debt_balance(&self, instrument_id: &str, period_id: &PeriodId) -> Option<f64> {
        self.by_instrument
            .get(instrument_id)?
            .get(period_id)
            .map(|cf| cf.debt_balance)
    }

    /// Get total interest expense across all instruments in a specific period.
    pub fn get_total_interest(&self, period_id: &PeriodId) -> Option<f64> {
        self.totals.get(period_id).map(|cf| cf.interest_expense)
    }

    /// Get total principal payments across all instruments in a specific period.
    pub fn get_total_principal(&self, period_id: &PeriodId) -> Option<f64> {
        self.totals.get(period_id).map(|cf| cf.principal_payment)
    }

    /// Get total debt balance across all instruments in a specific period.
    pub fn get_total_debt_balance(&self, period_id: &PeriodId) -> Option<f64> {
        self.totals.get(period_id).map(|cf| cf.debt_balance)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cashflow_breakdown_default() {
        let cf = CashflowBreakdown::default();
        assert_eq!(cf.interest_expense, 0.0);
        assert_eq!(cf.principal_payment, 0.0);
        assert_eq!(cf.fees, 0.0);
        assert_eq!(cf.debt_balance, 0.0);
    }

    #[test]
    fn test_capital_structure_cashflows_new() {
        let cs_cf = CapitalStructureCashflows::new();
        assert!(cs_cf.by_instrument.is_empty());
        assert!(cs_cf.totals.is_empty());
    }

    #[test]
    fn test_capital_structure_cashflows_accessors() {
        let mut cs_cf = CapitalStructureCashflows::new();

        let period_id = PeriodId::quarter(2025, 1);
        let breakdown = CashflowBreakdown {
            interest_expense: 50_000.0,
            principal_payment: 100_000.0,
            debt_balance: 1_000_000.0,
            fees: 0.0,
        };

        let mut period_map = IndexMap::new();
        period_map.insert(period_id, breakdown.clone());

        cs_cf
            .by_instrument
            .insert("BOND-001".to_string(), period_map);
        cs_cf.totals.insert(period_id, breakdown);

        // Test by-instrument accessors
        assert_eq!(cs_cf.get_interest("BOND-001", &period_id), Some(50_000.0));
        assert_eq!(cs_cf.get_principal("BOND-001", &period_id), Some(100_000.0));
        assert_eq!(
            cs_cf.get_debt_balance("BOND-001", &period_id),
            Some(1_000_000.0)
        );

        // Test total accessors
        assert_eq!(cs_cf.get_total_interest(&period_id), Some(50_000.0));
        assert_eq!(cs_cf.get_total_principal(&period_id), Some(100_000.0));
        assert_eq!(cs_cf.get_total_debt_balance(&period_id), Some(1_000_000.0));

        // Test missing instrument
        assert_eq!(cs_cf.get_interest("NONEXISTENT", &period_id), None);
    }
}
