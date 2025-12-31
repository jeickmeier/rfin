//! Golden value tests for PSA and SDA credit models.
//!
//! These tests verify that prepayment (PSA) and default (SDA) model implementations
//! produce correct values according to industry standards.
//!
//! # PSA (Public Securities Association) Prepayment Model
//!
//! - 100% PSA: Linear ramp from 0% CPR at month 0 to 6% CPR at month 30, then flat
//! - SMM (Single Monthly Mortality) = 1 - (1 - CPR)^(1/12)
//!
//! # SDA (Standard Default Assumption) Model
//!
//! - Ramp to 6% CDR at month 30, decline to 3% CDR terminal by month 60
//! - MDR (Monthly Default Rate) follows similar conversion to SMM

use crate::helpers::{FACTOR_TOLERANCE, RATE_TOLERANCE};

// =============================================================================
// PSA Golden Values
// =============================================================================

#[test]
fn psa_smm_golden_values() {
    // PSA (Public Securities Association) Prepayment Model Golden Values
    // 100% PSA ramps to 6% CPR over 30 months, then stays flat
    // SMM = 1 - (1 - CPR)^(1/12)
    use finstack_valuations::cashflow::builder::PrepaymentModelSpec;
    use finstack_valuations::cashflow::builder::{cpr_to_smm, smm_to_cpr};

    let model = PrepaymentModelSpec::psa_100();

    // Month 0: 0% CPR → 0% SMM
    let smm_0 = model.smm(0);
    assert!(
        smm_0.abs() < RATE_TOLERANCE,
        "PSA at month 0 should be 0% SMM, got {}",
        smm_0
    );

    // Month 15: 3% CPR (halfway through ramp) → ~0.2536% SMM
    let smm_15 = model.smm(15);
    let cpr_15 = smm_to_cpr(smm_15);
    assert!(
        (cpr_15 - 0.03).abs() < RATE_TOLERANCE,
        "PSA at month 15 should be 3% CPR, got {}",
        cpr_15
    );

    // Month 30: 6% CPR (end of ramp) → ~0.5143% SMM
    let smm_30 = model.smm(30);
    let expected_smm_30 = cpr_to_smm(0.06);
    assert!(
        (smm_30 - expected_smm_30).abs() < RATE_TOLERANCE,
        "PSA at month 30 should be {} SMM, got {}",
        expected_smm_30,
        smm_30
    );

    // Month 60: Still 6% CPR (flat after ramp)
    let smm_60 = model.smm(60);
    assert!(
        (smm_60 - expected_smm_30).abs() < RATE_TOLERANCE,
        "PSA at month 60 should still be {} SMM, got {}",
        expected_smm_30,
        smm_60
    );

    // 150% PSA should be 1.5x the base values
    let model_150 = PrepaymentModelSpec::psa(1.5);
    let smm_30_150 = model_150.smm(30);
    let cpr_30_150 = smm_to_cpr(smm_30_150);
    assert!(
        (cpr_30_150 - 0.09).abs() < RATE_TOLERANCE,
        "150% PSA at month 30 should be 9% CPR, got {}",
        cpr_30_150
    );
}

// =============================================================================
// SDA Golden Values
// =============================================================================

#[test]
fn sda_mdr_golden_values() {
    // SDA (Standard Default Assumption) Model Golden Values
    // SDA peaks at month 30 with 6% CDR, then declines to 3% terminal over next 30 months
    use finstack_valuations::cashflow::builder::smm_to_cpr;
    use finstack_valuations::cashflow::builder::DefaultModelSpec;

    let model = DefaultModelSpec::sda(1.0);

    // Month 0: 0% CDR
    let mdr_0 = model.mdr(0);
    assert!(
        mdr_0.abs() < RATE_TOLERANCE,
        "SDA at month 0 should be 0% MDR, got {}",
        mdr_0
    );

    // Month 15: 3% CDR (halfway to peak)
    let mdr_15 = model.mdr(15);
    let cdr_15 = smm_to_cpr(mdr_15);
    assert!(
        (cdr_15 - 0.03).abs() < RATE_TOLERANCE,
        "SDA at month 15 should be 3% CDR, got {}",
        cdr_15
    );

    // Month 30: 6% CDR (peak)
    let mdr_30 = model.mdr(30);
    let cdr_30 = smm_to_cpr(mdr_30);
    assert!(
        (cdr_30 - 0.06).abs() < RATE_TOLERANCE,
        "SDA at month 30 should be 6% CDR (peak), got {}",
        cdr_30
    );

    // Month 60: 3% CDR (terminal, 30 months after peak)
    let mdr_60 = model.mdr(60);
    let cdr_60 = smm_to_cpr(mdr_60);
    assert!(
        (cdr_60 - 0.03).abs() < RATE_TOLERANCE,
        "SDA at month 60 should be 3% CDR (terminal), got {}",
        cdr_60
    );

    // Month 90: Still 3% CDR (flat after terminal)
    let mdr_90 = model.mdr(90);
    let cdr_90 = smm_to_cpr(mdr_90);
    assert!(
        (cdr_90 - 0.03).abs() < RATE_TOLERANCE,
        "SDA at month 90 should still be 3% CDR, got {}",
        cdr_90
    );
}

