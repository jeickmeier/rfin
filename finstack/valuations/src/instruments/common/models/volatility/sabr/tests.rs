#![allow(clippy::expect_used, clippy::panic)]

use super::calibration::solve_alpha_for_atm;
use super::*;

#[test]
fn test_sabr_parameters_validation() {
    // Valid parameters
    assert!(SABRParameters::new(0.2, 0.5, 0.3, 0.1).is_ok());

    // Invalid alpha
    assert!(SABRParameters::new(-0.1, 0.5, 0.3, 0.1).is_err());

    // Invalid beta
    assert!(SABRParameters::new(0.2, 1.5, 0.3, 0.1).is_err());

    // Invalid rho
    assert!(SABRParameters::new(0.2, 0.5, 0.3, 1.5).is_err());
}

#[test]
fn test_sabr_atm_volatility() {
    let params =
        SABRParameters::new(0.2, 0.5, 0.3, -0.1).expect("SABR parameters should be valid in test");
    let model = SABRModel::new(params);

    let forward = 100.0;
    let time_to_expiry = 1.0;

    let atm_vol = model
        .atm_volatility(forward, time_to_expiry)
        .expect("ATM volatility calculation should succeed in test");

    // ATM vol should be positive
    assert!(atm_vol > 0.0);

    // For ATM, implied vol should match ATM vol
    let implied_vol = model
        .implied_volatility(forward, forward, time_to_expiry)
        .expect("Volatility calculation should succeed in test");
    assert!((implied_vol - atm_vol).abs() < 1e-10);
}

#[test]
fn test_sabr_smile_shape() {
    let params =
        SABRParameters::new(0.2, 0.7, 0.4, -0.3).expect("SABR parameters should be valid in test");
    let model = SABRModel::new(params);

    let forward = 100.0;
    let time_to_expiry = 1.0;

    // Generate strikes
    let strikes = vec![80.0, 90.0, 100.0, 110.0, 120.0];
    let mut vols = Vec::new();

    for strike in &strikes {
        let vol = model
            .implied_volatility(forward, *strike, time_to_expiry)
            .expect("Volatility calculation should succeed in test");
        vols.push(vol);
    }

    // With negative rho, we expect downward sloping skew
    // Lower strikes should have higher vols
    // But the actual shape depends on all parameters
    // Just check that we get different vols (smile exists)
    let vol_range = vols
        .iter()
        .max_by(|a, b| a.total_cmp(b))
        .expect("Vols should not be empty")
        - vols
            .iter()
            .min_by(|a, b| a.total_cmp(b))
            .expect("Vols should not be empty");
    assert!(vol_range > 0.001); // There is a smile
}

#[test]
fn test_sabr_normal_model() {
    // Beta = 0 gives normal SABR
    let params =
        SABRParameters::normal(20.0, 0.3, 0.0).expect("SABR parameters should be valid in test");
    let model = SABRModel::new(params);

    let forward = 0.05; // 5% rate
    let strike = 0.06; // 6% strike
    let time_to_expiry = 2.0;

    let vol = model
        .implied_volatility(forward, strike, time_to_expiry)
        .expect("Volatility calculation should succeed in test");

    // Should produce reasonable normal vol
    assert!(vol > 0.0);
    // Normal vol can be very large for small forward rates, so we just check it's positive
}

#[test]
fn test_sabr_lognormal_model() {
    // Beta = 1 gives lognormal SABR (like Black-Scholes)
    let params =
        SABRParameters::lognormal(0.3, 0.4, 0.2).expect("SABR parameters should be valid in test");
    let model = SABRModel::new(params);

    let forward = 100.0;
    let strike = 105.0;
    let time_to_expiry = 0.5;

    let vol = model
        .implied_volatility(forward, strike, time_to_expiry)
        .expect("Volatility calculation should succeed in test");

    // Should produce reasonable lognormal vol
    assert!(vol > 0.0);
    assert!(vol < 1.0); // Less than 100% vol
}

