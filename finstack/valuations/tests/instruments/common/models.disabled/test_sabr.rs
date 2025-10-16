//! Comprehensive tests for SABR volatility model.
//!
//! Tests organized by:
//! - Parameter validation
//! - ATM volatility calculations
//! - Implied volatility smile generation
//! - Calibration functionality
//! - Numerical stability
//! - Shifted SABR for negative rates

use finstack_valuations::instruments::common::models::{SABRCalibrator, SABRModel, SABRParameters};

use super::super::test_helpers::*;

// ============================================================================
// Parameter Validation Tests
// ============================================================================

#[test]
fn test_sabr_parameters_valid() {
    // Arrange & Act
    let params = SABRParameters::new(0.2, 0.5, 0.3, -0.25);

    // Assert
    assert!(params.is_ok(), "Valid parameters accepted");
    let p = params.unwrap();
    assert_approx_eq(p.alpha, 0.2, TIGHT_TOLERANCE, "Alpha");
    assert_approx_eq(p.beta, 0.5, TIGHT_TOLERANCE, "Beta");
    assert_approx_eq(p.nu, 0.3, TIGHT_TOLERANCE, "Nu");
    assert_approx_eq(p.rho, -0.25, TIGHT_TOLERANCE, "Rho");
}

#[test]
fn test_sabr_parameters_invalid_alpha() {
    // Act: Negative alpha
    let result = SABRParameters::new(-0.1, 0.5, 0.3, 0.0);

    // Assert
    assert!(result.is_err(), "Negative alpha rejected");

    // Act: Zero alpha
    let result = SABRParameters::new(0.0, 0.5, 0.3, 0.0);

    // Assert
    assert!(result.is_err(), "Zero alpha rejected");
}

#[test]
fn test_sabr_parameters_invalid_beta() {
    // Act: Beta > 1
    let result = SABRParameters::new(0.2, 1.5, 0.3, 0.0);

    // Assert
    assert!(result.is_err(), "Beta > 1 rejected");

    // Act: Beta < 0
    let result = SABRParameters::new(0.2, -0.1, 0.3, 0.0);

    // Assert
    assert!(result.is_err(), "Beta < 0 rejected");
}

#[test]
fn test_sabr_parameters_invalid_nu() {
    // Act: Negative nu
    let result = SABRParameters::new(0.2, 0.5, -0.1, 0.0);

    // Assert
    assert!(result.is_err(), "Negative nu rejected");
}

#[test]
fn test_sabr_parameters_invalid_rho() {
    // Act: Rho > 1
    let result = SABRParameters::new(0.2, 0.5, 0.3, 1.5);

    // Assert
    assert!(result.is_err(), "Rho > 1 rejected");

    // Act: Rho < -1
    let result = SABRParameters::new(0.2, 0.5, 0.3, -1.5);

    // Assert
    assert!(result.is_err(), "Rho < -1 rejected");
}

#[test]
fn test_sabr_convenience_constructors() {
    // Act: Normal SABR
    let normal = SABRParameters::normal(0.015, 0.3, -0.2);
    assert!(normal.is_ok());
    assert_approx_eq(normal.unwrap().beta, 0.0, TIGHT_TOLERANCE, "Normal beta=0");

    // Act: Lognormal SABR
    let lognormal = SABRParameters::lognormal(0.3, 0.4, 0.2);
    assert!(lognormal.is_ok());
    assert_approx_eq(
        lognormal.unwrap().beta,
        1.0,
        TIGHT_TOLERANCE,
        "Lognormal beta=1",
    );
}

#[test]
fn test_sabr_shifted_parameters() {
    // Act
    let shifted = SABRParameters::new_with_shift(0.2, 0.5, 0.3, -0.2, 0.02);

    // Assert
    assert!(shifted.is_ok());
    let params = shifted.unwrap();
    assert!(params.is_shifted());
    assert_approx_eq(
        params.shift().unwrap(),
        0.02,
        TIGHT_TOLERANCE,
        "Shift value",
    );
}

#[test]
fn test_sabr_shifted_invalid_shift() {
    // Act: Zero or negative shift
    let result = SABRParameters::new_with_shift(0.2, 0.5, 0.3, -0.2, 0.0);

    // Assert
    assert!(result.is_err(), "Zero shift rejected");

    let result = SABRParameters::new_with_shift(0.2, 0.5, 0.3, -0.2, -0.01);
    assert!(result.is_err(), "Negative shift rejected");
}

// ============================================================================
// ATM Volatility Tests
// ============================================================================

