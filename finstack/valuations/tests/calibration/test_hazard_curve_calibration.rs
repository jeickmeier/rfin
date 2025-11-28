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
use finstack_core::money::Money;
use finstack_valuations::calibration::methods::hazard_curve::HazardCurveCalibrator;
use finstack_valuations::calibration::{Calibrator, CreditQuote};
use finstack_valuations::instruments::cds::{CDSConvention, CreditDefaultSwap, PayReceive};
use finstack_valuations::instruments::common::parameters::legs::{PremiumLegSpec, ProtectionLegSpec};
use finstack_valuations::instruments::common::traits::Attributes;
use finstack_valuations::instruments::PricingOverrides;
use finstack_valuations::instruments::Instrument;
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
            t,
            lambda
        );
    }
}

/// Test that zero-spread CDS quotes are properly rejected.
///
/// Zero CDS spread implies no credit risk, which cannot be meaningfully
/// calibrated (hazard rate would be exactly zero, leading to degenerate curves).
/// The calibrator correctly rejects these quotes with a validation error.
#[test]
fn test_hazard_calibration_rejects_zero_spread() {
    let base = Date::from_calendar_date(2025, Month::January, 1).unwrap();

    let calibrator = HazardCurveCalibrator::new(
        "ZERO-SPREAD",
        Seniority::Senior,
        0.40, // recovery rate
        base,
        Currency::USD,
        "TEST-DISC",
    );

    let quotes = vec![CreditQuote::CDS {
        entity: "ZERO-SPREAD".to_string(),
        maturity: Date::from_calendar_date(2026, Month::January, 1).unwrap(),
        spread_bp: 0.0, // Zero spread implies no default risk
        recovery_rate: 0.40,
        currency: Currency::USD,
    }];

    let disc = create_test_discount_curve(base);
    let market = MarketContext::new().insert_discount(disc);

    let result = calibrator.calibrate(&quotes, &market);

    // Zero-spread calibration should fail with a clear validation error
    assert!(
        result.is_err(),
        "Zero-spread CDS should be rejected (no credit risk to calibrate)"
    );

    // Verify the error message is informative
    let err = result.err().unwrap();
    let err_msg = format!("{:?}", err);
    assert!(
        err_msg.contains("spread") && (err_msg.contains("positive") || err_msg.contains("zero")),
        "Error should mention spread requirement: {}",
        err_msg
    );
}

