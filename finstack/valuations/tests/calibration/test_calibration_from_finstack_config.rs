#![cfg(feature = "serde")]

use finstack_core::config::FinstackConfig;
use finstack_valuations::calibration::{
    CalibrationConfig, CalibrationMethod, RateBounds, RateBoundsPolicy, SolverKind,
    CALIBRATION_CONFIG_KEY_V1,
};
use serde_json::json;

#[test]
fn calibration_config_applies_extension_overrides() {
    let mut cfg = FinstackConfig::default();
    cfg.extensions.insert(
        CALIBRATION_CONFIG_KEY_V1,
        json!({
            "tolerance": 1e-8,
            "max_iterations": 250,
            "use_parallel": true,
            "random_seed": null,
            "solver_kind": "newton",
            "rate_bounds_policy": "explicit",
            "rate_bounds": { "min_rate": -0.01, "max_rate": 0.10 },
            "calibration_method": { "global_solve": { "use_analytical_jacobian": true } }
        }),
    );

    let cfg_out = CalibrationConfig::from_finstack_config_or_default(&cfg).expect("apply overrides");
    assert_eq!(cfg_out.tolerance, 1e-8);
    assert_eq!(cfg_out.max_iterations, 250);
    assert!(cfg_out.use_parallel);
    assert_eq!(cfg_out.random_seed, None);
    assert_eq!(cfg_out.solver_kind, SolverKind::Newton);
    assert_eq!(cfg_out.rate_bounds_policy, RateBoundsPolicy::Explicit);
    assert_eq!(
        cfg_out.rate_bounds,
        RateBounds {
            min_rate: -0.01,
            max_rate: 0.10
        }
    );
    assert!(matches!(
        cfg_out.calibration_method,
        CalibrationMethod::GlobalSolve {
            use_analytical_jacobian: true
        }
    ));
}

#[test]
fn calibration_config_defaults_without_extension() {
    let cfg = FinstackConfig::default();
    let cfg_out = CalibrationConfig::from_finstack_config_or_default(&cfg).expect("defaults");
    let defaults = CalibrationConfig::default();

    assert_eq!(cfg_out.tolerance, defaults.tolerance);
    assert_eq!(cfg_out.max_iterations, defaults.max_iterations);
    assert_eq!(cfg_out.use_parallel, defaults.use_parallel);
    assert_eq!(cfg_out.random_seed, defaults.random_seed);
    assert_eq!(cfg_out.solver_kind, defaults.solver_kind);
    assert_eq!(cfg_out.rate_bounds_policy, defaults.rate_bounds_policy);
    assert_eq!(cfg_out.rate_bounds, defaults.rate_bounds);
}

