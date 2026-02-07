//! Capital Structure Types
//!
//! This module defines the types used for aggregated cashflow storage.
//! Instrument types (Bond, InterestRateSwap) are re-exported from finstack-valuations.

use crate::error::Result;
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
///         principal_payment: Money::new(100_000.0, Currency::USD),
///         fees: Money::new(0.0, Currency::USD),
///         debt_balance: Money::new(4_900_000.0, Currency::USD),
///         accrued_interest: Money::new(5_000.0, Currency::USD),
///     });
/// cs.totals.insert(period, CashflowBreakdown {
///     interest_expense_cash: Money::new(10_000.0, Currency::USD),
///     interest_expense_pik: Money::new(2_500.0, Currency::USD),
///     principal_payment: Money::new(100_000.0, Currency::USD),
///     fees: Money::new(0.0, Currency::USD),
///     debt_balance: Money::new(4_900_000.0, Currency::USD),
///     accrued_interest: Money::new(5_000.0, Currency::USD),
/// });
///
/// assert_eq!(cs.get_total_interest(&period).unwrap(), 12_500.0);
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CapitalStructureCashflows {
    /// Map of instrument_id → (period_id → cashflow_type → amount)
    pub by_instrument: IndexMap<String, IndexMap<PeriodId, CashflowBreakdown>>,

    /// Total cashflows across all instruments in the reporting currency (if available)
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub totals: IndexMap<PeriodId, CashflowBreakdown>,

    /// Totals bucketed by native instrument currency
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub totals_by_currency: IndexMap<Currency, IndexMap<PeriodId, CashflowBreakdown>>,

    /// Reporting currency used for `totals` (if populated)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reporting_currency: Option<Currency>,
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

    /// Accrued interest not yet paid (liability)
    pub accrued_interest: Money,
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
            accrued_interest: Money::new(0.0, currency),
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
    #[allow(clippy::expect_used)] // Type invariant: all Money fields have same currency
    pub fn interest_expense_total(&self) -> Money {
        // SAFETY: Both values in a CashflowBreakdown have the same currency by construction
        self.interest_expense_cash
            .checked_add(self.interest_expense_pik)
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

    /// Generic accessor for instrument-period cashflow data.
    ///
    /// Extracts a value from the `by_instrument` map using the provided extractor function.
    fn get_instrument_field<F>(
        &self,
        instrument_id: &str,
        period_id: &PeriodId,
        field_name: &str,
        extractor: F,
    ) -> Result<f64>
    where
        F: Fn(&CashflowBreakdown) -> f64,
    {
        self.by_instrument
            .get(instrument_id)
            .and_then(|m| m.get(period_id))
            .map(extractor)
            .ok_or_else(|| {
                crate::error::Error::capital_structure(format!(
                    "No {} data for instrument '{}' in period {}",
                    field_name, instrument_id, period_id
                ))
            })
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
    /// assert_eq!(cashflows.get_interest("BOND-1", &period).unwrap(), 5_000.0);
    /// ```
    pub fn get_interest(&self, instrument_id: &str, period_id: &PeriodId) -> Result<f64> {
        self.get_instrument_field(instrument_id, period_id, "interest", |cf| {
            cf.interest_expense_total().amount()
        })
    }

    /// Get cash interest expense for a specific instrument and period.
    ///
    /// # Arguments
    /// * `instrument_id` - Instrument identifier
    /// * `period_id` - Period to inspect
    pub fn get_interest_cash(&self, instrument_id: &str, period_id: &PeriodId) -> Result<f64> {
        self.get_instrument_field(instrument_id, period_id, "cash interest", |cf| {
            cf.interest_expense_cash.amount()
        })
    }

    /// Get PIK interest expense for a specific instrument and period.
    ///
    /// # Arguments
    /// * `instrument_id` - Instrument identifier
    /// * `period_id` - Period to inspect
    pub fn get_interest_pik(&self, instrument_id: &str, period_id: &PeriodId) -> Result<f64> {
        self.get_instrument_field(instrument_id, period_id, "PIK interest", |cf| {
            cf.interest_expense_pik.amount()
        })
    }

    /// Get principal payment for a specific instrument and period.
    ///
    /// # Arguments
    /// * `instrument_id` - Instrument identifier
    /// * `period_id` - Period to inspect
    pub fn get_principal(&self, instrument_id: &str, period_id: &PeriodId) -> Result<f64> {
        self.get_instrument_field(instrument_id, period_id, "principal", |cf| {
            cf.principal_payment.amount()
        })
    }

    /// Get debt balance for a specific instrument and period.
    ///
    /// # Arguments
    /// * `instrument_id` - Instrument identifier
    /// * `period_id` - Period to inspect
    pub fn get_debt_balance(&self, instrument_id: &str, period_id: &PeriodId) -> Result<f64> {
        self.get_instrument_field(instrument_id, period_id, "debt balance", |cf| {
            cf.debt_balance.amount()
        })
    }

    /// Get accrued interest for a specific instrument and period.
    ///
    /// # Arguments
    /// * `instrument_id` - Instrument identifier
    /// * `period_id` - Period to inspect
    pub fn get_accrued_interest(&self, instrument_id: &str, period_id: &PeriodId) -> Result<f64> {
        self.get_instrument_field(instrument_id, period_id, "accrued interest", |cf| {
            cf.accrued_interest.amount()
        })
    }

    /// Get total interest expense (cash + PIK) across all instruments for a period.
    ///
    /// # Arguments
    /// * `period_id` - Period to inspect
    pub fn get_total_interest(&self, period_id: &PeriodId) -> Result<f64> {
        self.reporting_total(period_id, |cf| cf.interest_expense_total().amount())
    }

    /// Get total cash interest expense across all instruments for a period.
    ///
    /// # Arguments
    /// * `period_id` - Period to inspect
    pub fn get_total_interest_cash(&self, period_id: &PeriodId) -> Result<f64> {
        self.reporting_total(period_id, |cf| cf.interest_expense_cash.amount())
    }

    /// Get total PIK interest expense across all instruments for a period.
    ///
    /// # Arguments
    /// * `period_id` - Period to inspect
    pub fn get_total_interest_pik(&self, period_id: &PeriodId) -> Result<f64> {
        self.reporting_total(period_id, |cf| cf.interest_expense_pik.amount())
    }

    /// Get total principal payments across all instruments for a period.
    ///
    /// # Arguments
    /// * `period_id` - Period to inspect
    pub fn get_total_principal(&self, period_id: &PeriodId) -> Result<f64> {
        self.reporting_total(period_id, |cf| cf.principal_payment.amount())
    }

    /// Get total debt balance across all instruments for a period.
    ///
    /// # Arguments
    /// * `period_id` - Period to inspect
    pub fn get_total_debt_balance(&self, period_id: &PeriodId) -> Result<f64> {
        self.reporting_total(period_id, |cf| cf.debt_balance.amount())
    }

    /// Get total accrued interest across all instruments for a period.
    ///
    /// # Arguments
    /// * `period_id` - Period to inspect
    pub fn get_total_accrued_interest(&self, period_id: &PeriodId) -> Result<f64> {
        self.reporting_total(period_id, |cf| cf.accrued_interest.amount())
    }

    /// Helper to fetch reporting totals with better error messages.
    fn reporting_total(
        &self,
        period_id: &PeriodId,
        f: impl Fn(&CashflowBreakdown) -> f64,
    ) -> Result<f64> {
        if self.reporting_currency.is_none()
            && self.totals.is_empty()
            && self.totals_by_currency.len() > 1
        {
            return Err(crate::error::Error::capital_structure(
                "Multiple currencies present in capital structure totals and no FX provided. Supply FX in MarketContext or limit to a single currency.",
            ));
        }

        self.totals.get(period_id).map(f).ok_or_else(|| {
            crate::error::Error::capital_structure(format!(
                "No total cashflow data for period {}",
                period_id
            ))
        })
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
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
        assert_eq!(cf.accrued_interest.amount(), 0.0);
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
            accrued_interest: Money::new(2_500.0, Currency::USD),
        };

        let mut period_map = IndexMap::new();
        period_map.insert(period_id, breakdown.clone());

        cs_cf
            .by_instrument
            .insert("BOND-001".to_string(), period_map);
        cs_cf.totals.insert(period_id, breakdown);

        // Test by-instrument accessors
        assert_eq!(
            cs_cf
                .get_interest("BOND-001", &period_id)
                .expect("interest"),
            50_000.0
        );
        assert_eq!(
            cs_cf
                .get_principal("BOND-001", &period_id)
                .expect("principal"),
            100_000.0
        );
        assert_eq!(
            cs_cf
                .get_debt_balance("BOND-001", &period_id)
                .expect("balance"),
            1_000_000.0
        );
        assert_eq!(
            cs_cf
                .get_accrued_interest("BOND-001", &period_id)
                .expect("accrued"),
            2_500.0
        );

        // Test total accessors
        assert_eq!(
            cs_cf
                .get_total_interest(&period_id)
                .expect("total interest"),
            50_000.0
        );
        assert_eq!(
            cs_cf
                .get_total_principal(&period_id)
                .expect("total principal"),
            100_000.0
        );
        assert_eq!(
            cs_cf
                .get_total_debt_balance(&period_id)
                .expect("total balance"),
            1_000_000.0
        );
        assert_eq!(
            cs_cf
                .get_total_accrued_interest(&period_id)
                .expect("total accrued"),
            2_500.0
        );

        // Test missing instrument
        assert!(cs_cf.get_interest("NONEXISTENT", &period_id).is_err());
    }

    #[test]
    fn test_multi_currency_without_fx_errors_for_totals() {
        let mut cs = CapitalStructureCashflows::new();
        cs.totals_by_currency.insert(Currency::USD, IndexMap::new());
        cs.totals_by_currency.insert(Currency::EUR, IndexMap::new());

        let period = PeriodId::quarter(2025, 1);
        let err = cs.get_total_interest(&period);
        assert!(err.is_err());
    }
}

/// Waterfall specification for dynamic cash flow allocation.
///
/// Defines the priority of payments and sweep mechanics for capital structure.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WaterfallSpec {
    /// Priority order of payments (default: Fees > Interest > Amortization > Sweep > Equity)
    #[serde(default = "default_priority_of_payments")]
    pub priority_of_payments: Vec<PaymentPriority>,

    /// Excess Cash Flow (ECF) sweep specification
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ecf_sweep: Option<EcfSweepSpec>,

    /// PIK toggle specification for switching between cash and PIK interest
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pik_toggle: Option<PikToggleSpec>,
}

