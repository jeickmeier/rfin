//! Tests for CalibrationSpec pipeline and basic functionality

use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount, Frequency};
use finstack_valuations::calibration::methods::discount::DiscountCurveCalibrator;
use finstack_valuations::calibration::methods::hazard_curve::HazardCurveCalibrator;
use finstack_valuations::calibration::{
    CalibrationConfig, CalibrationSpec, CalibrationStep, RatesQuote,
};
use finstack_core::market_data::term_structures::Seniority;

#[test]
fn test_calibration_spec_new() {
    let base_date = Date::from_calendar_date(2024, time::Month::January, 1).unwrap();
    let _spec = CalibrationSpec {
        base_date,
        base_currency: Currency::USD,
        config: CalibrationConfig::default(),
        steps: vec![],
        schema_version: 1,
    };

    // Should create without panicking
}

#[test]
fn test_calibration_spec_with_config() {
    let base_date = Date::from_calendar_date(2024, time::Month::January, 1).unwrap();
    let config = CalibrationConfig {
        tolerance: 1e-12,
        max_iterations: 200,
        ..Default::default()
    };

    let _spec = CalibrationSpec {
        base_date,
        base_currency: Currency::USD,
        config,
        steps: vec![],
        schema_version: 1,
    };

    // Should support custom config
}

#[test]
fn test_calibration_spec_with_single_step() {
    let base_date = Date::from_calendar_date(2024, time::Month::January, 1).unwrap();

    let discount_step = CalibrationStep::Discount {
        calibrator: DiscountCurveCalibrator::new("USD-OIS", base_date, Currency::USD),
        quotes: vec![],
    };

    let _spec = CalibrationSpec {
        base_date,
        base_currency: Currency::USD,
        config: CalibrationConfig::default(),
        steps: vec![discount_step],
        schema_version: 1,
    };

    // Should support single step
}

#[test]
fn test_calibration_spec_with_multiple_steps() {
    let base_date = Date::from_calendar_date(2024, time::Month::January, 1).unwrap();

    let discount_step = CalibrationStep::Discount {
        calibrator: DiscountCurveCalibrator::new("USD-OIS", base_date, Currency::USD),
        quotes: vec![],
    };

    let hazard_step = CalibrationStep::Hazard {
        calibrator: HazardCurveCalibrator::new(
            "AAPL",
            Seniority::Senior,
            0.40,
            base_date,
            Currency::USD,
            "USD-OIS",
        ),
        quotes: vec![],
    };

    let _spec = CalibrationSpec {
        base_date,
        base_currency: Currency::USD,
        config: CalibrationConfig::default(),
        steps: vec![discount_step, hazard_step],
        schema_version: 1,
    };

    // Should support multiple steps
}

#[test]
fn test_calibration_spec_different_currencies() {
    let base_date = Date::from_calendar_date(2024, time::Month::January, 1).unwrap();

    let _spec_usd = CalibrationSpec {
        base_date,
        base_currency: Currency::USD,
        config: CalibrationConfig::default(),
        steps: vec![],
        schema_version: 1,
    };

    let _spec_eur = CalibrationSpec {
        base_date,
        base_currency: Currency::EUR,
        config: CalibrationConfig::default(),
        steps: vec![],
        schema_version: 1,
    };

    let _spec_gbp = CalibrationSpec {
        base_date,
        base_currency: Currency::GBP,
        config: CalibrationConfig::default(),
        steps: vec![],
        schema_version: 1,
    };

    // Should work with different currencies
}

#[test]
fn test_calibration_spec_different_dates() {
    let date1 = Date::from_calendar_date(2024, time::Month::January, 1).unwrap();
    let date2 = Date::from_calendar_date(2025, time::Month::June, 15).unwrap();

    let _spec1 = CalibrationSpec {
        base_date: date1,
        base_currency: Currency::USD,
        config: CalibrationConfig::default(),
        steps: vec![],
        schema_version: 1,
    };

    let _spec2 = CalibrationSpec {
        base_date: date2,
        base_currency: Currency::USD,
        config: CalibrationConfig::default(),
        steps: vec![],
        schema_version: 1,
    };

    // Should work with different dates
}