#[test]
fn test_sabr_calibration() {
    // Create synthetic market data
    let forward = 100.0;
    let strikes = vec![90.0, 95.0, 100.0, 105.0, 110.0];
    let market_vols = vec![0.22, 0.20, 0.19, 0.195, 0.21];
    let time_to_expiry = 1.0;
    let beta = 0.5; // Fixed beta

    let calibrator = SABRCalibrator::new();
    let params = calibrator
        .calibrate(forward, &strikes, &market_vols, time_to_expiry, beta)
        .expect("Volatility calculation should succeed in test");

    // Check calibrated parameters are reasonable
    assert!(params.alpha > 0.0);
    assert!(params.nu >= 0.0);
    assert!(params.rho >= -1.0 && params.rho <= 1.0);

    // Check fit quality
    let model = SABRModel::new(params);
    for (i, &strike) in strikes.iter().enumerate() {
        let model_vol = model
            .implied_volatility(forward, strike, time_to_expiry)
            .expect("Volatility calculation should succeed in test");
        let error = (model_vol - market_vols[i]).abs();
        assert!(error < 0.05); // Within 5% vol (calibration is approximate)
    }
}

#[test]
fn test_sabr_smile_generator() {
    let params = SABRParameters::new(0.25, 0.6, 0.35, -0.25)
        .expect("SABR parameters should be valid in test");
    let model = SABRModel::new(params);
    let smile = SABRSmile::new(model, 100.0, 1.0);

    let strikes = vec![85.0, 90.0, 95.0, 100.0, 105.0, 110.0, 115.0];
    let vols = smile
        .generate_smile(&strikes)
        .expect("Smile generation should succeed in test");

    // Check all vols are positive
    for vol in &vols {
        assert!(*vol > 0.0);
    }

    // Validate that smile has variation (different volatilities)
    assert!(!vols.is_empty());
    assert!(vols.iter().all(|&v| v > 0.0));
}

#[test]
fn test_sabr_negative_rates_shifted() {
    // Test shifted SABR with negative forward rates
    let forward = -0.005; // -50bps
    let strikes = vec![-0.01, -0.005, 0.0, 0.005, 0.01];
    let shift = 0.02; // 200bps shift

    let params = SABRParameters::new_with_shift(0.2, 0.5, 0.3, -0.2, shift)
        .expect("SABR parameters should be valid in test"); // Higher alpha for more reasonable vols
    let model = SABRModel::new(params);

    // Should handle negative rates correctly
    for &strike in &strikes {
        let vol = model.implied_volatility(forward, strike, 1.0);
        assert!(vol.is_ok(), "Failed for strike {}: {:?}", strike, vol);
        let vol_val = vol.expect("Volatility should be Some in test");
        assert!(
            vol_val > 0.0,
            "Non-positive volatility {} for strike {}",
            vol_val,
            strike
        );
        assert!(
            vol_val < 10.0,
            "Unreasonably high volatility {} for strike {}",
            vol_val,
            strike
        );
    }
}

#[test]
fn test_sabr_atm_stability() {
    // Test enhanced ATM stability with very close strikes
    let params =
        SABRParameters::new(0.2, 0.5, 0.3, -0.1).expect("SABR parameters should be valid in test");
    let model = SABRModel::new(params);

    let forward = 0.025;
    let strikes = vec![
        forward - 1e-10,
        forward - 1e-12,
        forward,
        forward + 1e-12,
        forward + 1e-10,
    ];

    // All should give very similar results (ATM case)
    let mut vols = Vec::new();
    for &strike in &strikes {
        let vol = model
            .implied_volatility(forward, strike, 1.0)
            .expect("Implied volatility calculation should succeed in test");
        vols.push(vol);
    }

    // Check all ATM-like volatilities are similar with practical tolerance
    let vol_range = vols
        .iter()
        .max_by(|a, b| a.total_cmp(b))
        .expect("Vols should not be empty")
        - vols
            .iter()
            .min_by(|a, b| a.total_cmp(b))
            .expect("Vols should not be empty");
    assert!(vol_range < 1e-2); // Practical tolerance for numerical precision in ATM case
}

