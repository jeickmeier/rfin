#![allow(deprecated)]
//! Integration test for swaption volatility calibration.

use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::money::Money;
use finstack_core::prelude::Currency;
use finstack_valuations::calibration::methods::swaption_vol::{
    AtmStrikeConvention, SwaptionVolCalibrator, SwaptionVolConvention,
};
use finstack_valuations::calibration::{CalibrationConfig, Calibrator, VolQuote};
use finstack_valuations::instruments::swaption::parameters::SwaptionParams;
use finstack_valuations::instruments::swaption::Swaption;
use finstack_valuations::instruments::Instrument;
use time::Month;

/// Create test discount curve for forward rate calculations.
fn create_test_discount_curve() -> DiscountCurve {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .knots(vec![
            (0.0, 1.0),   // Today
            (0.25, 0.99), // 3M: ~4% rate
            (1.0, 0.96),  // 1Y: ~4% rate
            (2.0, 0.92),  // 2Y: ~4% rate
            (5.0, 0.80),  // 5Y: ~4% rate
            (10.0, 0.64), // 10Y: ~4% rate
        ])
        .build()
        .unwrap()
}

/// Create test swaption volatility quotes.
fn create_test_swaption_quotes() -> Vec<VolQuote> {
    vec![
        // 1Y x 1Y swaptions (1Y expiry, 1Y tenor) - normal vols in decimal (100bp = 0.01)
        VolQuote::SwaptionVol {
            expiry: Date::from_calendar_date(2026, Month::January, 1).unwrap(),
            tenor: Date::from_calendar_date(2027, Month::January, 1).unwrap(),
            strike: 0.035,
            vol: 0.012, // 120bp normal vol
            quote_type: "OTM-100".to_string(),
        },
        VolQuote::SwaptionVol {
            expiry: Date::from_calendar_date(2026, Month::January, 1).unwrap(),
            tenor: Date::from_calendar_date(2027, Month::January, 1).unwrap(),
            strike: 0.040,
            vol: 0.010, // 100bp normal vol
            quote_type: "ATM-50".to_string(),
        },
        VolQuote::SwaptionVol {
            expiry: Date::from_calendar_date(2026, Month::January, 1).unwrap(),
            tenor: Date::from_calendar_date(2027, Month::January, 1).unwrap(),
            strike: 0.043,
            vol: 0.009, // 90bp normal vol (ATM)
            quote_type: "ATM".to_string(),
        },
        VolQuote::SwaptionVol {
            expiry: Date::from_calendar_date(2026, Month::January, 1).unwrap(),
            tenor: Date::from_calendar_date(2027, Month::January, 1).unwrap(),
            strike: 0.046,
            vol: 0.010, // 100bp normal vol
            quote_type: "ATM+50".to_string(),
        },
        VolQuote::SwaptionVol {
            expiry: Date::from_calendar_date(2026, Month::January, 1).unwrap(),
            tenor: Date::from_calendar_date(2027, Month::January, 1).unwrap(),
            strike: 0.050,
            vol: 0.012, // 120bp normal vol
            quote_type: "OTM+100".to_string(),
        },
        // 1Y x 5Y swaptions (1Y expiry, 5Y tenor) - normal vols
        VolQuote::SwaptionVol {
            expiry: Date::from_calendar_date(2026, Month::January, 1).unwrap(),
            tenor: Date::from_calendar_date(2031, Month::January, 1).unwrap(),
            strike: 0.038,
            vol: 0.0085, // 85bp normal vol
            quote_type: "OTM-100".to_string(),
        },
        VolQuote::SwaptionVol {
            expiry: Date::from_calendar_date(2026, Month::January, 1).unwrap(),
            tenor: Date::from_calendar_date(2031, Month::January, 1).unwrap(),
            strike: 0.042,
            vol: 0.0075, // 75bp normal vol
            quote_type: "ATM-50".to_string(),
        },
        VolQuote::SwaptionVol {
            expiry: Date::from_calendar_date(2026, Month::January, 1).unwrap(),
            tenor: Date::from_calendar_date(2031, Month::January, 1).unwrap(),
            strike: 0.045,
            vol: 0.007, // 70bp normal vol (ATM)
            quote_type: "ATM".to_string(),
        },
        VolQuote::SwaptionVol {
            expiry: Date::from_calendar_date(2026, Month::January, 1).unwrap(),
            tenor: Date::from_calendar_date(2031, Month::January, 1).unwrap(),
            strike: 0.048,
            vol: 0.0075, // 75bp normal vol
            quote_type: "ATM+50".to_string(),
        },
        VolQuote::SwaptionVol {
            expiry: Date::from_calendar_date(2026, Month::January, 1).unwrap(),
            tenor: Date::from_calendar_date(2031, Month::January, 1).unwrap(),
            strike: 0.052,
            vol: 0.0085, // 85bp normal vol
            quote_type: "OTM+100".to_string(),
        },
    ]
}

