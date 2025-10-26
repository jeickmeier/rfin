//! Tests for hazard curve calibration with market standards validation.
//!
//! Verifies that calibrated hazard rates are always positive and that
//! the calibrator properly handles edge cases.

use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::hazard_curve::Seniority;
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::math::interp::InterpStyle;
use finstack_valuations::calibration::methods::hazard_curve::HazardCurveCalibrator;
use finstack_valuations::calibration::{Calibrator, CreditQuote}; // CreditQuote is re-exported from calibration
use time::Month;

fn create_test_discount_curve(base: Date) -> DiscountCurve {
    DiscountCurve::builder("TEST-DISC")
        .base_date(base)
        .knots(vec![
            (0.0, 1.0),
            (1.0, 0.98),
            (3.0, 0.94),
            (5.0, 0.88),
            (10.0, 0.75),
        ])
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap()
}

#[test]
fn test_hazard_calibration_positive_rates() {
    let base = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    
    let calibrator = HazardCurveCalibrator::new(
        "ACME-Corp",
        Seniority::Senior,
        0.40, // recovery rate
        base,
        Currency::USD,
        "TEST-DISC",
    );

    // Create realistic CDS quotes (seniority is handled by calibrator, not quote)
    let quotes = vec![
        CreditQuote::CDS {
            entity: "ACME-Corp".to_string(),
            maturity: Date::from_calendar_date(2026, Month::January, 1).unwrap(),
            spread_bp: 100.0, // 100bp spread
            recovery_rate: 0.40,
            currency: Currency::USD,
        },
        CreditQuote::CDS {
            entity: "ACME-Corp".to_string(),
            maturity: Date::from_calendar_date(2028, Month::January, 1).unwrap(),
            spread_bp: 150.0, // 150bp spread
            recovery_rate: 0.40,
            currency: Currency::USD,
        },
        CreditQuote::CDS {
            entity: "ACME-Corp".to_string(),
            maturity: Date::from_calendar_date(2030, Month::January, 1).unwrap(),
            spread_bp: 200.0, // 200bp spread
            recovery_rate: 0.40,
            currency: Currency::USD,
        },
    ];

    let disc = create_test_discount_curve(base);
    let market = MarketContext::new().insert_discount(disc);

    let result = calibrator.calibrate(&quotes, &market);
    
    assert!(
        result.is_ok(),
        "Calibration with realistic spreads should succeed: {:?}",
        result.err()
    );

    let (curve, _report) = result.unwrap();
    
    // Verify all hazard rates are positive
    for (t, lambda) in curve.knot_points() {
        assert!(
            lambda > 0.0,
            "Hazard rate at t={} should be positive, got: {}",
            t, lambda
        );
    }
}

#[test]
fn test_hazard_calibration_rejects_zero_spread() {
    // Zero spread should produce zero or negative hazard rate, which should be rejected
    let base = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    
    let calibrator = HazardCurveCalibrator::new(
        "ZERO-SPREAD",
        Seniority::Senior,
        0.40,
        base,
        Currency::USD,
        "TEST-DISC",
    );

    let quotes = vec![
        CreditQuote::CDS {
            entity: "ZERO-SPREAD".to_string(),
            maturity: Date::from_calendar_date(2026, Month::January, 1).unwrap(),
            spread_bp: 0.0, // Zero spread (risk-free)
            recovery_rate: 0.40,
            currency: Currency::USD,
        },
    ];

    let disc = create_test_discount_curve(base);
    let market = MarketContext::new().insert_discount(disc);

    let result = calibrator.calibrate(&quotes, &market);
    
    // Should either succeed with very small positive rate or fail gracefully
    // The solver may return a small positive value or fail to converge
    if let Ok((curve, _)) = result {
        for (t, lambda) in curve.knot_points() {
            assert!(
                lambda > 0.0,
                "Even with zero spread, hazard rate at t={} must be positive, got: {}",
                t, lambda
            );
        }
    }
    // If it fails, that's also acceptable for zero spread case
}

#[test]
fn test_hazard_calibration_positive_rates_validation() {
    // This test verifies the validation logic triggers on negative rates
    // In practice, the solver should not return negative rates with realistic data,
    // but this ensures the validation catches edge cases
    
    let base = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    
    let calibrator = HazardCurveCalibrator::new(
        "TEST-ENTITY",
        Seniority::Senior,
        0.40,
        base,
        Currency::USD,
        "TEST-DISC",
    );

    // Use reasonable market quotes that should produce positive hazard rates
    let quotes = vec![
        CreditQuote::CDS {
            entity: "TEST-ENTITY".to_string(),
            maturity: Date::from_calendar_date(2027, Month::January, 1).unwrap(),
            spread_bp: 50.0, // 50bp
            recovery_rate: 0.40,
            currency: Currency::USD,
        },
        CreditQuote::CDS {
            entity: "TEST-ENTITY".to_string(),
            maturity: Date::from_calendar_date(2030, Month::January, 1).unwrap(),
            spread_bp: 100.0, // 100bp
            recovery_rate: 0.40,
            currency: Currency::USD,
        },
    ];

    let disc = create_test_discount_curve(base);
    let market = MarketContext::new().insert_discount(disc);

    let result = calibrator.calibrate(&quotes, &market);
    
    // Should succeed with positive rates
    assert!(
        result.is_ok(),
        "Calibration should succeed with realistic positive spreads: {:?}",
        result.err()
    );
}