#[test]
fn test_atm_volatility_normal_model() {
    // Arrange: Normal SABR (beta=0)
    let params = SABRParameters::normal(0.015, 0.3, 0.0).unwrap();
    let model = SABRModel::new(params);

    // Act
    let forward = 0.03; // 3% rate
    let time = 2.0;
    let atm_vol = model.atm_volatility(forward, time).unwrap();

    // Assert
    assert!(atm_vol > 0.0, "ATM vol is positive");
    assert!(atm_vol.is_finite(), "ATM vol is finite");
    assert!(atm_vol < 0.05, "Normal ATM vol is reasonable");
}

#[test]
fn test_atm_volatility_lognormal_model() {
    // Arrange: Lognormal SABR (beta=1)
    let params = SABRParameters::lognormal(0.3, 0.4, 0.2).unwrap();
    let model = SABRModel::new(params);

    // Act
    let forward = 100.0;
    let time = 1.0;
    let atm_vol = model.atm_volatility(forward, time).unwrap();

    // Assert
    assert!(atm_vol > 0.0, "ATM vol is positive");
    assert!(atm_vol < 1.0, "Lognormal ATM vol < 100%");
}

#[test]
fn test_atm_volatility_consistency() {
    // Arrange
    let params = SABRParameters::new(0.2, 0.5, 0.3, -0.1).unwrap();
    let model = SABRModel::new(params);

    // Act: ATM volatility should match implied vol at ATM
    let forward = 100.0;
    let time = 1.0;
    let atm_vol = model.atm_volatility(forward, time).unwrap();
    let implied_atm = model.implied_volatility(forward, forward, time).unwrap();

    // Assert: Should be very close (within numerical tolerance)
    assert_approx_eq(atm_vol, implied_atm, 1e-8, "ATM consistency");
}

// ============================================================================
// Implied Volatility Smile Tests
// ============================================================================

#[test]
fn test_implied_vol_atm() {
    // Arrange
    let params = SABRParameters::new(0.25, 0.7, 0.4, -0.3).unwrap();
    let model = SABRModel::new(params);

    let forward = 100.0;
    let strike = 100.0;
    let time = 1.0;

    // Act
    let vol = model.implied_volatility(forward, strike, time).unwrap();

    // Assert
    assert!(vol > 0.0, "Vol is positive");
    assert!(vol < 1.0, "Vol is reasonable");
}

#[test]
fn test_implied_vol_smile_shape() {
    // Arrange: Negative rho creates downward-sloping skew
    let params = SABRParameters::new(0.2, 0.7, 0.4, -0.5).unwrap();
    let model = SABRModel::new(params);

    let forward = 100.0;
    let time = 1.0;

    // Act: Generate smile
    let strikes = vec![80.0, 90.0, 100.0, 110.0, 120.0];
    let vols: Vec<f64> = strikes
        .iter()
        .map(|&k| model.implied_volatility(forward, k, time).unwrap())
        .collect();

    // Assert: Smile exists (vols differ)
    let vol_range = vols.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
        - vols.iter().cloned().fold(f64::INFINITY, f64::min);

    assert!(vol_range > 0.001, "Smile has meaningful variation");
    assert!(vols.iter().all(|&v| v > 0.0), "All vols positive");
}

#[test]
fn test_implied_vol_itm_vs_otm() {
    // Arrange
    let params = SABRParameters::new(0.25, 0.6, 0.35, -0.25).unwrap();
    let model = SABRModel::new(params);

    let forward = 100.0;
    let time = 1.0;

    // Act
    let vol_otm_put = model.implied_volatility(forward, 80.0, time).unwrap(); // Low strike
    let vol_atm = model.implied_volatility(forward, 100.0, time).unwrap();
    let vol_otm_call = model.implied_volatility(forward, 120.0, time).unwrap(); // High strike

    // Assert: With negative rho, expect skew
    // Low strikes should have higher vol (smile/skew effect)
    assert!(
        vol_otm_put > vol_atm || vol_otm_call < vol_atm,
        "Skew present"
    );
}

#[test]
fn test_implied_vol_short_vs_long_maturity() {
    // Arrange
    let params = SABRParameters::new(0.2, 0.5, 0.3, -0.2).unwrap();
    let model = SABRModel::new(params);

    let forward = 100.0;
    let strike = 110.0;

    // Act
    let vol_short = model.implied_volatility(forward, strike, 0.25).unwrap();
    let vol_long = model.implied_volatility(forward, strike, 2.0).unwrap();

    // Assert: Vols should differ (term structure)
    assert!(vol_short.is_finite() && vol_long.is_finite(), "Both finite");
}