#[test]
fn test_calibration_spec_clone() {
    let base_date = Date::from_calendar_date(2024, time::Month::January, 1).unwrap();

    let spec1 = CalibrationSpec {
        base_date,
        base_currency: Currency::USD,
        config: CalibrationConfig::default(),
        steps: vec![],
        schema_version: 1,
    };

    let _spec2 = spec1.clone();

    // Should support cloning
}

#[test]
#[cfg(feature = "serde")]
fn test_calibration_spec_serde_roundtrip() {
    let base_date = Date::from_calendar_date(2024, time::Month::January, 1).unwrap();

    let spec = CalibrationSpec {
        base_date,
        base_currency: Currency::USD,
        config: CalibrationConfig::default(),
        steps: vec![],
        schema_version: 1,
    };

    let json = serde_json::to_string(&spec).unwrap();
    let _deserialized: CalibrationSpec = serde_json::from_str(&json).unwrap();

    // Should serialize and deserialize
}

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

    // Should succeed with empty pipeline
    assert!(result.is_ok());
}

#[test]
fn test_calibration_step_discount() {
    let base_date = Date::from_calendar_date(2024, time::Month::January, 1).unwrap();

    let _step = CalibrationStep::Discount {
        calibrator: DiscountCurveCalibrator::new("USD-OIS", base_date, Currency::USD),
        quotes: vec![RatesQuote::Deposit {
            maturity: base_date + time::Duration::days(30),
            rate: 0.045,
            day_count: DayCount::Act360,
        }],
    };

    // Should create discount step
}

#[test]
fn test_calibration_step_hazard() {
    let base_date = Date::from_calendar_date(2024, time::Month::January, 1).unwrap();

    let _step = CalibrationStep::Hazard {
        calibrator: HazardCurveCalibrator::new(
            "CORP_A",
            Seniority::Senior,
            0.40,
            base_date,
            Currency::USD,
            "USD-OIS",
        ),
        quotes: vec![],
    };

    // Should create hazard step
}

#[test]
fn test_calibration_config_default() {
    let _config = CalibrationConfig::default();

    // Should create default config
}

#[test]
fn test_calibration_spec_config_with_custom_values() {
    let base_date = Date::from_calendar_date(2024, time::Month::January, 1).unwrap();
    let config = CalibrationConfig {
        tolerance: 1e-8,
        max_iterations: 100,
        ..Default::default()
    };

    let _spec = CalibrationSpec {
        base_date,
        base_currency: Currency::USD,
        config,
        steps: vec![],
        schema_version: 1,
    };

    // Should accept custom config values
}

#[test]
fn test_calibration_spec_multiple_hazard_steps() {
    let base_date = Date::from_calendar_date(2024, time::Month::January, 1).unwrap();

    let discount_step = CalibrationStep::Discount {
        calibrator: DiscountCurveCalibrator::new("USD-OIS", base_date, Currency::USD),
        quotes: vec![],
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

    let _spec = CalibrationSpec {
        base_date,
        base_currency: Currency::USD,
        config: CalibrationConfig::default(),
        steps: vec![discount_step, hazard_step1, hazard_step2],
        schema_version: 1,
    };

    // Should handle multiple hazard steps
}

#[test]
fn test_calibration_spec_with_quotes() {
    let base_date = Date::from_calendar_date(2024, time::Month::January, 1).unwrap();

    let quotes = vec![
        RatesQuote::Deposit {
            maturity: base_date + time::Duration::days(30),
            rate: 0.045,
            day_count: DayCount::Act360,
        },
        RatesQuote::Swap {
            maturity: base_date + time::Duration::days(365),
            rate: 0.046,
            fixed_freq: Frequency::annual(),
            float_freq: Frequency::daily(),
            fixed_dc: DayCount::Act365F,
            float_dc: DayCount::Act365F,
            index: "USD-OIS".to_string().into(),
        },
    ];

    let discount_step = CalibrationStep::Discount {
        calibrator: DiscountCurveCalibrator::new("USD-OIS", base_date, Currency::USD),
        quotes,
    };

    let _spec = CalibrationSpec {
        base_date,
        base_currency: Currency::USD,
        config: CalibrationConfig::default(),
        steps: vec![discount_step],
        schema_version: 1,
    };

    // Should support quotes in steps
}