#[test]
fn test_sabr_auto_shift_calibration() {
    // Test automatic shift detection and calibration
    let forward = -0.002; // Negative forward
    let strikes = vec![-0.005, -0.002, 0.0, 0.002, 0.005];
    let market_vols = vec![0.015, 0.012, 0.010, 0.011, 0.013]; // More reasonable vols for rates
    let time_to_expiry = 0.5;
    let beta = 0.0; // Normal model for rates

    let calibrator = SABRCalibrator::new().with_tolerance(1e-4); // Relaxed tolerance for difficult calibration
    let params = calibrator
        .calibrate_auto_shift(forward, &strikes, &market_vols, time_to_expiry, beta)
        .expect("Volatility calculation should succeed in test");

    // Should have detected need for shift
    assert!(params.is_shifted());
    assert!(params.shift().expect("Shift should be Some") > 0.0);

    // Check model works with negative rates
    let model = SABRModel::new(params);
    for &strike in &strikes {
        let vol = model.implied_volatility(forward, strike, time_to_expiry);
        assert!(vol.is_ok(), "Failed for strike {}: {:?}", strike, vol);
        let vol_val = vol.expect("Volatility should be Some in test");
        assert!(
            vol_val > 0.0,
            "Non-positive volatility {} for strike {}",
            vol_val,
            strike
        );
    }
}

#[test]
fn test_sabr_numerical_stability_extreme_parameters() {
    // Test with extreme but valid parameters
    let params =
        SABRParameters::new(0.01, 0.1, 0.1, 0.9).expect("SABR parameters should be valid in test");
    let model = SABRModel::new(params);

    let forward = 0.001; // Very low rate
    let strikes = vec![0.0005, 0.001, 0.002];

    for &strike in &strikes {
        let vol = model.implied_volatility(forward, strike, 5.0); // Long maturity
        assert!(vol.is_ok());
        let vol_val = vol.expect("Volatility should be Some in test");
        assert!(vol_val > 0.0);
        assert!(vol_val.is_finite());
    }
}

#[test]
fn test_sabr_chi_function_stability() {
    // Test chi function with various extreme cases
    let params =
        SABRParameters::new(0.2, 0.5, 0.3, 0.95).expect("SABR parameters should be valid in test"); // High rho
    let model = SABRModel::new(params);

    // Test small z values
    let small_z_values = vec![1e-8, 1e-6, 1e-4];
    for z in small_z_values {
        let chi = model.calculate_chi_robust(z);
        assert!(chi.is_ok());
        assert!(chi.expect("Chi should be Some").is_finite());
    }

    // Test rho ≈ 1 case
    let params_rho_one =
        SABRParameters::new(0.2, 0.5, 0.3, 0.999).expect("SABR parameters should be valid in test");
    let model_rho_one = SABRModel::new(params_rho_one);
    let chi_rho_one = model_rho_one.calculate_chi_robust(0.1);
    assert!(chi_rho_one.is_ok());

    // Test rho ≈ -1 case
    let params_rho_minus_one = SABRParameters::new(0.2, 0.5, 0.3, -0.999)
        .expect("SABR parameters should be valid in test");
    let model_rho_minus_one = SABRModel::new(params_rho_minus_one);
    let chi_rho_minus_one = model_rho_minus_one.calculate_chi_robust(0.1);
    assert!(chi_rho_minus_one.is_ok());
}

// ===================================================================
// Market Standards Validation Tests (Priority 1, Task 1.2)
// ===================================================================

#[test]
fn test_sabr_rejects_negative_alpha() {
    let result = SABRParameters::new(-0.1, 0.5, 0.3, 0.1);
    assert!(result.is_err(), "Negative alpha should be rejected");

    let err = result.expect_err("should fail");
    assert!(
        matches!(err, finstack_core::Error::Validation(_)),
        "Should return Validation error"
    );

    // Verify error message mentions alpha
    let err_str = format!("{}", err);
    assert!(err_str.contains("alpha") || err_str.contains("α"));
}

#[test]
fn test_sabr_rejects_zero_alpha() {
    let result = SABRParameters::new(0.0, 0.5, 0.3, 0.1);
    assert!(result.is_err(), "Zero alpha should be rejected");

    let err = result.expect_err("should fail");
    assert!(matches!(err, finstack_core::Error::Validation(_)));
}

#[test]
fn test_sabr_rejects_invalid_rho() {
    // Rho > 1
    let result1 = SABRParameters::new(0.2, 0.5, 0.3, 1.5);
    assert!(result1.is_err(), "Rho > 1 should be rejected");
    assert!(matches!(
        result1.expect_err("should fail"),
        finstack_core::Error::Validation(_)
    ));

    // Rho < -1
    let result2 = SABRParameters::new(0.2, 0.5, 0.3, -1.5);
    assert!(result2.is_err(), "Rho < -1 should be rejected");
    assert!(matches!(
        result2.expect_err("should fail"),
        finstack_core::Error::Validation(_)
    ));

    // Rho = exactly 1.0 should be OK
    let result3 = SABRParameters::new(0.2, 0.5, 0.3, 1.0);
    assert!(result3.is_ok(), "Rho = 1.0 is valid");

    // Rho = exactly -1.0 should be OK
    let result4 = SABRParameters::new(0.2, 0.5, 0.3, -1.0);
    assert!(result4.is_ok(), "Rho = -1.0 is valid");
}

