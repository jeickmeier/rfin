//! Capital Structure Types
//!
//! This module defines the types used for aggregated cashflow storage.
//! Instrument types (Bond, InterestRateSwap) are re-exported from finstack-valuations.

use finstack_core::currency::Currency;
use finstack_core::dates::PeriodId;
use finstack_core::money::Money;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Aggregated cashflows from capital structure instruments by period.
///
/// Instances of this type are produced by the evaluator and exposed to the DSL
/// via the `cs.*` namespace. It keeps both per-instrument details and totals so
/// that downstream consumers can drill down or report aggregates.
///
/// # Example
///
/// ```rust
/// # use finstack_statements::capital_structure::{CapitalStructureCashflows, CashflowBreakdown};
/// # use finstack_core::dates::PeriodId;
/// # use finstack_core::money::Money;
/// # use finstack_core::currency::Currency;
/// let mut cs = CapitalStructureCashflows::new();
/// let period = PeriodId::quarter(2025, 1);
/// cs.by_instrument
///     .entry("BOND-1".into())
///     .or_default()
///     .insert(period, CashflowBreakdown {
///         interest_expense_cash: Money::new(10_000.0, Currency::USD),
///         interest_expense_pik: Money::new(2_500.0, Currency::USD),
///        principal_payment: Money::new(100_000.0, Currency::USD),
///        fees: Money::new(0.0, Currency::USD),
///        debt_balance: Money::new(4_900_000.0, Currency::USD),
///     });
/// cs.totals.insert(period, CashflowBreakdown {
///     interest_expense_cash: Money::new(10_000.0, Currency::USD),
///     interest_expense_pik: Money::new(2_500.0, Currency::USD),
///     principal_payment: Money::new(100_000.0, Currency::USD),
///     fees: Money::new(0.0, Currency::USD),
///     debt_balance: Money::new(4_900_000.0, Currency::USD),
/// });
///
/// assert_eq!(cs.get_total_interest(&period), Some(12_500.0));
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CapitalStructureCashflows {
    /// Map of instrument_id → (period_id → cashflow_type → amount)
    pub by_instrument: IndexMap<String, IndexMap<PeriodId, CashflowBreakdown>>,

    /// Total cashflows across all instruments by period
    pub totals: IndexMap<PeriodId, CashflowBreakdown>,
}

/// Breakdown of cashflows by type for a single period.
///
/// # Breaking Change (v2.0)
///
/// As of v2.0, interest expense is split into cash and PIK components to provide
/// better visibility into non-cash interest accrual. The `interest_expense` field
/// is deprecated in favor of `interest_expense_cash` and `interest_expense_pik`.
///
/// Use `interest_expense_total()` to get the combined value for backward compatibility.
///
/// # Breaking Change (v3.0)
///
/// As of v3.0, all monetary fields use the Money type for currency safety.
/// Use the accessor methods to get f64 values for backward compatibility.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CashflowBreakdown {
    /// Cash interest payments (coupons, floating resets)
    pub interest_expense_cash: Money,

    /// PIK (payment-in-kind) interest accrued but not paid in cash
    pub interest_expense_pik: Money,

    /// Principal repayments (amortization, maturity)
    pub principal_payment: Money,

    /// Fees (commitment fees, etc.)
    pub fees: Money,

    /// Outstanding debt balance at period end
    pub debt_balance: Money,
}

impl CashflowBreakdown {
    /// Create a new breakdown with a specific currency.
    pub fn with_currency(currency: Currency) -> Self {
        Self {
            interest_expense_cash: Money::new(0.0, currency),
            interest_expense_pik: Money::new(0.0, currency),
            principal_payment: Money::new(0.0, currency),
            fees: Money::new(0.0, currency),
            debt_balance: Money::new(0.0, currency),
        }
    }

    /// Get total interest expense (cash + PIK).
    ///
    /// This method provides backward compatibility for code that used the
    /// deprecated `interest_expense` field.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use finstack_statements::capital_structure::CashflowBreakdown;
    /// # use finstack_core::money::Money;
    /// # use finstack_core::currency::Currency;
    /// let cf = CashflowBreakdown {
    ///     interest_expense_cash: Money::new(10_000.0, Currency::USD),
    ///     interest_expense_pik: Money::new(2_500.0, Currency::USD),
    ///     ..CashflowBreakdown::with_currency(Currency::USD)
    /// };
    /// assert_eq!(cf.interest_expense_total().amount(), 12_500.0);
    /// ```
    pub fn interest_expense_total(&self) -> Money {
        // SAFETY: Both values in a CashflowBreakdown have the same currency by construction
        (self.interest_expense_cash + self.interest_expense_pik)
            .expect("CashflowBreakdown values should have same currency")
    }
}

impl Default for CashflowBreakdown {
    fn default() -> Self {
        // Default to USD for backward compatibility
        Self::with_currency(Currency::USD)
    }
}