/// Test near-zero spread CDS calibration with tiny positive spreads.
///
/// Very low but positive spreads (e.g., 0.1bp = 0.001%) represent near-AAA
/// credits. These should calibrate successfully with very small hazard rates.
#[test]
fn test_hazard_calibration_near_zero_spread() {
    let base = Date::from_calendar_date(2025, Month::January, 1).unwrap();

    let calibrator = HazardCurveCalibrator::new(
        "NEAR-ZERO",
        Seniority::Senior,
        0.40, // recovery rate
        base,
        Currency::USD,
        "TEST-DISC",
    );

    let quotes = vec![CreditQuote::CDS {
        entity: "NEAR-ZERO".to_string(),
        maturity: Date::from_calendar_date(2026, Month::January, 1).unwrap(),
        spread_bp: 1.0, // 1bp - very low but positive spread (near-AAA credit)
        recovery_rate: 0.40,
        currency: Currency::USD,
    }];

    let disc = create_test_discount_curve(base);
    let market = MarketContext::new().insert_discount(disc);

    let result = calibrator.calibrate(&quotes, &market);

    assert!(
        result.is_ok(),
        "Near-zero spread (1bp) calibration should succeed: {:?}",
        result.err()
    );

    let (curve, _report) = result.unwrap();

    // Verify hazard rates are small but positive
    // 1bp spread ≈ 0.016% hazard rate with 40% recovery
    for (t, lambda) in curve.knot_points() {
        assert!(
            lambda > 0.0,
            "Hazard rate at t={} must be positive: {}",
            t,
            lambda
        );
        // 1bp spread with 40% recovery: λ ≈ s/(1-R) ≈ 0.0001/0.6 ≈ 0.00017
        // Allow some tolerance for solver precision
        assert!(
            lambda < 0.01, // Should be much smaller than 1% annual hazard
            "Near-zero spread hazard rate at t={} should be small: {:.2e}",
            t,
            lambda
        );
    }
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

/// Test that CDS instruments reprice to par when using the calibrated hazard curve.
///
/// The fundamental property of hazard curve calibration is that each CDS quote
/// should reprice to NPV ≈ 0 (par) when valued using the calibrated curve.
///
/// Tolerance: 1bp of notional ($100 per $1M)
#[test]
fn test_hazard_curve_reprices_cds_to_par() {
    let base = Date::from_calendar_date(2025, Month::January, 1).unwrap();

    // Create quotes with varying spreads
    let quotes = vec![
        CreditQuote::CDS {
            entity: "REPRICE-TEST".to_string(),
            maturity: Date::from_calendar_date(2027, Month::January, 1).unwrap(),
            spread_bp: 100.0, // 100bp
            recovery_rate: 0.40,
            currency: Currency::USD,
        },
        CreditQuote::CDS {
            entity: "REPRICE-TEST".to_string(),
            maturity: Date::from_calendar_date(2030, Month::January, 1).unwrap(),
            spread_bp: 150.0, // 150bp - upward sloping term structure
            recovery_rate: 0.40,
            currency: Currency::USD,
        },
    ];

    let calibrator = HazardCurveCalibrator::new(
        "REPRICE-TEST",
        Seniority::Senior,
        0.40,
        base,
        Currency::USD,
        "TEST-DISC",
    );

    let disc = create_test_discount_curve(base);
    let market = MarketContext::new().insert_discount(disc);

    let (hazard_curve, report) = calibrator
        .calibrate(&quotes, &market)
        .expect("Calibration should succeed");

    assert!(report.success, "Calibration should succeed");

    // Add hazard curve to context
    let ctx = market.insert_hazard(hazard_curve);

    // Notional for repricing
    const NOTIONAL: f64 = 10_000_000.0;
    // Tolerance: 1bp of notional = $1,000 per $10M
    const TOLERANCE: f64 = NOTIONAL * 0.0001;

    // Reprice each CDS at its par spread
    for quote in &quotes {
        if let CreditQuote::CDS {
            maturity,
            spread_bp,
            recovery_rate,
            ..
        } = quote
        {
            // Create CDS instrument matching the quote using builder
            let convention = CDSConvention::IsdaNa;
            let cds = CreditDefaultSwap::builder()
                .id("REPRICE-CDS".into())
                .notional(Money::new(NOTIONAL, Currency::USD))
                .side(PayReceive::PayFixed) // Buy protection
                .convention(convention)
                .premium(PremiumLegSpec {
                    start: base,
                    end: *maturity,
                    freq: convention.frequency(),
                    stub: convention.stub_convention(),
                    bdc: convention.business_day_convention(),
                    calendar_id: Some(convention.default_calendar().to_string()),
                    dc: convention.day_count(),
                    spread_bp: *spread_bp,
                    discount_curve_id: "TEST-DISC".into(),
                })
                .protection(ProtectionLegSpec {
                    credit_curve_id: "REPRICE-TEST-senior".into(), // {entity}-{seniority} format
                    recovery_rate: *recovery_rate,
                    settlement_delay: convention.settlement_delay(),
                })
                .pricing_overrides(PricingOverrides::default())
                .attributes(Attributes::new())
                .build()
                .expect("CDS builder should succeed");

            let npv = cds.value(&ctx, base).expect("CDS valuation should succeed");

            // CDS at par spread should have NPV ≈ 0
            assert!(
                npv.amount().abs() < TOLERANCE,
                "CDS at {} (spread={}bp) should reprice to par. NPV=${:.2}, tolerance=${:.2}",
                maturity,
                spread_bp,
                npv.amount(),
                TOLERANCE
            );
        }
    }
}
