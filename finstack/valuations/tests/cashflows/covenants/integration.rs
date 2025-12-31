//! Integration tests for covenant enforcement in private credit instruments.

use finstack_valuations::covenants::CovenantReport;

#[test]
fn test_covenant_report_smoke() {
    let report = CovenantReport::failed("Debt/EBITDA <= 4.00")
        .with_actual(5.0)
        .with_threshold(4.0);
    assert!(!report.passed);
}

// removed loan and revolver tests

// removed
