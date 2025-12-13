//! Determinism tests for curve calibration.
//!
//! Verifies that calibration processes (which involve iterative solvers)
//! produce bitwise-identical results with fixed inputs, and validates
//! calibration quality against market standards.

use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount, Tenor};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::{DiscountCurve, Seniority};
use finstack_core::math::interp::InterpStyle;
use finstack_valuations::calibration::methods::discount::DiscountCurveCalibrator;
use finstack_valuations::calibration::methods::hazard_curve::HazardCurveCalibrator;
use finstack_valuations::calibration::{CalibrationMethod, Calibrator, CreditQuote, RatesQuote};
use time::Month;

fn create_test_quotes() -> Vec<CreditQuote> {
    vec![
        CreditQuote::CDS {
            entity: "TEST-ENTITY".to_string(),
            maturity: Date::from_calendar_date(2026, Month::January, 15).unwrap(),
            spread_bp: 100.0,
            recovery_rate: 0.40,
            currency: Currency::USD,
        },
        CreditQuote::CDS {
            entity: "TEST-ENTITY".to_string(),
            maturity: Date::from_calendar_date(2028, Month::January, 15).unwrap(),
            spread_bp: 150.0,
            recovery_rate: 0.40,
            currency: Currency::USD,
        },
        CreditQuote::CDS {
            entity: "TEST-ENTITY".to_string(),
            maturity: Date::from_calendar_date(2030, Month::January, 15).unwrap(),
            spread_bp: 200.0,
            recovery_rate: 0.40,
            currency: Currency::USD,
        },
    ]
}

fn create_calibration_market(base_date: Date) -> MarketContext {
    let disc = DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .knots([
            (0.0, 1.0),
            (1.0, 0.98),
            (3.0, 0.94),
            (5.0, 0.88),
            (10.0, 0.70),
        ])
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap();

    MarketContext::new().insert_discount(disc)
}

#[test]
fn test_hazard_curve_calibration_determinism() {
    let base = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    let quotes = create_test_quotes();
    let market = create_calibration_market(base);

    let calibrator = HazardCurveCalibrator::new(
        "TEST-ENTITY",
        Seniority::Senior,
        0.40,
        base,
        Currency::USD,
        "USD-OIS",
    );

    // Calibrate the curve 20 times
    let curves: Vec<_> = (0..20)
        .map(|_| {
            calibrator
                .calibrate(&quotes, &market)
                .expect("Calibration should succeed")
                .0 // Extract the curve from (curve, report) tuple
        })
        .collect();

    // Verify all curves have identical knot points
    let first_knots: Vec<(f64, f64)> = curves[0].knot_points().collect();

    for (i, curve) in curves.iter().enumerate().skip(1) {
        let knots: Vec<(f64, f64)> = curve.knot_points().collect();

        assert_eq!(
            knots.len(),
            first_knots.len(),
            "Calibration {} produced different number of knots",
            i
        );

        for (j, ((t1, lambda1), (t2, lambda2))) in first_knots.iter().zip(knots.iter()).enumerate()
        {
            assert_eq!(
                *t1, *t2,
                "Knot {} time differs at calibration {}: {:.15} vs {:.15}",
                j, i, t1, t2
            );
            assert_eq!(
                *lambda1, *lambda2,
                "Knot {} hazard rate differs at calibration {}: {:.15} vs {:.15}",
                j, i, lambda1, lambda2
            );
        }
    }

    // Correctness: Verify calibrated curve produces reasonable survival probabilities
    let sp_0 = curves[0].sp(0.0);
    let sp_5 = curves[0].sp(5.0);

    assert!(
        (sp_0 - 1.0).abs() < 1e-10,
        "Survival probability at t=0 should be 1.0, got {}",
        sp_0
    );
    assert!(
        sp_5 > 0.0 && sp_5 < 1.0,
        "Survival probability at 5Y {} should be in (0, 1)",
        sp_5
    );
}