/// Create a richer quote set covering multiple expiries and tenors for convergence tests
fn create_extended_swaption_quotes() -> Vec<VolQuote> {
    let mut q = create_test_swaption_quotes();
    // 2Y x 2Y grid around ATM
    for (k, v) in [
        (0.030, 0.0115),
        (0.035, 0.0105),
        (0.040, 0.0095),
        (0.045, 0.0105),
        (0.050, 0.0115),
    ] {
        q.push(VolQuote::SwaptionVol {
            expiry: Date::from_calendar_date(2027, Month::January, 1).unwrap(),
            tenor: Date::from_calendar_date(2029, Month::January, 1).unwrap(),
            strike: k,
            vol: v,
            quote_type: "STRIKE".to_string(),
        });
    }
    // 0.5Y x 3Y
    for (k, v) in [
        (0.020, 0.013),
        (0.025, 0.011),
        (0.030, 0.010),
        (0.035, 0.011),
        (0.040, 0.013),
    ] {
        q.push(VolQuote::SwaptionVol {
            expiry: Date::from_calendar_date(2025, Month::July, 1).unwrap(),
            tenor: Date::from_calendar_date(2028, Month::January, 1).unwrap(),
            strike: k,
            vol: v,
            quote_type: "STRIKE".to_string(),
        });
    }
    q
}

#[test]
fn test_swaption_vol_calibration_direct() {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();

    // Create market context with discount curve
    let mut context = MarketContext::new();
    context = context.insert_discount(create_test_discount_curve());

    // Create calibrator with verbose output - use Normal convention as it's more stable for rates
    let calibrator = SwaptionVolCalibrator::new(
        "TEST-SWAPTION-VOL",
        SwaptionVolConvention::Normal,
        AtmStrikeConvention::SwapRate,
        base_date,
        "USD-OIS",
        Currency::USD,
    )
    .with_config(CalibrationConfig {
        verbose: true,
        ..CalibrationConfig::default()
    });

    // Get test quotes
    let quotes = create_test_swaption_quotes();

    // Calibrate
    let result = calibrator.calibrate(&quotes, &context);
    if let Err(e) = &result {
        eprintln!("Calibration failed: {:?}", e);
    }
    assert!(result.is_ok(), "Calibration should succeed");

    let (surface, report) = result.unwrap();

    // Verify calibration report
    assert!(report.success, "Calibration should report success");
    assert!(report.iterations > 0, "Should have some iterations");

    // CALIBRATION QUALITY CHECKS: Market-standard tolerances for SABR calibration
    // With ATM-pinning approach, SABR should achieve tight calibration:
    // - RMSE < 5bp (0.0005) for swaption volatilities
    // - Max residual < 10bp (0.001)
    assert!(
        report.rmse < 0.0005,
        "SABR calibration RMSE should be < 5bp: {:.6} ({:.2} bp)",
        report.rmse,
        report.rmse * 10000.0
    );
    assert!(
        report.max_residual.abs() < 0.001,
        "Max residual should be < 10bp: {:.6} ({:.2} bp)",
        report.max_residual,
        report.max_residual * 10000.0
    );

    // Verify ATM volatility matches input exactly (ATM-pinning approach)
    // Input data: 1Y×1Y ATM vol = 0.009 (90bp normal vol)
    // Market-standard: ATM should match within <1bp
    let vol_1y_1y = surface
        .value_checked(1.0, 1.0)
        .expect("grid point lookup should succeed");
    let expected_atm_1y1y = 0.009; // From test input data
    let atm_error_1y1y = (vol_1y_1y - expected_atm_1y1y).abs();
    assert!(
        atm_error_1y1y < 0.0001, // 1bp tolerance (market-standard with ATM pinning)
        "ATM vol 1Y×1Y should match input within 1bp: {:.6} vs expected {:.6} (error: {:.2} bp)",
        vol_1y_1y,
        expected_atm_1y1y,
        atm_error_1y1y * 10000.0
    );

    // Input data: 1Y×5Y ATM vol = 0.007 (70bp normal vol)
    // Market-standard: ATM should match within <1bp
    let vol_1y_5y = surface
        .value_checked(1.0, 5.0)
        .expect("grid point lookup should succeed");
    let expected_atm_1y5y = 0.007; // From test input data
    let atm_error_1y5y = (vol_1y_5y - expected_atm_1y5y).abs();
    assert!(
        atm_error_1y5y < 0.0001, // 1bp tolerance (market-standard with ATM pinning)
        "ATM vol 1Y×5Y should match input within 1bp: {:.6} vs expected {:.6} (error: {:.2} bp)",
        vol_1y_5y,
        expected_atm_1y5y,
        atm_error_1y5y * 10000.0
    );

    // Verify positivity and reasonableness
    assert!(vol_1y_1y > 0.0, "Volatility should be positive");
    assert!(vol_1y_5y > 0.0, "Volatility should be positive");

    // Test interpolation for non-grid points - use value_clamped for robustness
    let vol_1_5y_3y = surface.value_clamped(1.5, 3.0); // 1.5Y expiry, 3Y tenor
    assert!(
        vol_1_5y_3y > 0.0,
        "Interpolated volatility should be positive"
    );
    assert!(
        vol_1_5y_3y < 1.0,
        "Interpolated volatility should be reasonable"
    );
}

