//! Unit tests for rate conversion utilities.
//!
//! Tests cover:
//! - CPR ↔ SMM conversions
//! - CDR ↔ MDR conversions
//! - PSA to CPR conversions
//! - Mathematical precision and roundtrip conversions

use finstack_valuations::instruments::structured_credit::{
    cdr_to_mdr, cpr_to_smm, mdr_to_cdr, psa_to_cpr, smm_to_cpr,
};

// ============================================================================
// CPR ↔ SMM Conversion Tests
// ============================================================================

#[test]
fn test_cpr_to_smm_standard_rate() {
    // Arrange: 6% annual CPR (market standard)
    let cpr = 0.06;

    // Act
    let smm = cpr_to_smm(cpr);

    // Assert: Formula: SMM = 1 - (1 - CPR)^(1/12)
    // 6% CPR ≈ 0.5143% SMM
    assert!((smm - 0.005143).abs() < 0.0001);
}

#[test]
fn test_cpr_to_smm_zero_rate() {
    // Arrange
    let cpr = 0.0;

    // Act
    let smm = cpr_to_smm(cpr);

    // Assert
    assert_eq!(smm, 0.0);
}

#[test]
fn test_cpr_to_smm_high_rate() {
    // Arrange: 30% annual CPR (high prepayment)
    let cpr = 0.30;

    // Act
    let smm = cpr_to_smm(cpr);

    // Assert: Should be reasonable monthly rate
    assert!(smm > 0.02 && smm < 0.04);
}

#[test]
fn test_smm_to_cpr_standard_rate() {
    // Arrange: 0.5% monthly SMM
    let smm = 0.005;

    // Act
    let cpr = smm_to_cpr(smm);

    // Assert: Formula: CPR = 1 - (1 - SMM)^12
    // ~0.5% SMM ≈ 5.84% CPR
    assert!((cpr - 0.0584).abs() < 0.001);
}

#[test]
fn test_smm_to_cpr_zero_rate() {
    // Arrange
    let smm = 0.0;

    // Act
    let cpr = smm_to_cpr(smm);

    // Assert
    assert_eq!(cpr, 0.0);
}

#[test]
fn test_cpr_smm_roundtrip_precision() {
    // Arrange: Test multiple rates for precision
    let test_cprs = vec![0.01, 0.05, 0.10, 0.15, 0.20, 0.30];

    for cpr in test_cprs {
        // Act: Convert CPR → SMM → CPR
        let smm = cpr_to_smm(cpr);
        let cpr_back = smm_to_cpr(smm);

        // Assert: Should recover original within floating point precision
        assert!(
            (cpr - cpr_back).abs() < 1e-10,
            "Roundtrip failed for CPR={}: got {}",
            cpr,
            cpr_back
        );
    }
}

// ============================================================================
// CDR ↔ MDR Conversion Tests
// ============================================================================

#[test]
fn test_cdr_to_mdr_standard_rate() {
    // Arrange: 2% annual CDR (market standard for CLO)
    let cdr = 0.02;

    // Act
    let mdr = cdr_to_mdr(cdr);

    // Assert: Formula: MDR = 1 - (1 - CDR)^(1/12)
    // 2% CDR ≈ 0.168% MDR
    assert!((mdr - 0.00168).abs() < 0.0001);
}

#[test]
fn test_cdr_to_mdr_zero_rate() {
    // Arrange
    let cdr = 0.0;

    // Act
    let mdr = cdr_to_mdr(cdr);

    // Assert
    assert_eq!(mdr, 0.0);
}

#[test]
fn test_cdr_to_mdr_high_stress_rate() {
    // Arrange: 10% annual CDR (stress scenario)
    let cdr = 0.10;

    // Act
    let mdr = cdr_to_mdr(cdr);

    // Assert: Should be reasonable monthly rate
    assert!(mdr > 0.008 && mdr < 0.01);
}

