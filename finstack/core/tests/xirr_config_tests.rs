//! Tests for XIRR configuration from FinstackConfig extensions.

#![cfg(feature = "serde")]

use finstack_core::cashflow::InternalRateOfReturn;
use finstack_core::config::FinstackConfig;
use finstack_core::dates::create_date;
use finstack_core::xirr_config::{XirrConfig, XIRR_CONFIG_KEY_V1};
use serde_json::json;
use time::Month;

#[test]
fn xirr_config_defaults_without_extension() {
    let cfg = FinstackConfig::default();
    let xirr_cfg = XirrConfig::from_finstack_config(&cfg).expect("valid config");

    assert_eq!(xirr_cfg.tolerance, 1e-6);
    assert_eq!(xirr_cfg.max_iterations, 100);
    assert_eq!(xirr_cfg.default_guess, 0.1);
}

#[test]
fn xirr_config_applies_extension_overrides() {
    let mut cfg = FinstackConfig::default();
    cfg.extensions.insert(
        XIRR_CONFIG_KEY_V1,
        json!({
            "tolerance": 1e-8,
            "max_iterations": 200,
            "default_guess": 0.05
        }),
    );

    let xirr_cfg = XirrConfig::from_finstack_config(&cfg).expect("valid config");

    assert_eq!(xirr_cfg.tolerance, 1e-8);
    assert_eq!(xirr_cfg.max_iterations, 200);
    assert_eq!(xirr_cfg.default_guess, 0.05);
}

#[test]
fn xirr_config_partial_override() {
    let mut cfg = FinstackConfig::default();
    cfg.extensions.insert(
        XIRR_CONFIG_KEY_V1,
        json!({
            "tolerance": 1e-10
        }),
    );

    let xirr_cfg = XirrConfig::from_finstack_config(&cfg).expect("valid config");

    // Overridden
    assert_eq!(xirr_cfg.tolerance, 1e-10);
    // Defaults
    assert_eq!(xirr_cfg.max_iterations, 100);
    assert_eq!(xirr_cfg.default_guess, 0.1);
}

#[test]
fn xirr_config_roundtrip_serialization() {
    let original = XirrConfig {
        tolerance: 1e-9,
        max_iterations: 150,
        default_guess: 0.15,
    };

    let json = serde_json::to_string(&original).expect("serialize");
    let deserialized: XirrConfig = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(deserialized.tolerance, original.tolerance);
    assert_eq!(deserialized.max_iterations, original.max_iterations);
    assert_eq!(deserialized.default_guess, original.default_guess);
}

#[test]
fn irr_with_config_works_for_periodic() {
    let cfg = FinstackConfig::default();

    let amounts = [-100.0, 110.0];
    let irr = amounts.irr_with_config(&cfg, None).expect("IRR should work");

    // Should get ~10% return
    assert!((irr - 0.1).abs() < 1e-6);
}

#[test]
fn irr_with_config_works_for_dated() {
    let cfg = FinstackConfig::default();

    let flows = [
        (
            create_date(2024, Month::January, 1).expect("Valid date"),
            -100_000.0,
        ),
        (
            create_date(2025, Month::January, 1).expect("Valid date"),
            110_000.0,
        ),
    ];

    let xirr = flows.irr_with_config(&cfg, None).expect("XIRR should work");

    // Should be approximately 10% annualized (adjusted for actual days)
    assert!(xirr > 0.09 && xirr < 0.11);
}

#[test]
fn irr_with_custom_tolerance_config() {
    let mut cfg = FinstackConfig::default();
    cfg.extensions.insert(
        XIRR_CONFIG_KEY_V1,
        json!({
            "tolerance": 1e-10,
            "max_iterations": 200
        }),
    );

    let flows = [
        (
            create_date(2024, Month::January, 1).expect("Valid date"),
            -100_000.0,
        ),
        (
            create_date(2024, Month::July, 1).expect("Valid date"),
            5_000.0,
        ),
        (
            create_date(2025, Month::January, 1).expect("Valid date"),
            110_000.0,
        ),
    ];

    let xirr = flows.irr_with_config(&cfg, None).expect("XIRR should work");

    // Should converge with the stricter tolerance
    assert!(xirr > 0.1 && xirr < 0.2);
}

#[test]
fn irr_with_config_respects_guess_override() {
    let mut cfg = FinstackConfig::default();
    cfg.extensions.insert(
        XIRR_CONFIG_KEY_V1,
        json!({
            "default_guess": 0.5  // High default guess
        }),
    );

    let flows = [
        (
            create_date(2024, Month::January, 1).expect("Valid date"),
            -100_000.0,
        ),
        (
            create_date(2025, Month::January, 1).expect("Valid date"),
            110_000.0,
        ),
    ];

    // User-provided guess should override config default
    let xirr = flows
        .irr_with_config(&cfg, Some(0.05))
        .expect("XIRR should work");

    // Should still find the correct rate
    assert!(xirr > 0.09 && xirr < 0.11);
}

