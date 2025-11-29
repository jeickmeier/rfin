//! Tranche-specific valuation types and traits for structured credit instruments.
//!
//! This module provides result types for individual tranche valuation within
//! structured credit instruments (CLO, ABS, RMBS, CMBS).
//!
//! # Note
//!
//! The actual metric calculation functions have been moved to the `metrics/` module:
//! - [`calculate_tranche_wal`](super::super::metrics::calculate_tranche_wal) → `metrics/pricing/wal.rs`
//! - [`calculate_tranche_duration`](super::super::metrics::calculate_tranche_duration) → `metrics/risk/duration.rs`
//! - [`calculate_tranche_z_spread`](super::super::metrics::calculate_tranche_z_spread) → `metrics/risk/spreads.rs`
//! - [`calculate_tranche_cs01`](super::super::metrics::calculate_tranche_cs01) → `metrics/risk/spreads.rs`

use crate::cashflow::traits::DatedFlows;
use crate::metrics::MetricId;
use finstack_core::cashflow::CashFlow;
use finstack_core::dates::Date;
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::Result;
use std::collections::HashMap;

/// Result containing tranche-specific cashflows and metadata
#[derive(Debug, Clone)]
pub struct TrancheCashflowResult {
    /// Tranche identifier
    pub tranche_id: String,
    /// Cashflow schedule for this tranche (simple dated flows for backward compatibility)
    pub cashflows: DatedFlows,
    /// Detailed cashflows with proper classification using CFKind
    pub detailed_flows: Vec<CashFlow>,
    /// Interest cashflows (component of total)
    pub interest_flows: DatedFlows,
    /// Principal cashflows (component of total)
    pub principal_flows: DatedFlows,
    /// PIK capitalization flows (using CFKind::PIK)
    pub pik_flows: DatedFlows,
    /// Final tranche balance after all payments
    pub final_balance: Money,
    /// Total interest received
    pub total_interest: Money,
    /// Total principal received
    pub total_principal: Money,
    /// Total PIK capitalized
    pub total_pik: Money,
}

/// Tranche-specific valuation result
#[derive(Debug, Clone)]
pub struct TrancheValuation {
    /// Tranche identifier
    pub tranche_id: String,
    /// Present value of all cashflows
    pub pv: Money,
    /// Clean price (as percentage of par)
    pub clean_price: f64,
    /// Dirty price (as percentage of par)
    pub dirty_price: f64,
    /// Accrued interest
    pub accrued: Money,
    /// Weighted average life
    pub wal: f64,
    /// Modified duration
    pub modified_duration: f64,
    /// Z-spread (basis points)
    pub z_spread_bps: f64,
    /// CS01 (credit DV01)
    pub cs01: f64,
    /// Yield to maturity
    pub ytm: f64,
    /// Additional metrics
    pub metrics: HashMap<MetricId, f64>,
}

/// Extension trait for tranche-specific valuation
pub trait TrancheValuationExt {
    /// Generate cashflows for a specific tranche after waterfall allocation
    fn get_tranche_cashflows(
        &self,
        tranche_id: &str,
        context: &MarketContext,
        as_of: Date,
    ) -> Result<TrancheCashflowResult>;

    /// Calculate present value for a specific tranche
    fn value_tranche(
        &self,
        tranche_id: &str,
        context: &MarketContext,
        as_of: Date,
    ) -> Result<Money>;

    /// Get full valuation with metrics for a specific tranche
    fn value_tranche_with_metrics(
        &self,
        tranche_id: &str,
        context: &MarketContext,
        as_of: Date,
        metrics: &[MetricId],
    ) -> Result<TrancheValuation>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::currency::Currency;

    #[test]
    fn test_tranche_cashflow_result_creation() {
        let cashflow_result = TrancheCashflowResult {
            tranche_id: "AAA".to_string(),
            cashflows: vec![],
            detailed_flows: vec![],
            interest_flows: vec![],
            principal_flows: vec![
                (
                    Date::from_calendar_date(2024, time::Month::June, 30).expect("valid date"),
                    Money::new(100_000.0, Currency::USD),
                ),
                (
                    Date::from_calendar_date(2025, time::Month::June, 30).expect("valid date"),
                    Money::new(100_000.0, Currency::USD),
                ),
            ],
            pik_flows: vec![],
            final_balance: Money::new(0.0, Currency::USD),
            total_interest: Money::new(10_000.0, Currency::USD),
            total_principal: Money::new(200_000.0, Currency::USD),
            total_pik: Money::new(0.0, Currency::USD),
        };

        assert_eq!(cashflow_result.tranche_id, "AAA");
        assert_eq!(cashflow_result.principal_flows.len(), 2);
        assert_eq!(cashflow_result.total_principal.amount(), 200_000.0);
    }
}
