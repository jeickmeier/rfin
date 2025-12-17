//! Tests for CalibrationSpec pipeline execution.
//!
//! Validates that calibration specs can be constructed with quotes and executed.
//! Note: Trivial struct construction tests have been removed as Rust's type system
//! guarantees correct construction.

use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount, Tenor};
use finstack_core::market_data::term_structures::Seniority;
use finstack_valuations::calibration::methods::discount::DiscountCurveCalibrator;
use finstack_valuations::calibration::methods::hazard_curve::HazardCurveCalibrator;
use finstack_valuations::calibration::quotes::InstrumentConventions;
use finstack_valuations::calibration::{
    CalibrationConfig, CalibrationSpec, CalibrationStep, RatesQuote,
};

#[test]
fn test_calibration_spec_execute_empty() {
    let base_date = Date::from_calendar_date(2024, time::Month::January, 1).unwrap();

    let spec = CalibrationSpec {
        base_date,
        base_currency: Currency::USD,
        config: CalibrationConfig::default(),
        steps: vec![],
        schema_version: 1,
    };

    let result = spec.execute(None);

    // Market-standard behavior: an empty pipeline is invalid.
    // With default `ValidationMode::Error`, the pipeline fails fast.
    let err = result.expect_err("empty pipeline should fail");
    assert!(matches!(err, finstack_core::Error::Calibration { .. }));
}

#[test]
fn test_calibration_spec_with_quotes() {
    let base_date = Date::from_calendar_date(2024, time::Month::January, 1).unwrap();

    let quotes = vec![
        RatesQuote::Deposit {
            maturity: base_date + time::Duration::days(30),
            rate: 0.045,
            conventions: InstrumentConventions::default().with_day_count(DayCount::Act360),
        },
        RatesQuote::Swap {
            maturity: base_date + time::Duration::days(365),
            rate: 0.046,
            is_ois: true,
            conventions: Default::default(),
            fixed_leg_conventions: InstrumentConventions::default()
                .with_payment_frequency(Tenor::annual())
                .with_day_count(DayCount::Act365F),
            float_leg_conventions: InstrumentConventions::default()
                .with_payment_frequency(Tenor::daily())
                .with_day_count(DayCount::Act365F)
                .with_index("USD-OIS"),
        },
    ];

    let discount_step = CalibrationStep::Discount {
        calibrator: DiscountCurveCalibrator::new("USD-OIS", base_date, Currency::USD),
        quotes,
    };

    let spec = CalibrationSpec {
        base_date,
        base_currency: Currency::USD,
        config: CalibrationConfig::default(),
        steps: vec![discount_step],
        schema_version: 1,
    };

    // Verify spec can be serialized (important for JSON pipelines)
    let json = serde_json::to_string(&spec);
    assert!(json.is_ok(), "CalibrationSpec should serialize to JSON");
}

#[test]
fn test_calibration_spec_multiple_hazard_steps() {
    let base_date = Date::from_calendar_date(2024, time::Month::January, 1).unwrap();

    let discount_step = CalibrationStep::Discount {
        calibrator: DiscountCurveCalibrator::new("USD-OIS", base_date, Currency::USD),
        quotes: vec![RatesQuote::Deposit {
            maturity: base_date + time::Duration::days(365),
            rate: 0.045,
            conventions: InstrumentConventions::default().with_day_count(DayCount::Act360),
        }],
    };

    let hazard_step1 = CalibrationStep::Hazard {
        calibrator: HazardCurveCalibrator::new(
            "CORP_1",
            Seniority::Senior,
            0.40,
            base_date,
            Currency::USD,
            "USD-OIS",
        ),
        quotes: vec![],
    };

    let hazard_step2 = CalibrationStep::Hazard {
        calibrator: HazardCurveCalibrator::new(
            "CORP_2",
            Seniority::Subordinated,
            0.35,
            base_date,
            Currency::USD,
            "USD-OIS",
        ),
        quotes: vec![],
    };

    let spec = CalibrationSpec {
        base_date,
        base_currency: Currency::USD,
        config: CalibrationConfig::default(),
        steps: vec![discount_step, hazard_step1, hazard_step2],
        schema_version: 1,
    };

    // Verify multi-step spec construction and serialization
    assert_eq!(spec.steps.len(), 3);
    let json = serde_json::to_string(&spec);
    assert!(
        json.is_ok(),
        "Multi-step CalibrationSpec should serialize to JSON"
    );
}

#[test]
#[cfg(feature = "serde")]
fn test_calibration_spec_serde_roundtrip() {
    let base_date = Date::from_calendar_date(2024, time::Month::January, 1).unwrap();

    let spec = CalibrationSpec {
        base_date,
        base_currency: Currency::USD,
        config: CalibrationConfig {
            tolerance: 1e-12,
            max_iterations: 200,
            ..Default::default()
        },
        steps: vec![CalibrationStep::Discount {
            calibrator: DiscountCurveCalibrator::new("USD-OIS", base_date, Currency::USD),
            quotes: vec![RatesQuote::Deposit {
                maturity: base_date + time::Duration::days(90),
                rate: 0.045,
                conventions: InstrumentConventions::default().with_day_count(DayCount::Act360),
            }],
        }],
        schema_version: 1,
    };

    let json = serde_json::to_string(&spec).unwrap();
    let deserialized: CalibrationSpec = serde_json::from_str(&json).unwrap();

    // Verify key fields survived roundtrip
    assert_eq!(deserialized.base_date, spec.base_date);
    assert_eq!(deserialized.base_currency, spec.base_currency);
    assert_eq!(deserialized.config.tolerance, spec.config.tolerance);
    assert_eq!(deserialized.steps.len(), spec.steps.len());
}