fn default_priority_of_payments() -> Vec<PaymentPriority> {
    vec![
        PaymentPriority::Fees,
        PaymentPriority::Interest,
        PaymentPriority::Amortization,
        PaymentPriority::Sweep,
        PaymentPriority::Equity,
    ]
}

impl Default for WaterfallSpec {
    fn default() -> Self {
        Self {
            priority_of_payments: default_priority_of_payments(),
            ecf_sweep: None,
            pik_toggle: None,
        }
    }
}

/// Payment priority levels in the waterfall.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaymentPriority {
    /// Fees (commitment fees, facility fees, etc.)
    Fees,
    /// Cash interest payments
    Interest,
    /// Scheduled amortization
    Amortization,
    /// Mandatory prepayments
    MandatoryPrepayment,
    /// Voluntary prepayments
    VoluntaryPrepayment,
    /// Excess cash flow sweep
    Sweep,
    /// Equity distributions
    Equity,
}

/// Excess Cash Flow (ECF) sweep specification.
///
/// Defines how to calculate ECF and what percentage to sweep to pay down debt.
///
/// # ECF Calculation
///
/// The standard ECF formula deducts cash interest from EBITDA:
///
/// ```text
/// ECF = EBITDA - Taxes - CapEx - ΔWC - Cash Interest Paid
/// ```
///
/// Set `cash_interest_node` to include cash interest in the deduction (recommended
/// for LBO models). If omitted, cash interest is not deducted (legacy behavior).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EcfSweepSpec {
    /// Formula or node reference for EBITDA (e.g., "ebitda" or "revenue - cogs - opex")
    pub ebitda_node: String,

    /// Formula or node reference for taxes (e.g., "taxes")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub taxes_node: Option<String>,

    /// Formula or node reference for capital expenditures (e.g., "capex")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capex_node: Option<String>,

    /// Formula or node reference for working capital change (e.g., "wc_change")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub working_capital_node: Option<String>,

    /// Formula or node reference for cash interest paid (e.g., "cs.interest_expense_cash.total").
    ///
    /// Per S&P LCD / standard LPA definitions, ECF should deduct cash interest paid.
    /// If omitted, cash interest is NOT deducted (legacy behavior).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cash_interest_node: Option<String>,

    /// Sweep percentage (e.g., 0.5 for 50%, 0.75 for 75%)
    pub sweep_percentage: f64,

    /// Target instrument ID for sweep payments (if None, applies to all term loans)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_instrument_id: Option<String>,
}