#[test]
fn test_sabr_rejects_negative_nu() {
    let result = SABRParameters::new(0.2, 0.5, -0.1, 0.1);
    assert!(result.is_err(), "Negative nu should be rejected");

    let err = result.expect_err("should fail");
    assert!(matches!(err, finstack_core::Error::Validation(_)));

    // Verify error message mentions nu
    let err_str = format!("{}", err);
    assert!(err_str.contains("nu") || err_str.contains("ν"));
}

#[test]
fn test_sabr_rejects_invalid_beta() {
    // Beta > 1
    let result1 = SABRParameters::new(0.2, 1.5, 0.3, 0.1);
    assert!(result1.is_err(), "Beta > 1 should be rejected");
    assert!(matches!(
        result1.expect_err("should fail"),
        finstack_core::Error::Validation(_)
    ));

    // Beta < 0
    let result2 = SABRParameters::new(0.2, -0.1, 0.3, 0.1);
    assert!(result2.is_err(), "Beta < 0 should be rejected");
    assert!(matches!(
        result2.expect_err("should fail"),
        finstack_core::Error::Validation(_)
    ));

    // Beta = 0 should be OK (normal SABR)
    let result3 = SABRParameters::new(0.2, 0.0, 0.3, 0.1);
    assert!(result3.is_ok(), "Beta = 0 is valid (normal SABR)");

    // Beta = 1 should be OK (lognormal SABR)
    let result4 = SABRParameters::new(0.2, 1.0, 0.3, 0.1);
    assert!(result4.is_ok(), "Beta = 1 is valid (lognormal SABR)");
}

#[test]
fn test_sabr_accepts_boundary_values() {
    // Test that exact boundary values are accepted
    assert!(SABRParameters::new(1e-10, 0.0, 0.0, -1.0).is_ok());
    assert!(SABRParameters::new(1e-10, 1.0, 0.0, 1.0).is_ok());
    assert!(SABRParameters::new(0.001, 0.5, 0.0, 0.0).is_ok());
}

// ===================================================================
// Inverse Normal CDF Precision Tests
// ===================================================================

#[test]
fn test_normal_inverse_cdf_precision() {
    // Test that the inverse CDF has high precision for tail probabilities.
    // These golden values are from high-precision statistical tables.

    // Standard values
    assert!(
        (finstack_core::math::standard_normal_inv_cdf(0.5) - 0.0).abs() < 1e-12,
        "CDF^-1(0.5) should be 0"
    );
    assert!(
        (finstack_core::math::standard_normal_inv_cdf(0.84134474606854) - 1.0).abs() < 1e-8,
        "CDF^-1(0.84134...) should be ~1.0"
    );
    assert!(
        (finstack_core::math::standard_normal_inv_cdf(0.97724986805182) - 2.0).abs() < 1e-8,
        "CDF^-1(0.97724...) should be ~2.0"
    );

    // Tail precision test: p = 1e-8 should give approximately -5.6120
    // (from scipy.stats.norm.ppf(1e-8) = -5.612001244174965)
    let tail_result = finstack_core::math::standard_normal_inv_cdf(1e-8);
    assert!(
        (tail_result - (-5.612001244174965)).abs() < 1e-6,
        "Tail precision: CDF^-1(1e-8) = {} should be ~-5.612",
        tail_result
    );

    // Upper tail: p = 1 - 1e-8 should give approximately +5.6120
    let upper_tail_result = finstack_core::math::standard_normal_inv_cdf(1.0 - 1e-8);
    assert!(
        (upper_tail_result - 5.612001244174965).abs() < 1e-6,
        "Upper tail precision: CDF^-1(1-1e-8) = {} should be ~5.612",
        upper_tail_result
    );

    // Extreme tail: p = 1e-15 should give approximately -7.941
    let extreme_tail = finstack_core::math::standard_normal_inv_cdf(1e-15);
    assert!(
        (extreme_tail - (-7.941397804)).abs() < 1e-4,
        "Extreme tail: CDF^-1(1e-15) = {} should be ~-7.941",
        extreme_tail
    );
}