// =============================================================================
// CPR/SMM Conversion Tests
// =============================================================================

#[test]
fn cpr_smm_conversion_roundtrip_precision() {
    // Test that CPR ↔ SMM conversion maintains precision across range
    // Formula: SMM = 1 - (1 - CPR)^(1/12)
    //          CPR = 1 - (1 - SMM)^12
    use finstack_valuations::cashflow::builder::{cpr_to_smm, smm_to_cpr};

    let test_cprs = [0.0, 0.01, 0.03, 0.06, 0.10, 0.15, 0.20, 0.50];

    for &cpr in &test_cprs {
        let smm = cpr_to_smm(cpr);
        let cpr_back = smm_to_cpr(smm);

        assert!(
            (cpr - cpr_back).abs() < FACTOR_TOLERANCE,
            "CPR {} roundtrip failed: got {}",
            cpr,
            cpr_back
        );

        // SMM should always be less than CPR (except for 0)
        if cpr > 0.0 {
            assert!(smm < cpr, "SMM ({}) should be less than CPR ({})", smm, cpr);
        }
    }

    // Verify specific golden value: 6% CPR ≈ 0.5143% SMM
    // Using exact calculation: SMM = 1 - (1 - 0.06)^(1/12) ≈ 0.005143
    let smm_6pct = cpr_to_smm(0.06);
    let expected_smm = 1.0 - (1.0 - 0.06_f64).powf(1.0 / 12.0);
    assert!(
        (smm_6pct - expected_smm).abs() < FACTOR_TOLERANCE,
        "6% CPR should convert to {} SMM, got {}",
        expected_smm,
        smm_6pct
    );
}

// =============================================================================
// PSA Industry Standard Benchmark Tests
// =============================================================================

#[test]
fn psa_matches_industry_standard_ramp() {
    // Reference: Bond Market Association PSA Standard Prepayment Model
    // 100% PSA: Linear ramp from 0% CPR at month 0 to 6% CPR at month 30
    use finstack_valuations::cashflow::builder::smm_to_cpr;
    use finstack_valuations::cashflow::builder::PrepaymentModelSpec;

    let model = PrepaymentModelSpec::psa_100();

    // Month 1: 0.2% CPR (1/30 * 6%)
    let cpr_1 = smm_to_cpr(model.smm(1));
    assert!(
        (cpr_1 - 0.002).abs() < RATE_TOLERANCE,
        "PSA month 1 should be 0.2% CPR, got {}",
        cpr_1
    );

    // Month 10: 2.0% CPR (10/30 * 6%)
    let cpr_10 = smm_to_cpr(model.smm(10));
    assert!(
        (cpr_10 - 0.02).abs() < RATE_TOLERANCE,
        "PSA month 10 should be 2.0% CPR, got {}",
        cpr_10
    );

    // Month 20: 4.0% CPR (20/30 * 6%)
    let cpr_20 = smm_to_cpr(model.smm(20));
    assert!(
        (cpr_20 - 0.04).abs() < RATE_TOLERANCE,
        "PSA month 20 should be 4.0% CPR, got {}",
        cpr_20
    );

    // Verify ramp is linear for all months 1-30
    for month in 1..=30 {
        let expected_cpr = (month as f64 / 30.0) * 0.06;
        let actual_cpr = smm_to_cpr(model.smm(month));
        assert!(
            (actual_cpr - expected_cpr).abs() < RATE_TOLERANCE,
            "PSA month {} should be {:.4}% CPR, got {:.4}%",
            month,
            expected_cpr * 100.0,
            actual_cpr * 100.0
        );
    }
}

#[test]
fn psa_multiplier_scales_correctly() {
    // Test that PSA multipliers scale linearly
    use finstack_valuations::cashflow::builder::smm_to_cpr;
    use finstack_valuations::cashflow::builder::PrepaymentModelSpec;

    // 50% PSA, 100% PSA, 200% PSA at month 30
    let psa_50 = PrepaymentModelSpec::psa(0.5);
    let psa_100 = PrepaymentModelSpec::psa_100();
    let psa_200 = PrepaymentModelSpec::psa(2.0);

    let cpr_50 = smm_to_cpr(psa_50.smm(30));
    let cpr_100 = smm_to_cpr(psa_100.smm(30));
    let cpr_200 = smm_to_cpr(psa_200.smm(30));

    assert!(
        (cpr_50 - 0.03).abs() < RATE_TOLERANCE,
        "50% PSA at month 30 should be 3% CPR, got {}",
        cpr_50
    );
    assert!(
        (cpr_100 - 0.06).abs() < RATE_TOLERANCE,
        "100% PSA at month 30 should be 6% CPR, got {}",
        cpr_100
    );
    assert!(
        (cpr_200 - 0.12).abs() < RATE_TOLERANCE,
        "200% PSA at month 30 should be 12% CPR, got {}",
        cpr_200
    );

    // Verify linear scaling relationship
    assert!(
        (cpr_100 - 2.0 * cpr_50).abs() < RATE_TOLERANCE,
        "100% PSA should be 2x 50% PSA"
    );
    assert!(
        (cpr_200 - 2.0 * cpr_100).abs() < RATE_TOLERANCE,
        "200% PSA should be 2x 100% PSA"
    );
}