/// PIK toggle specification.
///
/// Defines conditions for switching between cash and PIK interest modes.
///
/// # Hysteresis
///
/// Set `min_periods_in_pik` to prevent oscillation when the liquidity metric
/// hovers near the threshold. Once PIK is triggered, it stays active for at
/// least that many periods before it can switch back.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PikToggleSpec {
    /// Node reference or formula for liquidity metric (e.g., "cash_balance" or "ebitda / interest_expense")
    pub liquidity_metric: String,

    /// Threshold value: if metric < threshold, enable PIK; otherwise use cash
    pub threshold: f64,

    /// Target instrument IDs (if None, applies to all instruments with PIK capability)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_instrument_ids: Option<Vec<String>>,

    /// Minimum number of periods PIK must stay active once triggered (hysteresis).
    /// Prevents oscillation when the metric hovers near the threshold.
    /// Default: 0 (no hysteresis, PIK can toggle every period).
    #[serde(default)]
    pub min_periods_in_pik: usize,
}

/// Capital structure state tracking for dynamic evaluation.
///
/// Maintains opening/closing balances and cumulative metrics across periods.
#[derive(Debug, Clone, Default)]
pub struct CapitalStructureState {
    /// Opening balances by instrument ID at the start of the current period
    pub opening_balances: IndexMap<String, Money>,