#[test]
fn test_mdr_to_cdr_standard_rate() {
    // Arrange: 0.2% monthly MDR
    let mdr = 0.002;

    // Act
    let cdr = mdr_to_cdr(mdr);

    // Assert: Formula: CDR = 1 - (1 - MDR)^12
    // ~0.2% MDR ≈ 2.38% CDR
    assert!((cdr - 0.0238).abs() < 0.001);
}

#[test]
fn test_mdr_to_cdr_zero_rate() {
    // Arrange
    let mdr = 0.0;

    // Act
    let cdr = mdr_to_cdr(mdr);

    // Assert
    assert_eq!(cdr, 0.0);
}

#[test]
fn test_cdr_mdr_roundtrip_precision() {
    // Arrange: Test multiple rates for precision
    let test_cdrs = vec![0.005, 0.01, 0.02, 0.03, 0.05, 0.10];

    for cdr in test_cdrs {
        // Act: Convert CDR → MDR → CDR
        let mdr = cdr_to_mdr(cdr);
        let cdr_back = mdr_to_cdr(mdr);

        // Assert: Should recover original within floating point precision
        assert!(
            (cdr - cdr_back).abs() < 1e-10,
            "Roundtrip failed for CDR={}: got {}",
            cdr,
            cdr_back
        );
    }
}

// ============================================================================
// PSA to CPR Conversion Tests
// ============================================================================

#[test]
fn test_psa_to_cpr_100pct_at_month_30() {
    // Arrange: 100% PSA at month 30 (terminal rate)
    let psa_speed = 1.0;
    let month = 30;

    // Act
    let cpr = psa_to_cpr(psa_speed, month);

    // Assert: Should be 6% CPR
    assert!((cpr - 0.06).abs() < 0.0001);
}

#[test]
fn test_psa_to_cpr_100pct_at_month_15() {
    // Arrange: 100% PSA at month 15 (halfway through ramp)
    let psa_speed = 1.0;
    let month = 15;

    // Act
    let cpr = psa_to_cpr(psa_speed, month);

    // Assert: 15/30 × 6% = 3% CPR
    assert!((cpr - 0.03).abs() < 0.0001);
}

#[test]
fn test_psa_to_cpr_150pct_at_month_30() {
    // Arrange: 150% PSA at month 30
    let psa_speed = 1.5;
    let month = 30;

    // Act
    let cpr = psa_to_cpr(psa_speed, month);

    // Assert: 6% × 1.5 = 9% CPR
    assert!((cpr - 0.09).abs() < 0.0001);
}

#[test]
fn test_psa_to_cpr_200pct_at_month_20() {
    // Arrange: 200% PSA at month 20 (during ramp)
    let psa_speed = 2.0;
    let month = 20;

    // Act
    let cpr = psa_to_cpr(psa_speed, month);

    // Assert: (20/30) × 6% × 2.0 = 8% CPR
    assert!((cpr - 0.08).abs() < 0.0001);
}

#[test]
fn test_psa_to_cpr_after_month_30() {
    // Arrange: 100% PSA at month 60 (well past ramp)
    let psa_speed = 1.0;
    let month = 60;

    // Act
    let cpr = psa_to_cpr(psa_speed, month);

    // Assert: Should stay at terminal 6% CPR
    assert!((cpr - 0.06).abs() < 0.0001);
}

#[test]
fn test_psa_to_cpr_month_zero() {
    // Arrange: At origination (month 0)
    let psa_speed = 1.0;
    let month = 0;

    // Act
    let cpr = psa_to_cpr(psa_speed, month);

    // Assert: Should be 0% CPR
    assert_eq!(cpr, 0.0);
}

#[test]
fn test_psa_to_cpr_50pct_speed() {
    // Arrange: 50% PSA at month 30
    let psa_speed = 0.5;
    let month = 30;

    // Act
    let cpr = psa_to_cpr(psa_speed, month);

    // Assert: 6% × 0.5 = 3% CPR
    assert!((cpr - 0.03).abs() < 0.0001);
}