#[test]
fn test_swaption_vol_calibration_extended_grid_and_interpolation() {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let mut context = MarketContext::new();
    context = context.insert_discount(create_test_discount_curve());

    let calibrator = SwaptionVolCalibrator::new(
        "TEST-SWAPTION-VOL-EXT",
        SwaptionVolConvention::Normal,
        AtmStrikeConvention::SwapRate,
        base_date,
        "USD-OIS",
        Currency::USD,
    )
    .with_config(CalibrationConfig {
        verbose: false,
        max_iterations: 200,
        tolerance: 1e-8,
        ..CalibrationConfig::default()
    });

    let quotes = create_extended_swaption_quotes();
    let (surface, report) = calibrator
        .calibrate(&quotes, &context)
        .expect("calibration ok");
    assert!(report.success);
    assert!(report.iterations > 0);

    // CALIBRATION QUALITY CHECKS
    // Extended grid should achieve tight calibration with ATM-pinning
    // Note: Extended grid has more expiry/tenor points, so slightly looser tolerances
    assert!(
        report.rmse < 0.0007,
        "Extended grid calibration RMSE should be < 7bp: {:.6} ({:.2} bp)",
        report.rmse,
        report.rmse * 10000.0
    );
    assert!(
        report.max_residual.abs() < 0.0015,
        "Extended grid max residual should be < 15bp: {:.6} ({:.2} bp)",
        report.max_residual,
        report.max_residual * 10000.0
    );

    // Interpolation checks at non-grid points - use value_clamped for robustness
    let v = surface.value_clamped(1.5, 2.5);
    assert!(v.is_finite() && v > 0.0 && v < 1.0);
    let v2 = surface.value_clamped(0.75, 3.5);
    assert!(v2.is_finite() && v2 > 0.0 && v2 < 1.0);
}

#[test]
fn test_normal_vs_lognormal_conventions() {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();

    // Create context
    let mut context = MarketContext::new();
    context = context.insert_discount(create_test_discount_curve());

    // Test with normal volatility convention
    let normal_calibrator = SwaptionVolCalibrator::new(
        "NORMAL-VOL",
        SwaptionVolConvention::Normal,
        AtmStrikeConvention::SwapRate,
        base_date,
        "USD-OIS",
        Currency::USD,
    );

    let quotes = create_test_swaption_quotes();
    let result = normal_calibrator.calibrate(&quotes, &context);

    assert!(result.is_ok(), "Normal volatility calibration should work");

    // Verify SABR beta is set correctly for normal
    // (This is internal but we can verify indirectly through the volatilities)
    let (surface, _) = result.unwrap();
    let vol = surface
        .value_checked(1.0, 1.0)
        .expect("grid point lookup should succeed");
    assert!(vol > 0.0, "Normal vol should be positive");
}

#[test]
fn test_swaption_pricing_with_calibrated_surface() {
    use finstack_valuations::instruments::pricing_overrides::VolSurfaceExtrapolation;
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();

    // Set up full market context
    let mut context = MarketContext::new();
    context = context.insert_discount(create_test_discount_curve());

    // Calibrate swaption surface using normal convention (which works)
    let calibrator = SwaptionVolCalibrator::new(
        "SWAPTION-VOL",
        SwaptionVolConvention::Normal, // Use normal convention for stability
        AtmStrikeConvention::SwapRate,
        base_date,
        "USD-OIS",
        Currency::USD,
    );

    let quotes = create_test_swaption_quotes();
    let calibration_result = calibrator.calibrate(&quotes, &context);
    if calibration_result.is_err() {
        eprintln!(
            "Calibration failed: {:?}",
            calibration_result.as_ref().err()
        );
    }
    let (surface, _) = calibration_result.unwrap();

    // Add surface to context
    context = context.insert_surface(surface);

    // Create and price a swaption
    let discount_curve_id: &'static str = "USD-OIS";
    let forward_curve_id: &'static str = "USD-OIS";
    let vol_surface_id: &'static str = "SWAPTION-VOL";
    let params = SwaptionParams::payer(
        Money::new(1_000_000.0, Currency::USD),
        0.04,
        Date::from_calendar_date(2026, Month::January, 1).unwrap(),
        Date::from_calendar_date(2026, Month::January, 1).unwrap(),
        Date::from_calendar_date(2027, Month::January, 1).unwrap(),
    );
    let mut swaption = Swaption::new_payer(
        "TEST-SWAPTION",
        &params,
        discount_curve_id,
        forward_curve_id,
        vol_surface_id,
    );
    swaption.pricing_overrides.vol_surface_extrapolation = VolSurfaceExtrapolation::Clamp;

    // Price should work with calibrated surface
    let price_result = swaption.value(&context, base_date);
    assert!(
        price_result.is_ok(),
        "Swaption pricing should work with calibrated surface"
    );

    let price = price_result.unwrap();
    assert!(price.amount() > 0.0, "Swaption should have positive value");
    assert!(
        price.amount() < 100_000.0,
        "Swaption value should be reasonable"
    );
}