#[test]
fn test_hazard_curve_survival_probability_determinism() {
    let base = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    let quotes = create_test_quotes();
    let market = create_calibration_market(base);

    let calibrator = HazardCurveCalibrator::new(
        "TEST-ENTITY",
        Seniority::Senior,
        0.40,
        base,
        Currency::USD,
        "USD-OIS",
    );

    // Calibrate 30 times and check survival probabilities
    let test_times = vec![0.5, 1.0, 2.0, 3.0, 5.0];

    for t in test_times {
        let survival_probs: Vec<f64> = (0..30)
            .map(|_| {
                let (curve, _) = calibrator.calibrate(&quotes, &market).unwrap();
                curve.sp(t)
            })
            .collect();

        for i in 1..survival_probs.len() {
            assert_eq!(
                survival_probs[i], survival_probs[0],
                "Survival prob at t={} differs at calibration {}: {:.15} vs {:.15}",
                t, i, survival_probs[i], survival_probs[0]
            );
        }
    }
}

#[test]
fn test_calibration_report_determinism() {
    let base = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    let quotes = create_test_quotes();
    let market = create_calibration_market(base);

    let calibrator = HazardCurveCalibrator::new(
        "TEST-ENTITY",
        Seniority::Senior,
        0.40,
        base,
        Currency::USD,
        "USD-OIS",
    );

    // Calibrate 20 times and check reports
    let reports: Vec<_> = (0..20)
        .map(|_| {
            calibrator
                .calibrate(&quotes, &market)
                .expect("Calibration should succeed")
                .1 // Extract the report from (curve, report) tuple
        })
        .collect();

    // Verify iteration counts are identical
    for i in 1..reports.len() {
        assert_eq!(
            reports[i].iterations, reports[0].iterations,
            "Iteration count differs at calibration {}: {} vs {}",
            i, reports[i].iterations, reports[0].iterations
        );
    }

    // Verify residuals are identical
    for i in 1..reports.len() {
        assert_eq!(
            reports[i].residuals, reports[0].residuals,
            "Residuals differ at calibration {}",
            i
        );
    }

    // Correctness: Verify residuals are small (successful calibration)
    // Use the pre-computed max_residual from the report
    assert!(
        reports[0].max_residual < 1e-8,
        "Calibration max residual {} exceeds tolerance 1e-6",
        reports[0].max_residual
    );

    // Verify iterations are reasonable
    assert!(
        reports[0].iterations > 0 && reports[0].iterations < 100,
        "Iteration count {} outside reasonable range [1, 100)",
        reports[0].iterations
    );
}

#[test]
fn test_discount_curve_global_solve_smoke() {
    let base = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    let quotes = vec![
        RatesQuote::Deposit {
            maturity: Date::from_calendar_date(2025, Month::July, 15).unwrap(),
            rate: 0.03,
            day_count: DayCount::Act360,
            conventions: Default::default(),
        },
        RatesQuote::Deposit {
            maturity: Date::from_calendar_date(2026, Month::January, 15).unwrap(),
            rate: 0.031,
            day_count: DayCount::Act360,
            conventions: Default::default(),
        },
        RatesQuote::Swap {
            maturity: Date::from_calendar_date(2027, Month::January, 15).unwrap(),
            rate: 0.0325,
            fixed_freq: Tenor::annual(),
            float_freq: Tenor::annual(),
            fixed_dc: DayCount::Act360,
            float_dc: DayCount::Act360,
            index: "USD-SOFR".into(),
            conventions: Default::default(),
        },
    ];

    let calibrator = DiscountCurveCalibrator::new("USD-OIS", base, Currency::USD)
        .with_calibration_method(CalibrationMethod::GlobalSolve {
            use_analytical_jacobian: false,
        })
        .with_solve_interp(InterpStyle::PiecewiseQuadraticForward);

    let market = MarketContext::new();
    let (curve, report) = calibrator
        .calibrate(&quotes, &market)
        .expect("Global solve should succeed");

    // Expect knots for each quote plus t=0
    assert_eq!(curve.knots().len(), quotes.len() + 1);
    assert!(
        report.residuals.len() >= quotes.len(),
        "Residuals should track each instrument"
    );
}