#[test]
fn psa_terminal_rate_is_flat() {
    // After month 30, PSA should stay flat at terminal rate
    use finstack_valuations::cashflow::builder::smm_to_cpr;
    use finstack_valuations::cashflow::builder::PrepaymentModelSpec;

    let model = PrepaymentModelSpec::psa_100();
    let terminal_cpr = 0.06;

    // Test various months after the ramp
    for month in [31, 50, 100, 200, 360] {
        let actual_cpr = smm_to_cpr(model.smm(month));
        assert!(
            (actual_cpr - terminal_cpr).abs() < RATE_TOLERANCE,
            "PSA month {} should be terminal 6% CPR, got {}",
            month,
            actual_cpr
        );
    }
}

// =============================================================================
// SDA Industry Standard Benchmark Tests
// =============================================================================

#[test]
fn sda_matches_industry_standard_curve() {
    // Reference: Standard Default Assumption curve
    // Ramp to 6% CDR at month 30, decline to 3% CDR terminal by month 60
    use finstack_valuations::cashflow::builder::smm_to_cpr;
    use finstack_valuations::cashflow::builder::DefaultModelSpec;

    let model = DefaultModelSpec::sda(1.0);

    // Verify ramp phase (months 1-30)
    for month in 1..=30 {
        let expected_cdr = (month as f64 / 30.0) * 0.06;
        let actual_cdr = smm_to_cpr(model.mdr(month));
        assert!(
            (actual_cdr - expected_cdr).abs() < RATE_TOLERANCE,
            "SDA month {} (ramp) should be {:.4}% CDR, got {:.4}%",
            month,
            expected_cdr * 100.0,
            actual_cdr * 100.0
        );
    }

    // Verify decline phase (months 31-60)
    for month in 31..=60 {
        let months_past_peak = (month - 30) as f64;
        let expected_cdr = 0.06 - (months_past_peak / 30.0) * 0.03;
        let actual_cdr = smm_to_cpr(model.mdr(month));
        assert!(
            (actual_cdr - expected_cdr).abs() < RATE_TOLERANCE,
            "SDA month {} (decline) should be {:.4}% CDR, got {:.4}%",
            month,
            expected_cdr * 100.0,
            actual_cdr * 100.0
        );
    }

    // Verify terminal phase (month 61+)
    for month in [61, 100, 360] {
        let actual_cdr = smm_to_cpr(model.mdr(month));
        assert!(
            (actual_cdr - 0.03).abs() < RATE_TOLERANCE,
            "SDA month {} (terminal) should be 3% CDR, got {}",
            month,
            actual_cdr
        );
    }
}

#[test]
fn sda_multiplier_scales_correctly() {
    // Test that SDA multipliers scale linearly
    use finstack_valuations::cashflow::builder::smm_to_cpr;
    use finstack_valuations::cashflow::builder::DefaultModelSpec;

    let sda_100 = DefaultModelSpec::sda(1.0);
    let sda_200 = DefaultModelSpec::sda(2.0);

    // At peak (month 30)
    let cdr_100_peak = smm_to_cpr(sda_100.mdr(30));
    let cdr_200_peak = smm_to_cpr(sda_200.mdr(30));

    assert!(
        (cdr_100_peak - 0.06).abs() < RATE_TOLERANCE,
        "100% SDA peak should be 6% CDR"
    );
    assert!(
        (cdr_200_peak - 0.12).abs() < RATE_TOLERANCE,
        "200% SDA peak should be 12% CDR"
    );
    assert!(
        (cdr_200_peak - 2.0 * cdr_100_peak).abs() < RATE_TOLERANCE,
        "200% SDA should be 2x 100% SDA at peak"
    );

    // At terminal (month 90)
    let cdr_100_term = smm_to_cpr(sda_100.mdr(90));
    let cdr_200_term = smm_to_cpr(sda_200.mdr(90));

    assert!(
        (cdr_100_term - 0.03).abs() < RATE_TOLERANCE,
        "100% SDA terminal should be 3% CDR"
    );
    assert!(
        (cdr_200_term - 0.06).abs() < RATE_TOLERANCE,
        "200% SDA terminal should be 6% CDR"
    );
}