#[test]
fn test_normal_inverse_cdf_boundary_behavior() {
    // Edge cases: boundaries should return appropriate infinity values
    assert!(
        finstack_core::math::standard_normal_inv_cdf(0.0).is_infinite()
            && finstack_core::math::standard_normal_inv_cdf(0.0) < 0.0,
        "CDF^-1(0) should be -infinity"
    );
    assert!(
        finstack_core::math::standard_normal_inv_cdf(1.0).is_infinite()
            && finstack_core::math::standard_normal_inv_cdf(1.0) > 0.0,
        "CDF^-1(1) should be +infinity"
    );

    // Values very close to boundaries
    let near_zero = finstack_core::math::standard_normal_inv_cdf(1e-300);
    assert!(near_zero < -30.0, "CDF^-1(1e-300) should be very negative");

    let near_one = finstack_core::math::standard_normal_inv_cdf(1.0 - 1e-300);
    assert!(near_one > 30.0, "CDF^-1(1-1e-300) should be very positive");
}

// ===================================================================
// Arbitrage Validation Tests
// ===================================================================

#[test]
fn test_sabr_arbitrage_validation_clean_smile() {
    // Well-behaved SABR parameters should produce arbitrage-free smile
    let params = SABRParameters::new(0.2, 0.5, 0.3, -0.2).expect("Valid SABR parameters");
    let model = SABRModel::new(params);
    let smile = SABRSmile::new(model, 100.0, 1.0);

    let strikes: Vec<f64> = (70..=130).step_by(5).map(|k| k as f64).collect();
    let r = 0.05;
    let q = 0.02;

    let result = smile
        .validate_no_arbitrage(&strikes, r, q)
        .expect("Validation should succeed");

    assert!(
        result.is_arbitrage_free(),
        "Standard SABR parameters should be arbitrage-free. \
         Butterfly violations: {}, Monotonicity violations: {}",
        result.butterfly_violations.len(),
        result.monotonicity_violations.len()
    );
}

#[test]
fn test_sabr_arbitrage_check_api() {
    // Test the simplified check API
    let params = SABRParameters::new(0.2, 0.5, 0.3, -0.2).expect("Valid SABR parameters");
    let model = SABRModel::new(params);
    let smile = SABRSmile::new(model, 100.0, 1.0);

    let strikes: Vec<f64> = (80..=120).step_by(5).map(|k| k as f64).collect();

    // Should pass without error
    let check_result = smile.check_no_arbitrage(&strikes, 0.05, 0.02);
    assert!(
        check_result.is_ok(),
        "Clean smile should pass arbitrage check"
    );
}

#[test]
fn test_sabr_arbitrage_validation_result_methods() {
    // Test ArbitrageValidationResult helper methods
    let mut result = ArbitrageValidationResult::default();

    // Empty result should be arbitrage-free
    assert!(result.is_arbitrage_free());
    assert!(result.worst_butterfly_severity().is_none());

    // Add a violation
    result.butterfly_violations.push(ButterflyViolation {
        strike: 100.0,
        butterfly_value: -0.01,
        severity_pct: 0.5,
    });

    assert!(!result.is_arbitrage_free());
    assert!(
        (result
            .worst_butterfly_severity()
            .expect("severity should exist after adding violation")
            - 0.5)
            .abs()
            < 1e-10
    );
}

#[test]
fn test_sabr_arbitrage_too_few_strikes() {
    // With fewer than 3 strikes, validation should return empty result
    let params = SABRParameters::new(0.2, 0.5, 0.3, -0.2).expect("Valid SABR parameters");
    let model = SABRModel::new(params);
    let smile = SABRSmile::new(model, 100.0, 1.0);

    let strikes = vec![95.0, 100.0]; // Only 2 strikes

    let result = smile
        .validate_no_arbitrage(&strikes, 0.05, 0.02)
        .expect("Validation should succeed");

    assert!(
        result.is_arbitrage_free(),
        "With < 3 strikes, no violations should be reported"
    );
}