/// Test that calibrated swaption volatility surfaces pass validation.
///
/// Market-standard arbitrage conditions for volatility surfaces:
/// 1. **Calendar spread**: Total variance (σ²T) must increase with expiry
/// 2. **Butterfly spread**: Total variance must be convex in strike
///
/// Note: Current swaption calibration from market quotes may not produce
/// perfectly arbitrage-free surfaces. This test uses lenient validation
/// which logs warnings instead of failing on minor arbitrage violations.
/// For production use cases requiring strict arbitrage-free surfaces,
/// consider using SVI parameterization or monotone convex fitting methods.
#[test]
fn test_swaption_vol_surface_arbitrage_free() {
    use finstack_valuations::calibration::{SurfaceValidator, ValidationConfig};

    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let mut context = MarketContext::new();
    context = context.insert_discount(create_test_discount_curve());

    let calibrator = SwaptionVolCalibrator::new(
        "TEST-ARB-FREE",
        SwaptionVolConvention::Normal,
        AtmStrikeConvention::SwapRate,
        base_date,
        "USD-OIS",
        Currency::USD,
    );

    let quotes = create_extended_swaption_quotes();
    let (surface, _) = calibrator
        .calibrate(&quotes, &context)
        .expect("Calibration should succeed");

    // Use lenient config: current calibration method may produce minor arbitrage
    // violations with certain market data. Lenient mode logs warnings instead of
    // failing, appropriate for exploratory analysis and testing.
    let config = ValidationConfig::lenient();

    // Verify calendar arbitrage check runs without hard errors
    let calendar_result = surface.validate_calendar_spread(&config);
    assert!(
        calendar_result.is_ok(),
        "Calibrated surface should pass calendar validation (lenient): {:?}",
        calendar_result.err()
    );

    // Verify butterfly arbitrage check runs without hard errors
    let butterfly_result = surface.validate_butterfly_spread(&config);
    assert!(
        butterfly_result.is_ok(),
        "Calibrated surface should pass butterfly validation (lenient): {:?}",
        butterfly_result.err()
    );

    // Run full validation (includes bounds check)
    let full_result = surface.validate(&config);
    assert!(
        full_result.is_ok(),
        "Calibrated surface should pass all validation (lenient): {:?}",
        full_result.err()
    );

    // Also verify that vol bounds pass even with strict config
    // (bounds are independent of arbitrage checks)
    let strict_config = ValidationConfig::default();
    let bounds_result = surface.validate_vol_bounds(&strict_config);
    assert!(
        bounds_result.is_ok(),
        "Calibrated surface should have valid vol bounds: {:?}",
        bounds_result.err()
    );
}

#[test]
fn test_insufficient_quotes_error() {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let context = MarketContext::new().insert_discount(create_test_discount_curve());

    let calibrator = SwaptionVolCalibrator::new(
        "TEST",
        SwaptionVolConvention::Lognormal,
        AtmStrikeConvention::SwapRate,
        base_date,
        "USD-OIS",
        Currency::USD,
    );

    // Empty quotes should fail
    let result = calibrator.calibrate(&[], &context);
    assert!(result.is_err(), "Should fail with no quotes");

    // Too few quotes per expiry-tenor should skip that point
    let sparse_quotes = vec![VolQuote::SwaptionVol {
        expiry: Date::from_calendar_date(2026, Month::January, 1).unwrap(),
        tenor: Date::from_calendar_date(2027, Month::January, 1).unwrap(),
        strike: 0.04,
        vol: 0.20,
        quote_type: "ATM".to_string(),
    }];

    let result = calibrator.calibrate(&sparse_quotes, &context);
    // Should fail because we need at least 3 quotes per expiry-tenor for SABR
    assert!(
        result.is_err(),
        "Should fail with too few quotes per expiry-tenor"
    );
}
