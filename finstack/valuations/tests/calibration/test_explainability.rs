//! Tests for explainability features in calibration.

use finstack_core::currency::Currency;
use finstack_core::dates::{create_date, DayCount, Frequency};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::math::interp::InterpStyle;
use finstack_valuations::calibration::methods::ForwardCurveCalibrator;
use finstack_valuations::calibration::{CalibrationConfig, Calibrator, RatesQuote};
use time::Month;

#[test]
fn test_jacobian_not_computed_by_default() {
    // Create a simple discount curve
    let base_date = create_date(2025, Month::January, 15).unwrap();
    let discount_curve = DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .knots(vec![
            (0.0, 1.0),
            (0.25, 0.9888),
            (0.5, 0.9775),
            (1.0, 0.9550),
            (2.0, 0.9100),
        ])
        .set_interp(InterpStyle::MonotoneConvex)
        .build()
        .unwrap();

    let context = MarketContext::new().insert_discount(discount_curve);

    // Create some test quotes
    let quotes = vec![
        RatesQuote::FRA {
            start: create_date(2025, Month::April, 15).unwrap(),
            end: create_date(2025, Month::July, 15).unwrap(),
            rate: 0.045,
            day_count: DayCount::Act360,
        },
        RatesQuote::FRA {
            start: create_date(2025, Month::July, 15).unwrap(),
            end: create_date(2025, Month::October, 15).unwrap(),
            rate: 0.046,
            day_count: DayCount::Act360,
        },
    ];

    let calibrator =
        ForwardCurveCalibrator::new("USD-SOFR-3M", 0.25, base_date, Currency::USD, "USD-OIS");

    let (_curve, report) = calibrator.calibrate(&quotes, &context).unwrap();

    // Explanation should be None by default
    assert!(report.explanation.is_none());
}

#[test]
fn test_jacobian_computed_when_enabled() {
    let base_date = create_date(2025, Month::January, 15).unwrap();
    let discount_curve = DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .knots(vec![
            (0.0, 1.0),
            (0.25, 0.9888),
            (0.5, 0.9775),
            (1.0, 0.9550),
            (2.0, 0.9100),
        ])
        .set_interp(InterpStyle::MonotoneConvex)
        .build()
        .unwrap();

    let context = MarketContext::new().insert_discount(discount_curve);

    let quotes = vec![
        RatesQuote::FRA {
            start: create_date(2025, Month::April, 15).unwrap(),
            end: create_date(2025, Month::July, 15).unwrap(),
            rate: 0.045,
            day_count: DayCount::Act360,
        },
        RatesQuote::FRA {
            start: create_date(2025, Month::July, 15).unwrap(),
            end: create_date(2025, Month::October, 15).unwrap(),
            rate: 0.046,
            day_count: DayCount::Act360,
        },
    ];

    // Enable explanation
    let config = CalibrationConfig::default().with_explain();

    let calibrator =
        ForwardCurveCalibrator::new("USD-SOFR-3M", 0.25, base_date, Currency::USD, "USD-OIS")
            .with_config(config);

    let (_curve, report) = calibrator.calibrate(&quotes, &context).unwrap();

    // Explanation should be present
    assert!(report.explanation.is_some());

    let trace = report.explanation.unwrap();
    assert_eq!(trace.trace_type, "forward_curve_calibration");

    // Find Jacobian entry
    let jacobian = trace.entries.iter().find_map(|entry| match entry {
        finstack_core::explain::TraceEntry::Jacobian {
            row_labels,
            col_labels,
            sensitivity_matrix,
        } => Some((row_labels, col_labels, sensitivity_matrix)),
        _ => None,
    });

    assert!(jacobian.is_some());
    let (row_labels, col_labels, matrix) = jacobian.unwrap();

    // Should have 2 instruments (rows)
    assert_eq!(row_labels.len(), 2);
    assert_eq!(matrix.len(), 2);

    // Should have curve points (columns)
    assert!(!col_labels.is_empty());

    // Each row should have same number of columns
    for row in matrix {
        assert_eq!(row.len(), col_labels.len());
    }
}

#[test]
fn test_jacobian_sensitivities_nonzero() {
    let base_date = create_date(2025, Month::January, 15).unwrap();
    let discount_curve = DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .knots(vec![
            (0.0, 1.0),
            (0.25, 0.9888),
            (0.5, 0.9775),
            (1.0, 0.9550),
            (2.0, 0.9100),
        ])
        .set_interp(InterpStyle::MonotoneConvex)
        .build()
        .unwrap();

    let context = MarketContext::new().insert_discount(discount_curve);

    let quotes = vec![
        RatesQuote::Swap {
            maturity: create_date(2026, Month::January, 15).unwrap(),
            rate: 0.045,
            fixed_freq: Frequency::semi_annual(),
            float_freq: Frequency::quarterly(),
            fixed_dc: DayCount::Thirty360,
            float_dc: DayCount::Act360,
            index: "USD-SOFR-3M".into(),
        },
        RatesQuote::Swap {
            maturity: create_date(2027, Month::January, 15).unwrap(),
            rate: 0.046,
            fixed_freq: Frequency::semi_annual(),
            float_freq: Frequency::quarterly(),
            fixed_dc: DayCount::Thirty360,
            float_dc: DayCount::Act360,
            index: "USD-SOFR-3M".into(),
        },
    ];

    let config = CalibrationConfig::default().with_explain();

    let calibrator =
        ForwardCurveCalibrator::new("USD-SOFR-3M", 0.25, base_date, Currency::USD, "USD-OIS")
            .with_config(config);

    let (_curve, report) = calibrator.calibrate(&quotes, &context).unwrap();

    let trace = report.explanation.unwrap();
    let jacobian = trace.entries.iter().find_map(|entry| match entry {
        finstack_core::explain::TraceEntry::Jacobian {
            sensitivity_matrix, ..
        } => Some(sensitivity_matrix),
        _ => None,
    });

    assert!(jacobian.is_some());
    let matrix = jacobian.unwrap();

    // Check sensitivities are in meaningful range (|val| > 1e-4 is economically significant)
    let meaningful_count = matrix
        .iter()
        .flat_map(|row| row.iter())
        .filter(|&&val| val.abs() > 1e-4)
        .count();

    assert!(
        meaningful_count > 0,
        "Expected at least one meaningful sensitivity (|val| > 1e-4)"
    );

    // Check no exploding sensitivities
    let max_sensitivity = matrix
        .iter()
        .flat_map(|row| row.iter())
        .map(|v| v.abs())
        .fold(0.0_f64, f64::max);

    assert!(
        max_sensitivity < 1e6,
        "Sensitivities should not explode: max={:.2e}",
        max_sensitivity
    );
}