#[test]
fn test_psa_to_cpr_linear_ramp() {
    // Arrange: Test linearity during ramp period
    let psa_speed = 1.0;

    // Act & Assert: Check several points during ramp
    let month_10_cpr = psa_to_cpr(psa_speed, 10);
    let month_20_cpr = psa_to_cpr(psa_speed, 20);
    let month_30_cpr = psa_to_cpr(psa_speed, 30);

    // Should be linear: 2%, 4%, 6%
    assert!((month_10_cpr - 0.02).abs() < 0.0001);
    assert!((month_20_cpr - 0.04).abs() < 0.0001);
    assert!((month_30_cpr - 0.06).abs() < 0.0001);
}

// ============================================================================
// Formula Consistency Tests
// ============================================================================

#[test]
fn test_cpr_smm_and_cdr_mdr_use_same_formula() {
    // Arrange
    let rate = 0.05;

    // Act
    let monthly_prepay = cpr_to_smm(rate);
    let monthly_default = cdr_to_mdr(rate);

    // Assert: Both should use identical formula: 1 - (1 - r)^(1/12)
    assert!((monthly_prepay - monthly_default).abs() < 1e-15);
}

// ============================================================================
// Market Standard Rate Tests
// ============================================================================

#[test]
fn test_market_standard_rmbs_prepayment() {
    // Arrange: 100% PSA (market standard for RMBS)
    let psa_speed = 1.0;
    let month = 30;

    // Act
    let cpr = psa_to_cpr(psa_speed, month);
    let smm = cpr_to_smm(cpr);

    // Assert
    assert!((cpr - 0.06).abs() < 0.0001); // 6% CPR
    assert!((smm - 0.005143).abs() < 0.0001); // ~0.51% SMM
}

#[test]
fn test_market_standard_clo_default() {
    // Arrange: 2% CDR (market standard for CLO)
    let cdr = 0.02;

    // Act
    let mdr = cdr_to_mdr(cdr);

    // Assert
    assert!((mdr - 0.00168).abs() < 0.0001); // ~0.17% MDR
}

#[test]
fn test_market_standard_abs_prepayment() {
    // Arrange: 1.5% monthly ABS speed (typical auto loan)
    let smm = 0.015;

    // Act
    let cpr = smm_to_cpr(smm);
    
    // Debug: Check actual value
    eprintln!("SMM: {}, CPR: {}, Expected: 0.1682", smm, cpr);

    // Assert
    // Correct calculation: CPR = 1 - (1-0.015)^12 = 1 - 0.985^12 ≈ 0.1652
    assert!((cpr - 0.1652).abs() < 0.001, "Expected ~16.52% CPR, got {}", cpr);
}

// ============================================================================
// Edge Cases and Boundary Conditions
// ============================================================================

#[test]
fn test_full_prepayment_rate() {
    // Arrange: 100% prepayment (unrealistic but mathematically valid)
    let cpr = 1.0;

    // Act
    let smm = cpr_to_smm(cpr);

    // Assert: Formula should handle gracefully
    assert!(smm > 0.0 && smm <= 1.0);
}

#[test]
fn test_very_small_rates() {
    // Arrange: Very small rates (basis point level)
    let cpr = 0.0001; // 0.01% = 1bp

    // Act
    let smm = cpr_to_smm(cpr);
    let cpr_back = smm_to_cpr(smm);

    // Assert: Should maintain precision
    assert!((cpr - cpr_back).abs() < 1e-10);
}

#[test]
fn test_conversion_monotonicity() {
    // Arrange: Test that conversions are monotonic
    let rates = vec![0.0, 0.05, 0.10, 0.15, 0.20];

    // Act & Assert: CPR to SMM should be monotonically increasing
    let smms: Vec<f64> = rates.iter().map(|&cpr| cpr_to_smm(cpr)).collect();
    for i in 1..smms.len() {
        assert!(
            smms[i] > smms[i - 1],
            "CPR to SMM conversion not monotonic"
        );
    }
}