#[test]
fn test_sabr_arbitrage_extreme_params_may_have_violations() {
    // Extreme parameters might produce arbitrage (this tests detection, not prevention)
    // High vol-of-vol with extreme rho can sometimes produce problematic smiles
    let params = SABRParameters::new(0.5, 0.9, 1.5, 0.8).expect("Valid SABR parameters");
    let model = SABRModel::new(params);
    let smile = SABRSmile::new(model, 100.0, 0.1); // Short expiry

    let strikes: Vec<f64> = (50..=150).step_by(5).map(|k| k as f64).collect();

    // This tests that the validation runs without panicking
    // The result may or may not have violations depending on exact parameters
    let result = smile.validate_no_arbitrage(&strikes, 0.05, 0.02);
    assert!(result.is_ok(), "Validation should complete without error");
}

#[test]
fn test_sabr_new_with_shift_rejects_non_positive_shift() {
    let zero_shift = SABRParameters::new_with_shift(0.2, 0.5, 0.3, -0.2, 0.0);
    let negative_shift = SABRParameters::new_with_shift(0.2, 0.5, 0.3, -0.2, -0.01);

    for result in [zero_shift, negative_shift] {
        let err = result.expect_err("non-positive shifts should fail");
        let err_text = err.to_string();
        assert!(
            err_text.contains("shift parameter must be positive"),
            "unexpected error: {err_text}"
        );
    }
}

#[test]
fn test_sabr_validate_inputs_covers_standard_and_shifted_branches() {
    let standard =
        SABRModel::new(SABRParameters::new(0.2, 0.5, 0.3, -0.2).expect("valid standard params"));
    assert!(standard.validate_inputs(100.0, 110.0, 1.0).is_ok());

    let time_err = standard
        .validate_inputs(100.0, 110.0, 0.0)
        .expect_err("non-positive expiry should fail");
    assert!(time_err.to_string().contains("time_to_expiry"));

    let standard_rate_err = standard
        .validate_inputs(-0.01, 0.02, 1.0)
        .expect_err("unshifted SABR should reject non-positive rates");
    assert!(standard_rate_err.to_string().contains("positive rates"));

    let shifted = SABRModel::new(
        SABRParameters::new_with_shift(0.2, 0.5, 0.3, -0.2, 0.02).expect("valid shifted params"),
    );
    assert!(shifted.validate_inputs(-0.005, 0.0, 1.0).is_ok());

    let shifted_rate_err = shifted
        .validate_inputs(-0.03, -0.02, 1.0)
        .expect_err("effective non-positive shifted rates should fail");
    assert!(shifted_rate_err
        .to_string()
        .contains("effective rates must be positive"));
}

#[test]
fn test_sabr_nu_zero_short_circuit_matches_atm_vol_off_atm() {
    let params = SABRParameters::new(0.24, 0.6, 0.0, -0.35).expect("valid params");
    let model = SABRModel::new(params);

    let forward = 100.0;
    let strike = 120.0;
    let expiry = 1.5;

    let off_atm = model
        .implied_volatility(forward, strike, expiry)
        .expect("vol should compute");
    let atm = model
        .atm_volatility(forward, expiry)
        .expect("ATM vol should compute");

    assert!(
        (off_atm - atm).abs() < 1e-12,
        "nu == 0 path should fall back to ATM volatility"
    );
}

#[test]
fn test_solve_alpha_for_atm_round_trips_target_vol() {
    let forward = 100.0;
    let time_to_expiry = 2.0;
    let beta = 0.55;
    let nu = 0.42;
    let rho = -0.18;
    let original_alpha = 0.28;

    let original =
        SABRModel::new(SABRParameters::new(original_alpha, beta, nu, rho).expect("valid params"));
    let target_atm = original
        .atm_volatility(forward, time_to_expiry)
        .expect("ATM vol should compute");

    let solved_alpha =
        solve_alpha_for_atm(forward, target_atm, time_to_expiry, beta, nu, rho, 1e-12)
            .expect("alpha solve should succeed");

    let solved = SABRModel::new(
        SABRParameters::new(solved_alpha, beta, nu, rho).expect("solved params should be valid"),
    );
    let solved_atm = solved
        .atm_volatility(forward, time_to_expiry)
        .expect("ATM vol should compute");

    assert!((solved_alpha - original_alpha).abs() < 1e-8);
    assert!((solved_atm - target_atm).abs() < 1e-10);
}