    /// Closing balances by instrument ID at the end of the current period
    pub closing_balances: IndexMap<String, Money>,

    /// Cumulative interest paid (cash) by instrument
    pub cumulative_interest_cash: IndexMap<String, Money>,

    /// Cumulative interest accrued (PIK) by instrument
    pub cumulative_interest_pik: IndexMap<String, Money>,

    /// Cumulative principal payments by instrument
    pub cumulative_principal: IndexMap<String, Money>,

    /// Current PIK mode by instrument (true = PIK enabled, false = cash)
    pub pik_mode: IndexMap<String, bool>,

    /// Number of consecutive periods each instrument has been in PIK mode.
    /// Used for hysteresis: PIK stays active until `min_periods_in_pik` is met.
    pub pik_periods_active: IndexMap<String, usize>,
}

impl CapitalStructureState {
    /// Create a new empty state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get opening balance for an instrument, defaulting to zero if not present.
    pub fn get_opening_balance(&self, instrument_id: &str, currency: Currency) -> Money {
        self.opening_balances
            .get(instrument_id)
            .copied()
            .unwrap_or_else(|| Money::new(0.0, currency))
    }

    /// Get closing balance for an instrument, defaulting to zero if not present.
    pub fn get_closing_balance(&self, instrument_id: &str, currency: Currency) -> Money {
        self.closing_balances
            .get(instrument_id)
            .copied()
            .unwrap_or_else(|| Money::new(0.0, currency))
    }

    /// Update closing balance for an instrument.
    pub fn set_closing_balance(&mut self, instrument_id: String, balance: Money) {
        self.closing_balances.insert(instrument_id, balance);
    }

    /// Advance state to next period: closing balances become opening balances.
    pub fn advance_period(&mut self) {
        self.opening_balances = self.closing_balances.clone();
    }
}
