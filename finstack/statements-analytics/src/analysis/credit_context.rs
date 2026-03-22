//! Credit context metrics — coverage ratios derived from statement + capital structure data.

use finstack_statements::capital_structure::CapitalStructureCashflows;
use finstack_statements::evaluator::StatementResult;
use finstack_core::dates::{Period, PeriodId};
use serde::{Deserialize, Serialize};

/// Per-instrument credit context metrics derived from statement data.
///
/// Ratios are stored as plain scalars, so `2.0` means `2.0x` coverage and
/// `0.40` means `40%` loan-to-value.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CreditContextMetrics {
    /// DSCR by period: coverage_node_value / (interest + principal)
    pub dscr: Vec<(PeriodId, f64)>,
    /// Interest coverage by period: coverage_node_value / interest_expense
    pub interest_coverage: Vec<(PeriodId, f64)>,
    /// LTV by period: debt_balance / reference_value
    pub ltv: Vec<(PeriodId, f64)>,
    /// Minimum DSCR across all periods
    pub dscr_min: Option<f64>,
    /// Minimum interest coverage across all periods
    pub interest_coverage_min: Option<f64>,
}

/// Compute credit context metrics for a specific instrument.
///
/// Combines data from the statement evaluation (`coverage_node` values) with
/// capital structure cashflows to compute DSCR, interest coverage, and LTV.
///
/// # Arguments
///
/// * `statement` - Evaluated statement results containing the coverage node
///   values
/// * `cs_cashflows` - Capital structure cashflows from evaluation
/// * `instrument_id` - Which instrument to compute metrics for
/// * `coverage_node` - Statement node used as the coverage numerator, typically
///   EBITDA or EBIT
/// * `periods` - Periods over which to compute metrics
/// * `reference_value` - Optional denominator for LTV, typically enterprise
///   value or collateral value
///
/// # Returns
///
/// Returns [`CreditContextMetrics`]. If the instrument is absent from
/// `cs_cashflows`, the result is empty rather than fallible so callers can
/// aggregate over partial capital structures.
///
/// # Examples
///
/// ```rust
/// use finstack_statements::analysis::compute_credit_context;
/// use finstack_statements::capital_structure::{CapitalStructureCashflows, CashflowBreakdown};
/// use finstack_statements::evaluator::StatementResult;
/// use finstack_core::currency::Currency;
/// use finstack_core::dates::{Period, PeriodId};
/// use finstack_core::money::Money;
/// use indexmap::IndexMap;
///
/// let mut results = StatementResult::new();
/// results.nodes.insert(
///     "ebitda".to_string(),
///     IndexMap::from([(PeriodId::quarter(2025, 1), 300_000.0)]),
/// );
///
/// let period = Period {
///     id: PeriodId::quarter(2025, 1),
///     start: time::macros::date!(2025 - 01 - 01),
///     end: time::macros::date!(2025 - 04 - 01),
///     is_actual: false,
/// };
///
/// let mut cs = CapitalStructureCashflows::new();
/// cs.by_instrument.insert(
///     "TLB".to_string(),
///     IndexMap::from([(
///         period.id,
///         CashflowBreakdown {
///             interest_expense_cash: Money::new(50_000.0, Currency::USD),
///             interest_expense_pik: Money::new(0.0, Currency::USD),
///             principal_payment: Money::new(100_000.0, Currency::USD),
///             fees: Money::new(0.0, Currency::USD),
///             debt_balance: Money::new(4_000_000.0, Currency::USD),
///             accrued_interest: Money::new(0.0, Currency::USD),
///         },
///     )]),
/// );
///
/// let metrics = compute_credit_context(
///     &results,
///     &cs,
///     "TLB",
///     "ebitda",
///     std::slice::from_ref(&period),
///     Some(10_000_000.0),
/// );
///
/// assert_eq!(metrics.dscr.len(), 1);
/// assert_eq!(metrics.interest_coverage.len(), 1);
/// ```
///
/// # References
///
/// - Coverage and leverage interpretation: `docs/REFERENCES.md#tuckman-serrat-fixed-income`
pub fn compute_credit_context(
    statement: &StatementResult,
    cs_cashflows: &CapitalStructureCashflows,
    instrument_id: &str,
    coverage_node: &str,
    periods: &[Period],
    reference_value: Option<f64>,
) -> CreditContextMetrics {
    let inst_data = match cs_cashflows.by_instrument.get(instrument_id) {
        Some(data) => data,
        None => return CreditContextMetrics::default(),
    };

    let mut dscr = Vec::new();
    let mut interest_coverage = Vec::new();
    let mut ltv = Vec::new();

    for period in periods {
        let coverage_val = statement.get(coverage_node, &period.id).unwrap_or(0.0);
        if let Some(cf) = inst_data.get(&period.id) {
            let interest = cf.interest_expense_total().amount();
            let principal = cf.principal_payment.amount();
            let debt_service = interest + principal;
            let balance = cf.debt_balance.amount();

            if debt_service > 0.0 {
                dscr.push((period.id, coverage_val / debt_service));
            }
            if interest > 0.0 {
                interest_coverage.push((period.id, coverage_val / interest));
            }
            if let Some(ref_val) = reference_value {
                if ref_val > 0.0 {
                    ltv.push((period.id, balance / ref_val));
                }
            }
        }
    }

    let dscr_min = dscr.iter().map(|(_, v)| *v).reduce(f64::min);
    let interest_coverage_min = interest_coverage.iter().map(|(_, v)| *v).reduce(f64::min);

    CreditContextMetrics {
        dscr,
        interest_coverage,
        ltv,
        dscr_min,
        interest_coverage_min,
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use finstack_statements::capital_structure::CashflowBreakdown;
    use finstack_core::currency::Currency;
    use finstack_core::money::Money;
    use indexmap::IndexMap;

    fn make_result_and_cs() -> (StatementResult, CapitalStructureCashflows, Vec<Period>) {
        let mut result = StatementResult::new();
        let periods = vec![
            Period {
                id: PeriodId::quarter(2025, 1),
                start: time::macros::date!(2025 - 01 - 01),
                end: time::macros::date!(2025 - 04 - 01),
                is_actual: false,
            },
            Period {
                id: PeriodId::quarter(2025, 2),
                start: time::macros::date!(2025 - 04 - 01),
                end: time::macros::date!(2025 - 07 - 01),
                is_actual: false,
            },
        ];

        // EBITDA = 500k per quarter
        let mut ebitda_map = IndexMap::new();
        ebitda_map.insert(PeriodId::quarter(2025, 1), 500_000.0);
        ebitda_map.insert(PeriodId::quarter(2025, 2), 500_000.0);
        result.nodes.insert("ebitda".to_string(), ebitda_map);

        // CS cashflows: Bond with 50k interest, 100k principal per period
        let mut cs = CapitalStructureCashflows::new();
        let mut inst_map = IndexMap::new();
        for p in &periods {
            inst_map.insert(
                p.id,
                CashflowBreakdown {
                    interest_expense_cash: Money::new(50_000.0, Currency::USD),
                    interest_expense_pik: Money::new(0.0, Currency::USD),
                    principal_payment: Money::new(100_000.0, Currency::USD),
                    fees: Money::new(0.0, Currency::USD),
                    debt_balance: Money::new(4_000_000.0, Currency::USD),
                    accrued_interest: Money::new(0.0, Currency::USD),
                },
            );
        }
        cs.by_instrument.insert("BOND-001".to_string(), inst_map);
        (result, cs, periods)
    }

    #[test]
    fn test_dscr_computed_correctly() {
        let (result, cs, periods) = make_result_and_cs();
        let metrics = compute_credit_context(&result, &cs, "BOND-001", "ebitda", &periods, None);

        // DSCR = 500k / (50k + 100k) = 3.333x
        assert_eq!(metrics.dscr.len(), 2);
        assert!((metrics.dscr[0].1 - 3.333).abs() < 0.01);
        assert!(metrics.dscr_min.is_some());
        assert!((metrics.dscr_min.expect("dscr_min should be set") - 3.333).abs() < 0.01);
    }

    #[test]
    fn test_interest_coverage_computed_correctly() {
        let (result, cs, periods) = make_result_and_cs();
        let metrics = compute_credit_context(&result, &cs, "BOND-001", "ebitda", &periods, None);

        // Interest coverage = 500k / 50k = 10x
        assert_eq!(metrics.interest_coverage.len(), 2);
        assert!((metrics.interest_coverage[0].1 - 10.0).abs() < 0.01);
    }

    #[test]
    fn test_ltv_computed_when_reference_value_provided() {
        let (result, cs, periods) = make_result_and_cs();
        let metrics = compute_credit_context(
            &result,
            &cs,
            "BOND-001",
            "ebitda",
            &periods,
            Some(10_000_000.0),
        );

        // LTV = 4M / 10M = 0.4
        assert_eq!(metrics.ltv.len(), 2);
        assert!((metrics.ltv[0].1 - 0.4).abs() < 0.01);
    }

    #[test]
    fn test_missing_instrument_returns_empty() {
        let (result, cs, periods) = make_result_and_cs();
        let metrics = compute_credit_context(&result, &cs, "NONEXISTENT", "ebitda", &periods, None);
        assert!(metrics.dscr.is_empty());
        assert!(metrics.interest_coverage.is_empty());
        assert!(metrics.dscr_min.is_none());
    }
}
