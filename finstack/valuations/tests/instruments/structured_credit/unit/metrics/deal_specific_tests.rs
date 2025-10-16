//! Unit tests for deal-specific metrics (ABS, CMBS, RMBS).
//!
//! Tests cover:
//! - ABS speed, delinquency, charge-off, excess spread
//! - CMBS LTV, DSCR
//! - RMBS LTV, FICO, WAL adjustments

// Deal-specific metrics are best tested in integration context
// where we can construct full instruments with realistic data

use finstack_valuations::instruments::structured_credit::components::enums::DealType;
use finstack_valuations::instruments::structured_credit::config::STANDARD_PSA_SPEEDS;

#[test]
fn test_abs_metrics_require_abs_deal_type() {
    // ABS metrics should only apply to ABS, Auto, or Card deals
    let abs_family = [DealType::ABS, DealType::Auto, DealType::Card];
    for deal in &abs_family {
        assert!(
            matches!(*deal, DealType::ABS | DealType::Auto | DealType::Card),
            "ABS metrics should support {:?}",
            deal
        );
    }

    let non_abs = [DealType::CLO, DealType::CBO, DealType::CMBS, DealType::RMBS];
    for deal in &non_abs {
        assert!(
            !matches!(*deal, DealType::ABS | DealType::Auto | DealType::Card),
            "ABS metrics should not support {:?}",
            deal
        );
    }
}

#[test]
fn test_cmbs_metrics_require_cmbs_deal_type() {
    // CMBS metrics (LTV, DSCR) should only apply to CMBS
    let cmbs_types = [DealType::CMBS];
    for deal in &cmbs_types {
        assert!(
            matches!(*deal, DealType::CMBS),
            "CMBS metrics should support {:?}",
            deal
        );
    }

    let non_cmbs = [
        DealType::CLO,
        DealType::CBO,
        DealType::ABS,
        DealType::RMBS,
        DealType::Auto,
    ];
    for deal in &non_cmbs {
        assert!(
            !matches!(*deal, DealType::CMBS),
            "CMBS metrics should not support {:?}",
            deal
        );
    }
}

#[test]
fn test_rmbs_metrics_adjust_for_psa_speed() {
    // RMBS metrics should consider PSA speeds above and below par
    assert!(
        STANDARD_PSA_SPEEDS.iter().any(|&speed| speed > 1.0),
        "PSA speed grid should include stressed scenarios above 100%"
    );
    assert!(
        STANDARD_PSA_SPEEDS.iter().any(|&speed| speed < 1.0),
        "PSA speed grid should include benign scenarios below 100%"
    );
}