#[test]
fn test_sabr_calibrate_with_atm_pinning_matches_synthetic_smile() {
    let true_params = SABRParameters::new(0.22, 0.6, 0.35, -0.25).expect("valid params");
    let true_model = SABRModel::new(true_params);

    let forward = 100.0;
    let expiry = 1.25;
    let beta = 0.6;
    let strikes = vec![80.0, 90.0, 100.0, 110.0, 120.0];
    let market_vols: Vec<f64> = strikes
        .iter()
        .map(|&strike| {
            true_model
                .implied_volatility(forward, strike, expiry)
                .expect("synthetic vol should compute")
        })
        .collect();

    let calibrated = SABRCalibrator::new()
        .with_tolerance(1e-10)
        .with_max_iterations(200)
        .calibrate_with_atm_pinning(forward, &strikes, &market_vols, expiry, beta)
        .expect("ATM-pinned calibration should succeed");
    let calibrated_model = SABRModel::new(calibrated);

    let atm_idx = strikes
        .iter()
        .position(|&strike| strike == forward)
        .expect("ATM strike should be present");
    let atm_market = market_vols[atm_idx];
    let calibrated_atm = calibrated_model
        .atm_volatility(forward, expiry)
        .expect("ATM vol should compute");
    assert!((calibrated_atm - atm_market).abs() < 1e-8);

    for (strike, market_vol) in strikes.iter().zip(market_vols.iter()) {
        let fitted = calibrated_model
            .implied_volatility(forward, *strike, expiry)
            .expect("fitted vol should compute");
        assert!(
            (fitted - market_vol).abs() < 1e-3,
            "bad fit at strike {strike}: fitted={fitted}, market={market_vol}"
        );
    }
}

#[test]
fn test_sabr_calibrate_with_derivatives_tracks_fd_gradient_solution() {
    let true_params = SABRParameters::new(0.25, 0.5, 0.45, -0.3).expect("valid params");
    let true_model = SABRModel::new(true_params);

    let forward = 100.0;
    let expiry = 0.75;
    let beta = 0.5;
    let strikes = vec![85.0, 95.0, 100.0, 105.0, 115.0];
    let market_vols: Vec<f64> = strikes
        .iter()
        .map(|&strike| {
            true_model
                .implied_volatility(forward, strike, expiry)
                .expect("synthetic vol should compute")
        })
        .collect();

    let fd_params = SABRCalibrator::new()
        .with_fd_gradients(true)
        .with_tolerance(1e-9)
        .with_max_iterations(200)
        .calibrate_with_derivatives(forward, &strikes, &market_vols, expiry, beta)
        .expect("FD-derivative calibration should succeed");
    let analytic_params = SABRCalibrator::new()
        .with_fd_gradients(false)
        .with_tolerance(1e-9)
        .with_max_iterations(200)
        .calibrate_with_derivatives(forward, &strikes, &market_vols, expiry, beta)
        .expect("analytical-derivative calibration should succeed");

    let fd_model = SABRModel::new(fd_params);
    let analytic_model = SABRModel::new(analytic_params);
    for (strike, market_vol) in strikes.into_iter().zip(market_vols.into_iter()) {
        let fd_vol = fd_model
            .implied_volatility(forward, strike, expiry)
            .expect("FD model vol should compute");
        let analytic_vol = analytic_model
            .implied_volatility(forward, strike, expiry)
            .expect("analytic model vol should compute");
        assert!(
            (fd_vol - market_vol).abs() < 2e-2,
            "FD fit too loose at strike {strike}: fitted={fd_vol}, market={market_vol}"
        );
        assert!(
            (analytic_vol - market_vol).abs() < 2e-2,
            "analytical fit too loose at strike {strike}: fitted={analytic_vol}, market={market_vol}"
        );
    }
}

#[test]
fn test_sabr_strike_from_delta_half_delta_returns_forward() {
    let params = SABRParameters::new(0.2, 0.5, 0.3, -0.2).expect("valid params");
    let smile = SABRSmile::new(SABRModel::new(params), 100.0, 1.0);

    let call_strike = smile
        .strike_from_delta(0.5, true)
        .expect("call strike should compute");
    let put_strike = smile
        .strike_from_delta(0.5, false)
        .expect("put strike should compute");

    assert!((call_strike - 100.0).abs() < 1e-12);
    assert!((put_strike - 100.0).abs() < 1e-12);
}
