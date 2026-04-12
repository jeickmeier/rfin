//! Pre-built check suites for common financial model patterns.
//!
//! Each factory function accepts a typed mapping and returns a ready-to-run
//! [`CheckSuite`] with the appropriate structural, data-quality, and
//! credit-reasonableness checks.

use finstack_statements::checks::builtins::{
    BalanceSheetArticulation, CashReconciliation, MissingValueCheck, NonFiniteCheck,
    RetainedEarningsReconciliation,
};
use finstack_statements::checks::{CheckSuite, PeriodScope, Severity};

use super::credit::{CoverageFloorCheck, FcfSignCheck, LeverageRangeCheck, TrendCheck};
use super::reconciliation::DepreciationReconciliation;
use super::{CreditMapping, ThreeStatementMapping, TrendDirection};

// ---------------------------------------------------------------------------
// Three-statement suite
// ---------------------------------------------------------------------------

/// Build a check suite for a three-statement financial model.
///
/// Includes:
/// - **BalanceSheetArticulation** (always)
/// - **RetainedEarningsReconciliation** (always)
/// - **CashReconciliation** (if `total_cf_node` is provided)
/// - **DepreciationReconciliation** (if `ppe_node`, `depreciation_node`, and
///   `capex_node` are all provided)
/// - **NonFiniteCheck** over all mapping nodes
/// - **MissingValueCheck** for required nodes
pub fn three_statement_checks(mapping: ThreeStatementMapping) -> CheckSuite {
    let mut builder = CheckSuite::builder("Three-Statement Checks")
        .description("Structural and data-quality checks for a three-statement model");

    // Balance sheet identity.
    builder = builder.add_check(BalanceSheetArticulation {
        assets_nodes: mapping.assets_nodes.clone(),
        liabilities_nodes: mapping.liabilities_nodes.clone(),
        equity_nodes: mapping.equity_nodes.clone(),
        tolerance: None,
    });

    // Retained earnings reconciliation.
    builder = builder.add_check(RetainedEarningsReconciliation {
        retained_earnings_node: mapping.retained_earnings_node.clone(),
        net_income_node: mapping.net_income_node.clone(),
        dividends_node: mapping.dividends_node.clone(),
        other_adjustments: vec![],
        tolerance: None,
    });

    // Cash reconciliation (requires total_cf_node).
    if let Some(ref total_cf_node) = mapping.total_cf_node {
        builder = builder.add_check(CashReconciliation {
            cash_balance_node: mapping.cash_node.clone(),
            total_cash_flow_node: total_cf_node.clone(),
            cfo_node: mapping.cfo_node.clone(),
            cfi_node: mapping.cfi_node.clone(),
            cff_node: mapping.cff_node.clone(),
            tolerance: None,
        });
    }

    // Depreciation reconciliation (requires ppe + depreciation + capex).
    if let (Some(ref ppe), Some(ref dep), Some(ref capex)) = (
        &mapping.ppe_node,
        &mapping.depreciation_node,
        &mapping.capex_node,
    ) {
        builder = builder.add_check(DepreciationReconciliation {
            depreciation_expense_node: dep.clone(),
            ppe_node: ppe.clone(),
            capex_node: capex.clone(),
            disposals_node: None,
            tolerance: None,
        });
    }

    // Data quality: NaN/Inf detection.
    builder = builder.add_check(NonFiniteCheck {
        nodes: mapping.all_nodes(),
    });

    // Data quality: required nodes present.
    let required = vec![
        mapping.cash_node,
        mapping.retained_earnings_node,
        mapping.net_income_node,
    ];
    builder = builder.add_check(MissingValueCheck {
        required_nodes: required,
        scope: PeriodScope::AllPeriods,
    });

    builder.build()
}

// ---------------------------------------------------------------------------
// Credit underwriting suite
// ---------------------------------------------------------------------------

/// Build a check suite for credit underwriting analysis.
///
/// Includes:
/// - **LeverageRangeCheck** (always)
/// - **CoverageFloorCheck** (always)
/// - **FcfSignCheck** (if `fcf_node` is provided)
/// - **TrendCheck** on leverage (decreasing is good) and coverage
///   (increasing is good), both with 3-period lookback
pub fn credit_underwriting_checks(mapping: CreditMapping) -> CheckSuite {
    let warn = mapping.leverage_warn.unwrap_or((0.0, 6.0));
    let cov_warn = mapping.coverage_min_warn.unwrap_or(1.5);

    let mut builder = CheckSuite::builder("Credit Underwriting Checks")
        .description("Leverage, coverage, cash-flow, and trend checks for credit analysis");

    builder = builder.add_check(LeverageRangeCheck {
        debt_node: mapping.debt_node.clone(),
        ebitda_node: mapping.ebitda_node.clone(),
        warn_range: warn,
        error_range: (0.0, 10.0),
    });

    builder = builder.add_check(CoverageFloorCheck {
        numerator_node: mapping.ebitda_node.clone(),
        denominator_node: mapping.interest_expense_node.clone(),
        min_warning: cov_warn,
        min_error: 1.0,
    });

    if let Some(ref fcf_node) = mapping.fcf_node {
        builder = builder.add_check(FcfSignCheck {
            fcf_node: fcf_node.clone(),
            consecutive_negative_warning: 2,
            consecutive_negative_error: 4,
        });
    }

    // Trend checks — use EBITDA as a proxy for coverage direction and
    // debt/EBITDA leverage direction.
    builder = builder.add_check(TrendCheck {
        node: mapping.ebitda_node.clone(),
        direction: TrendDirection::IncreasingIsGood,
        lookback_periods: 3,
        severity: Severity::Warning,
    });

    builder = builder.add_check(TrendCheck {
        node: mapping.debt_node,
        direction: TrendDirection::DecreasingIsGood,
        lookback_periods: 3,
        severity: Severity::Warning,
    });

    builder.build()
}

// ---------------------------------------------------------------------------
// LBO suite
// ---------------------------------------------------------------------------

/// Build a combined check suite for an LBO model.
///
/// Merges [`three_statement_checks`] and [`credit_underwriting_checks`],
/// overriding the leverage warning range to `(0.0, 8.0)` to accommodate
/// the higher leverage typical in buyouts.
pub fn lbo_model_checks(mapping: ThreeStatementMapping, credit: CreditMapping) -> CheckSuite {
    let ts_suite = three_statement_checks(mapping);

    let credit_override = CreditMapping {
        leverage_warn: Some((0.0, 8.0)),
        ..credit
    };
    let credit_suite = credit_underwriting_checks(credit_override);

    ts_suite.merge(credit_suite)
}
