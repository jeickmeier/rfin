//! Monte Carlo v0.10 integration tests - xVA framework.
//!
//! Tests CVA, DVA, and collateral modeling.

#![cfg(feature = "mc")]

use finstack_valuations::instruments::common::mc::xva::collateral::{
    apply_collateral_to_profile, CollateralAgreement,
};
use finstack_valuations::instruments::common::mc::xva::cva::{
    calculate_cva, calculate_dva, FlatHazardCurve,
};
use finstack_valuations::instruments::common::mc::xva::exposure::ExposureProfile;

// ============================================================================
// CVA Calculation Tests
// ============================================================================

#[test]
fn test_cva_basic() {
    // Simple CVA calculation
    let times = vec![0.0, 0.5, 1.0, 1.5, 2.0];
    let mut profile = ExposureProfile::new(times.clone());

    // Exposure increasing then decreasing (typical swap profile)
    profile.epe = vec![0.0, 10.0, 15.0, 12.0, 8.0];
    profile.ene = vec![0.0, 0.0, 0.0, 0.0, 0.0];

    // Flat hazard: 100bp spread, 40% recovery
    let survival = FlatHazardCurve::from_cds_spread(0.01, 0.40);

    // Flat discount factors (5% rate)
    let discount_factors = times.iter().map(|&t| (-0.05 * t).exp()).collect::<Vec<_>>();

    let recovery = 0.40;

    let result = calculate_cva(&profile, &survival, &discount_factors, recovery);

    println!("CVA Calculation:");
    println!("  Total CVA: {:.6}", result.cva);
    println!("  Average EPE: {:.6}", result.average_epe);

    // CVA should be positive
    assert!(result.cva > 0.0);

    // Should have bucket breakdown
    assert_eq!(result.time_buckets.len(), profile.num_points() - 1);
}

#[test]
fn test_cva_increases_with_exposure() {
    // Higher exposure → higher CVA
    let times = vec![0.0, 1.0];

    // Low exposure
    let mut profile_low = ExposureProfile::new(times.clone());
    profile_low.epe = vec![0.0, 10.0];

    // High exposure
    let mut profile_high = ExposureProfile::new(times.clone());
    profile_high.epe = vec![0.0, 20.0];

    let survival = FlatHazardCurve::new(0.02);
    let discount_factors = vec![1.0, 0.95];
    let recovery = 0.40;

    let cva_low = calculate_cva(&profile_low, &survival, &discount_factors, recovery);
    let cva_high = calculate_cva(&profile_high, &survival, &discount_factors, recovery);

    println!(
        "CVA comparison: Low exposure={:.6}, High exposure={:.6}",
        cva_low.cva, cva_high.cva
    );

    assert!(cva_high.cva > cva_low.cva);
    assert!((cva_high.cva / cva_low.cva - 2.0).abs() < 0.1); // Should be ~2x
}

#[test]
fn test_cva_increases_with_default_probability() {
    // Higher default probability → higher CVA
    let times = vec![0.0, 1.0];
    let mut profile = ExposureProfile::new(times.clone());
    profile.epe = vec![0.0, 10.0];

    // Low default probability
    let survival_low = FlatHazardCurve::new(0.01); // 1% hazard

    // High default probability
    let survival_high = FlatHazardCurve::new(0.05); // 5% hazard

    let discount_factors = vec![1.0, 0.95];
    let recovery = 0.40;

    let cva_low = calculate_cva(&profile, &survival_low, &discount_factors, recovery);
    let cva_high = calculate_cva(&profile, &survival_high, &discount_factors, recovery);

    println!(
        "CVA vs default prob: Low PD={:.6}, High PD={:.6}",
        cva_low.cva, cva_high.cva
    );

    assert!(cva_high.cva > cva_low.cva);
}

// ============================================================================
// DVA Calculation Tests
// ============================================================================

#[test]
fn test_dva_basic() {
    let times = vec![0.0, 1.0, 2.0];
    let mut profile = ExposureProfile::new(times);

    // Negative exposure (we owe counterparty)
    profile.epe = vec![0.0, 0.0, 0.0];
    profile.ene = vec![0.0, 5.0, 8.0];

    let own_survival = FlatHazardCurve::new(0.02);
    let discount_factors = vec![1.0, 0.95, 0.90];
    let own_recovery = 0.40;

    let dva = calculate_dva(&profile, &own_survival, &discount_factors, own_recovery);

    println!("DVA: {:.6}", dva);

    // DVA should be positive (benefit from our default)
    assert!(dva > 0.0);
}

// ============================================================================
// Collateral Tests
// ============================================================================

#[test]
fn test_collateral_reduces_cva() {
    // CVA with collateral should be lower than without
    let times = vec![0.0, 1.0, 2.0];
    let mut profile = ExposureProfile::new(times.clone());
    profile.epe = vec![0.0, 100.0, 150.0];
    profile.ene = vec![0.0, 0.0, 0.0];

    let survival = FlatHazardCurve::new(0.02);
    let discount_factors = vec![1.0, 0.95, 0.90];
    let recovery = 0.40;

    // CVA without collateral
    let cva_uncoll = calculate_cva(&profile, &survival, &discount_factors, recovery);

    // Apply collateral (50 threshold, 10 MTA)
    let agreement = CollateralAgreement::new(50.0, 10.0, 0.0);
    let coll_profile = apply_collateral_to_profile(&profile, &agreement);

    // CVA with collateral
    let cva_coll = calculate_cva(&coll_profile, &survival, &discount_factors, recovery);

    println!("CVA without collateral: {:.6}", cva_uncoll.cva);
    println!("CVA with collateral:    {:.6}", cva_coll.cva);

    // Collateral should reduce CVA
    assert!(cva_coll.cva < cva_uncoll.cva);
}

#[test]
fn test_collateral_fully_coll_zero_cva() {
    // Fully collateralized should give near-zero CVA
    let times = vec![0.0, 1.0];
    let mut profile = ExposureProfile::new(times.clone());
    profile.epe = vec![0.0, 100.0];
    profile.ene = vec![0.0, 0.0];

    // Fully collateralized
    let agreement = CollateralAgreement::fully_collateralized();
    let coll_profile = apply_collateral_to_profile(&profile, &agreement);

    let survival = FlatHazardCurve::new(0.02);
    let discount_factors = vec![1.0, 0.95];
    let recovery = 0.40;

    let cva = calculate_cva(&coll_profile, &survival, &discount_factors, recovery);

    println!("CVA with full collateral: {:.6}", cva.cva);

    // Should be near zero (fully collateralized)
    assert!(cva.cva < 0.1);
}