impl CapitalStructureCashflows {
    /// Create empty capital-structure cashflows.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use finstack_statements::capital_structure::CapitalStructureCashflows;
    /// let cashflows = CapitalStructureCashflows::new();
    /// assert!(cashflows.by_instrument.is_empty());
    /// assert!(cashflows.totals.is_empty());
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Get total interest expense (cash + PIK) for a specific instrument and period.
    ///
    /// # Arguments
    ///
    /// * `instrument_id` - Identifier supplied when the instrument was added to the model
    /// * `period_id` - Period for which the cashflow should be returned
    ///
    /// # Example
    ///
    /// ```rust
    /// # use finstack_statements::capital_structure::{CapitalStructureCashflows, CashflowBreakdown};
    /// # use finstack_core::dates::PeriodId;
    /// # use finstack_core::money::Money;
    /// # use finstack_core::currency::Currency;
    /// let mut cashflows = CapitalStructureCashflows::new();
    /// let period = PeriodId::quarter(2025, 1);
    /// cashflows.by_instrument.insert(
    ///     "BOND-1".into(),
    ///     [(period, CashflowBreakdown { interest_expense_cash: Money::new(5_000.0, Currency::USD), ..CashflowBreakdown::with_currency(Currency::USD) })]
    ///         .into_iter()
    ///         .collect(),
    /// );
    /// assert_eq!(cashflows.get_interest("BOND-1", &period), Some(5_000.0));
    /// ```
    pub fn get_interest(&self, instrument_id: &str, period_id: &PeriodId) -> Option<f64> {
        self.by_instrument
            .get(instrument_id)?
            .get(period_id)
            .map(|cf| cf.interest_expense_total().amount())
    }

    /// Get cash interest expense for a specific instrument and period.
    ///
    /// # Arguments
    /// * `instrument_id` - Instrument identifier
    /// * `period_id` - Period to inspect
    pub fn get_interest_cash(&self, instrument_id: &str, period_id: &PeriodId) -> Option<f64> {
        self.by_instrument
            .get(instrument_id)?
            .get(period_id)
            .map(|cf| cf.interest_expense_cash.amount())
    }

    /// Get PIK interest expense for a specific instrument and period.
    ///
    /// # Arguments
    /// * `instrument_id` - Instrument identifier
    /// * `period_id` - Period to inspect
    pub fn get_interest_pik(&self, instrument_id: &str, period_id: &PeriodId) -> Option<f64> {
        self.by_instrument
            .get(instrument_id)?
            .get(period_id)
            .map(|cf| cf.interest_expense_pik.amount())
    }

    /// Get principal payment for a specific instrument and period.
    ///
    /// # Arguments
    /// * `instrument_id` - Instrument identifier
    /// * `period_id` - Period to inspect
    pub fn get_principal(&self, instrument_id: &str, period_id: &PeriodId) -> Option<f64> {
        self.by_instrument
            .get(instrument_id)?
            .get(period_id)
            .map(|cf| cf.principal_payment.amount())
    }

    /// Get debt balance for a specific instrument and period.
    ///
    /// # Arguments
    /// * `instrument_id` - Instrument identifier
    /// * `period_id` - Period to inspect
    pub fn get_debt_balance(&self, instrument_id: &str, period_id: &PeriodId) -> Option<f64> {
        self.by_instrument
            .get(instrument_id)?
            .get(period_id)
            .map(|cf| cf.debt_balance.amount())
    }

    /// Get total interest expense (cash + PIK) across all instruments for a period.
    ///
    /// # Arguments
    /// * `period_id` - Period to inspect
    pub fn get_total_interest(&self, period_id: &PeriodId) -> Option<f64> {
        self.totals
            .get(period_id)
            .map(|cf| cf.interest_expense_total().amount())
    }

    /// Get total cash interest expense across all instruments for a period.
    ///
    /// # Arguments
    /// * `period_id` - Period to inspect
    pub fn get_total_interest_cash(&self, period_id: &PeriodId) -> Option<f64> {
        self.totals
            .get(period_id)
            .map(|cf| cf.interest_expense_cash.amount())
    }

    /// Get total PIK interest expense across all instruments for a period.
    ///
    /// # Arguments
    /// * `period_id` - Period to inspect
    pub fn get_total_interest_pik(&self, period_id: &PeriodId) -> Option<f64> {
        self.totals
            .get(period_id)
            .map(|cf| cf.interest_expense_pik.amount())
    }

    /// Get total principal payments across all instruments for a period.
    ///
    /// # Arguments
    /// * `period_id` - Period to inspect
    pub fn get_total_principal(&self, period_id: &PeriodId) -> Option<f64> {
        self.totals
            .get(period_id)
            .map(|cf| cf.principal_payment.amount())
    }

    /// Get total debt balance across all instruments for a period.
    ///
    /// # Arguments
    /// * `period_id` - Period to inspect
    pub fn get_total_debt_balance(&self, period_id: &PeriodId) -> Option<f64> {
        self.totals
            .get(period_id)
            .map(|cf| cf.debt_balance.amount())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cashflow_breakdown_default() {
        let cf = CashflowBreakdown::default();
        assert_eq!(cf.interest_expense_cash.amount(), 0.0);
        assert_eq!(cf.interest_expense_pik.amount(), 0.0);
        assert_eq!(cf.interest_expense_total().amount(), 0.0);
        assert_eq!(cf.principal_payment.amount(), 0.0);
        assert_eq!(cf.fees.amount(), 0.0);
        assert_eq!(cf.debt_balance.amount(), 0.0);
    }

    #[test]
    fn test_cashflow_breakdown_interest_total() {
        let cf = CashflowBreakdown {
            interest_expense_cash: Money::new(10_000.0, Currency::USD),
            interest_expense_pik: Money::new(2_500.0, Currency::USD),
            ..CashflowBreakdown::with_currency(Currency::USD)
        };
        assert_eq!(cf.interest_expense_total().amount(), 12_500.0);
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
            interest_expense_cash: Money::new(45_000.0, Currency::USD),
            interest_expense_pik: Money::new(5_000.0, Currency::USD),
            principal_payment: Money::new(100_000.0, Currency::USD),
            debt_balance: Money::new(1_000_000.0, Currency::USD),
            fees: Money::new(0.0, Currency::USD),
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