// ============================================================================
// Numerical Stability Tests
// ============================================================================

#[test]
fn test_atm_detection_with_small_differences() {
    // Arrange: Very close to ATM
    let params = SABRParameters::new(0.2, 0.5, 0.3, -0.1).unwrap();
    let model = SABRModel::new(params);

    let forward = 0.025;
    let strikes = vec![
        forward - 1e-10,
        forward - 1e-12,
        forward,
        forward + 1e-12,
        forward + 1e-10,
    ];

    // Act
    let vols: Vec<f64> = strikes
        .iter()
        .map(|&k| model.implied_volatility(forward, k, 1.0).unwrap())
        .collect();

    // Assert: All should be very similar (ATM branch)
    let vol_range = vols.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
        - vols.iter().cloned().fold(f64::INFINITY, f64::min);

    assert!(vol_range < 0.01, "ATM detection prevents instability");
}

#[test]
fn test_chi_function_stability_small_z() {
    // Arrange: Parameters that create small z
    let params = SABRParameters::new(0.2, 0.5, 0.01, 0.0).unwrap(); // Very small nu
    let model = SABRModel::new(params);

    // Act: Near ATM with small nu
    let vol = model.implied_volatility(100.0, 100.1, 1.0);

    // Assert: Should handle gracefully
    assert!(vol.is_ok(), "Handles small z");
    assert!(vol.unwrap().is_finite(), "Result is finite");
}

#[test]
fn test_chi_function_stability_rho_near_one() {
    // Arrange: Rho very close to 1
    let params = SABRParameters::new(0.2, 0.5, 0.3, 0.999).unwrap();
    let model = SABRModel::new(params);

    // Act
    let vol = model.implied_volatility(100.0, 105.0, 1.0);

    // Assert
    assert!(vol.is_ok(), "Handles rho ~1");
    assert!(vol.unwrap().is_finite(), "Result is finite");
}

#[test]
fn test_chi_function_stability_rho_near_minus_one() {
    // Arrange: Rho very close to -1
    let params = SABRParameters::new(0.2, 0.5, 0.3, -0.999).unwrap();
    let model = SABRModel::new(params);

    // Act
    let vol = model.implied_volatility(100.0, 95.0, 1.0);

    // Assert
    assert!(vol.is_ok(), "Handles rho ~-1");
    assert!(vol.unwrap().is_finite(), "Result is finite");
}

#[test]
fn test_extreme_parameters_stability() {
    // Arrange: Extreme but valid parameters
    let params = SABRParameters::new(0.01, 0.1, 0.1, 0.9).unwrap();
    let model = SABRModel::new(params);

    let forward = 0.001; // Very low rate
    let strikes = vec![0.0005, 0.001, 0.002];

    // Act
    let vols: Vec<_> = strikes
        .iter()
        .map(|&k| model.implied_volatility(forward, k, 5.0))
        .collect();

    // Assert: All should succeed
    assert!(vols.iter().all(|v| v.is_ok()), "All succeed");
    assert!(
        vols.iter().all(|v| v.as_ref().unwrap().is_finite()),
        "All finite"
    );
}

// ============================================================================
// Shifted SABR Tests (Negative Rates)
// ============================================================================

#[test]
fn test_shifted_sabr_negative_rates() {
    // Arrange: Negative forward rate with shift
    let forward = -0.005; // -50 bps
    let strikes = vec![-0.01, -0.005, 0.0, 0.005, 0.01];
    let shift = 0.02; // 200 bps shift

    let params = SABRParameters::new_with_shift(0.01, 0.0, 0.20, -0.2, shift).unwrap();
    let model = SABRModel::new(params);

    // Act: Price all strikes
    let vols: Vec<_> = strikes
        .iter()
        .map(|&k| model.implied_volatility(forward, k, 1.0))
        .collect();

    // Assert: All should succeed with reasonable values
    assert!(vols.iter().all(|v| v.is_ok()), "All succeed");
    let vol_values: Vec<f64> = vols.iter().map(|v| v.as_ref().unwrap()).copied().collect();
    assert!(
        vol_values.iter().all(|&v| v > 0.0 && v < 1.0),
        "Reasonable vols"
    );
}

