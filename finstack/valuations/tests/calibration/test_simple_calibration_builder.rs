//! Tests for SimpleCalibration builder and basic functionality

use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::term_structures::Seniority;
use finstack_valuations::calibration::simple_calibration::SimpleCalibration;
use finstack_valuations::calibration::{CalibrationConfig, MultiCurveConfig};

#[test]
fn test_simple_calibration_new() {
    let base_date = Date::from_calendar_date(2024, time::Month::January, 1).unwrap();
    let _cal = SimpleCalibration::new(base_date, Currency::USD);

    // Should create without panicking
}

#[test]
fn test_simple_calibration_with_config() {
    let base_date = Date::from_calendar_date(2024, time::Month::January, 1).unwrap();
    let config = CalibrationConfig::default();

    let _cal = SimpleCalibration::new(base_date, Currency::USD).with_config(config.clone());

    // Should chain builder methods
}

#[test]
fn test_simple_calibration_with_multi_curve_config() {
    let base_date = Date::from_calendar_date(2024, time::Month::January, 1).unwrap();
    let multi_curve_config = MultiCurveConfig::default();

    let _cal = SimpleCalibration::new(base_date, Currency::USD)
        .with_multi_curve_config(multi_curve_config);

    // Should set multi-curve config
}

#[test]
fn test_simple_calibration_with_entity_seniority() {
    let base_date = Date::from_calendar_date(2024, time::Month::January, 1).unwrap();

    let _cal = SimpleCalibration::new(base_date, Currency::USD)
        .with_entity_seniority("CORP_A", Seniority::Senior)
        .with_entity_seniority("CORP_B", Seniority::Subordinated);

    // Should add entity seniorities
}

#[test]
fn test_simple_calibration_builder_chaining() {
    let base_date = Date::from_calendar_date(2024, time::Month::January, 1).unwrap();
    let config = CalibrationConfig::default();

    let _cal = SimpleCalibration::new(base_date, Currency::EUR)
        .with_config(config.clone())
        .with_entity_seniority("ENTITY_1", Seniority::Senior)
        .with_entity_seniority("ENTITY_2", Seniority::Subordinated);

    // Should support chaining multiple builder methods
}

#[test]
fn test_simple_calibration_different_currencies() {
    let base_date = Date::from_calendar_date(2024, time::Month::January, 1).unwrap();

    let _cal_usd = SimpleCalibration::new(base_date, Currency::USD);
    let _cal_eur = SimpleCalibration::new(base_date, Currency::EUR);
    let _cal_gbp = SimpleCalibration::new(base_date, Currency::GBP);

    // Should work with different currencies
}

#[test]
fn test_simple_calibration_different_dates() {
    let date1 = Date::from_calendar_date(2024, time::Month::January, 1).unwrap();
    let date2 = Date::from_calendar_date(2025, time::Month::June, 15).unwrap();

    let _cal1 = SimpleCalibration::new(date1, Currency::USD);
    let _cal2 = SimpleCalibration::new(date2, Currency::USD);

    // Should work with different dates
}

#[test]
fn test_simple_calibration_entity_seniority_overwrite() {
    let base_date = Date::from_calendar_date(2024, time::Month::January, 1).unwrap();

    let _cal = SimpleCalibration::new(base_date, Currency::USD)
        .with_entity_seniority("ENTITY", Seniority::Senior)
        .with_entity_seniority("ENTITY", Seniority::Subordinated); // Overwrite

    // Should allow overwriting entity seniority
}

#[test]
fn test_simple_calibration_clone() {
    let base_date = Date::from_calendar_date(2024, time::Month::January, 1).unwrap();

    let cal1 = SimpleCalibration::new(base_date, Currency::USD)
        .with_entity_seniority("ENTITY", Seniority::Senior);

    let _cal2 = cal1.clone();

    // Should support cloning
}

#[test]
#[cfg(feature = "serde")]
fn test_simple_calibration_serde_roundtrip() {
    let base_date = Date::from_calendar_date(2024, time::Month::January, 1).unwrap();

    let cal = SimpleCalibration::new(base_date, Currency::USD)
        .with_entity_seniority("ENTITY", Seniority::Senior);

    let json = serde_json::to_string(&cal).unwrap();
    let _deserialized: SimpleCalibration = serde_json::from_str(&json).unwrap();

    // Should serialize and deserialize
}

#[test]
fn test_simple_calibration_empty_quotes() {
    let base_date = Date::from_calendar_date(2024, time::Month::January, 1).unwrap();
    let cal = SimpleCalibration::new(base_date, Currency::USD);

    let quotes = vec![];
    let result = cal.calibrate(&quotes);

    // Should succeed with empty market
    assert!(result.is_ok());
}

#[test]
fn test_simple_calibration_multiple_entities() {
    let base_date = Date::from_calendar_date(2024, time::Month::January, 1).unwrap();

    let _cal = SimpleCalibration::new(base_date, Currency::USD)
        .with_entity_seniority("CORP_1", Seniority::Senior)
        .with_entity_seniority("CORP_2", Seniority::Senior)
        .with_entity_seniority("CORP_3", Seniority::Subordinated)
        .with_entity_seniority("CORP_4", Seniority::Junior);

    // Should handle multiple entities
}

#[test]
fn test_calibration_config_default() {
    let _config = CalibrationConfig::default();

    // Should create default config
}

#[test]
fn test_multi_curve_config_default() {
    let _config = MultiCurveConfig::default();

    // Should create default multi-curve config
}

#[test]
fn test_simple_calibration_config_with_custom_values() {
    let base_date = Date::from_calendar_date(2024, time::Month::January, 1).unwrap();
    let config = CalibrationConfig {
        tolerance: 1e-8,
        max_iterations: 100,
        ..Default::default()
    };

    let _cal = SimpleCalibration::new(base_date, Currency::USD).with_config(config);

    // Should accept custom config values
}

#[test]
fn test_simple_calibration_all_seniorities() {
    let base_date = Date::from_calendar_date(2024, time::Month::January, 1).unwrap();

    let _cal = SimpleCalibration::new(base_date, Currency::USD)
        .with_entity_seniority("E1", Seniority::Senior)
        .with_entity_seniority("E2", Seniority::Subordinated)
        .with_entity_seniority("E3", Seniority::Junior);

    // Should handle all seniority levels
}