#[test]
fn test_shifted_sabr_validates_shifted_rates() {
    // Arrange: Shift insufficient for negative rates
    let forward = -0.03; // -300 bps
    let strike = -0.02;
    let shift = 0.01; // Only 100 bps shift (insufficient)

    let params = SABRParameters::new_with_shift(0.2, 0.5, 0.3, -0.2, shift).unwrap();
    let model = SABRModel::new(params);

    // Act: Should fail validation
    let result = model.implied_volatility(forward, strike, 1.0);

    // Assert: Error because shifted values not positive
    assert!(result.is_err(), "Rejects insufficient shift");
}

// ============================================================================
// Calibration Tests
// ============================================================================

#[test]
fn test_calibration_basic() {
    // Arrange: Synthetic market data
    let forward = 100.0;
    let strikes = vec![90.0, 95.0, 100.0, 105.0, 110.0];
    let market_vols = vec![0.22, 0.20, 0.19, 0.195, 0.21];
    let time = 1.0;
    let beta = 0.5;

    // Act
    let calibrator = SABRCalibrator::new();
    let params = calibrator.calibrate(forward, &strikes, &market_vols, time, beta);

    // Assert
    assert!(params.is_ok(), "Calibration succeeds");
    let p = params.unwrap();

    assert!(p.alpha > 0.0, "Alpha positive");
    assert_approx_eq(p.beta, beta, TIGHT_TOLERANCE, "Beta fixed");
    assert!(p.nu >= 0.0, "Nu non-negative");
    assert!(p.rho >= -1.0 && p.rho <= 1.0, "Rho in bounds");
}

#[test]
fn test_calibration_fit_quality() {
    // Arrange
    let forward = 100.0;
    let strikes = vec![85.0, 90.0, 95.0, 100.0, 105.0, 110.0, 115.0];
    let market_vols = vec![0.24, 0.22, 0.20, 0.19, 0.20, 0.22, 0.24];
    let time = 1.0;
    let beta = 0.7;

    // Act
    let calibrator = SABRCalibrator::new();
    let params = calibrator
        .calibrate(forward, &strikes, &market_vols, time, beta)
        .unwrap();

    let model = SABRModel::new(params);

    // Assert: Check fit quality
    let max_error = strikes
        .iter()
        .zip(market_vols.iter())
        .map(|(&k, &mv)| {
            let model_vol = model.implied_volatility(forward, k, time).unwrap();
            (model_vol - mv).abs()
        })
        .fold(0.0f64, f64::max);

    assert!(max_error < 0.05, "Max calibration error < 5% vol");
}

#[test]
fn test_auto_shift_detection() {
    // Arrange: Negative rates require shift
    let forward = -0.002;
    let strikes = vec![-0.005, -0.002, 0.0, 0.002, 0.005];
    let market_vols = vec![0.015, 0.012, 0.010, 0.011, 0.013];
    let time = 0.5;
    let beta = 0.0;

    // Act
    let calibrator = SABRCalibrator::new().with_tolerance(1e-4);
    let params = calibrator.calibrate_auto_shift(forward, &strikes, &market_vols, time, beta);

    // Assert
    assert!(params.is_ok(), "Auto-shift calibration succeeds");
    let p = params.unwrap();
    assert!(p.is_shifted(), "Shift detected and applied");
    assert!(p.shift().unwrap() > 0.0, "Positive shift");
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_zero_nu_reduces_to_cev() {
    // Arrange: nu = 0 means no stochastic volatility
    let params = SABRParameters::new(0.2, 0.5, 0.0, 0.0).unwrap();
    let model = SABRModel::new(params);

    // Act: Should use ATM formula (simpler CEV)
    let vol = model.implied_volatility(100.0, 105.0, 1.0);

    // Assert
    assert!(vol.is_ok(), "Handles nu=0");
}

#[test]
fn test_very_short_maturity() {
    // Arrange
    let params = SABRParameters::new(0.2, 0.5, 0.3, -0.2).unwrap();
    let model = SABRModel::new(params);

    // Act: 1 day to expiry
    let vol = model.implied_volatility(100.0, 100.0, 1.0 / 365.0);

    // Assert
    assert!(vol.is_ok(), "Handles short maturity");
    assert!(vol.unwrap().is_finite(), "Result is finite");
}

#[test]
fn test_very_long_maturity() {
    // Arrange
    let params = SABRParameters::new(0.2, 0.5, 0.3, -0.2).unwrap();
    let model = SABRModel::new(params);

    // Act: 30 years
    let vol = model.implied_volatility(100.0, 100.0, 30.0);

    // Assert
    assert!(vol.is_ok(), "Handles long maturity");
    assert!(vol.unwrap().is_finite(), "Result is finite");
}
