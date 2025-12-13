//! Tests for solver configuration from FinstackConfig extensions.

#![cfg(feature = "serde")]

use finstack_core::config::FinstackConfig;
use finstack_core::math::solver::{BrentSolver, NewtonSolver, Solver};
use finstack_core::solver_config::{
    BrentSolverConfig, NewtonSolverConfig, SolverConfig, SOLVER_CONFIG_KEY_V1,
};
use serde_json::json;

#[test]
fn solver_config_defaults_without_extension() {
    let cfg = FinstackConfig::default();
    let solver_cfg = SolverConfig::from_finstack_config(&cfg).expect("valid config");

    assert_eq!(solver_cfg.newton.tolerance, 1e-12);
    assert_eq!(solver_cfg.newton.max_iterations, 50);
    assert_eq!(solver_cfg.newton.fd_step, 1e-8);
    assert_eq!(solver_cfg.newton.min_derivative, 1e-14);
    assert_eq!(solver_cfg.newton.min_derivative_rel, 1e-6);

    assert_eq!(solver_cfg.brent.tolerance, 1e-12);
    assert_eq!(solver_cfg.brent.max_iterations, 100);
    assert_eq!(solver_cfg.brent.bracket_expansion, 2.0);
}

#[test]
fn solver_config_applies_extension_overrides() {
    let mut cfg = FinstackConfig::default();
    cfg.extensions.insert(
        SOLVER_CONFIG_KEY_V1,
        json!({
            "newton": {
                "tolerance": 1e-10,
                "max_iterations": 100,
                "fd_step": 1e-6,
                "min_derivative": 1e-12,
                "min_derivative_rel": 1e-4
            },
            "brent": {
                "tolerance": 1e-8,
                "max_iterations": 200,
                "bracket_expansion": 3.0
            }
        }),
    );

    let solver_cfg = SolverConfig::from_finstack_config(&cfg).expect("valid config");

    assert_eq!(solver_cfg.newton.tolerance, 1e-10);
    assert_eq!(solver_cfg.newton.max_iterations, 100);
    assert_eq!(solver_cfg.newton.fd_step, 1e-6);
    assert_eq!(solver_cfg.newton.min_derivative, 1e-12);
    assert_eq!(solver_cfg.newton.min_derivative_rel, 1e-4);

    assert_eq!(solver_cfg.brent.tolerance, 1e-8);
    assert_eq!(solver_cfg.brent.max_iterations, 200);
    assert_eq!(solver_cfg.brent.bracket_expansion, 3.0);
}

#[test]
fn solver_config_partial_override_newton_only() {
    let mut cfg = FinstackConfig::default();
    cfg.extensions.insert(
        SOLVER_CONFIG_KEY_V1,
        json!({
            "newton": {
                "tolerance": 1e-8
            }
        }),
    );

    let solver_cfg = SolverConfig::from_finstack_config(&cfg).expect("valid config");

    // Newton overridden
    assert_eq!(solver_cfg.newton.tolerance, 1e-8);
    // Other Newton fields use defaults from NewtonSolverConfig
    assert_eq!(solver_cfg.newton.max_iterations, 50);

    // Brent uses defaults
    assert_eq!(solver_cfg.brent.tolerance, 1e-12);
    assert_eq!(solver_cfg.brent.max_iterations, 100);
}

#[test]
fn solver_config_roundtrip_serialization() {
    let original = SolverConfig {
        newton: NewtonSolverConfig {
            tolerance: 1e-10,
            max_iterations: 75,
            fd_step: 1e-7,
            min_derivative: 1e-15,
            min_derivative_rel: 1e-5,
        },
        brent: BrentSolverConfig {
            tolerance: 1e-9,
            max_iterations: 150,
            bracket_expansion: 2.5,
        },
    };

    let json = serde_json::to_string(&original).expect("serialize");
    let deserialized: SolverConfig = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(deserialized.newton.tolerance, original.newton.tolerance);
    assert_eq!(
        deserialized.newton.max_iterations,
        original.newton.max_iterations
    );
    assert_eq!(deserialized.brent.tolerance, original.brent.tolerance);
    assert_eq!(
        deserialized.brent.bracket_expansion,
        original.brent.bracket_expansion
    );
}

#[test]
fn newton_solver_from_config() {
    let cfg = NewtonSolverConfig {
        tolerance: 1e-10,
        max_iterations: 100,
        fd_step: 1e-7,
        min_derivative: 1e-15,
        min_derivative_rel: 1e-5,
    };

    let solver = NewtonSolver::from_config(&cfg);

    assert_eq!(solver.tolerance, 1e-10);
    assert_eq!(solver.max_iterations, 100);
    assert_eq!(solver.fd_step, 1e-7);
    assert_eq!(solver.min_derivative, 1e-15);
}

#[test]
fn brent_solver_from_config() {
    let cfg = BrentSolverConfig {
        tolerance: 1e-10,
        max_iterations: 200,
        bracket_expansion: 3.0,
    };

    let solver = BrentSolver::from_config(&cfg);

    assert_eq!(solver.tolerance, 1e-10);
    assert_eq!(solver.max_iterations, 200);
    assert_eq!(solver.bracket_expansion, 3.0);
}

#[test]
fn solver_from_config_works_correctly() {
    // Create a config with custom tolerance
    let cfg = NewtonSolverConfig {
        tolerance: 1e-8,
        ..Default::default()
    };

    let solver = NewtonSolver::from_config(&cfg);

    // Solve a simple quadratic: x^2 - 4 = 0
    let f = |x: f64| x * x - 4.0;
    let root = solver.solve(f, 1.0).expect("solver should converge");

    // Root should be 2.0
    assert!((root - 2.0).abs() < 1e-6);
}

#[test]
fn brent_from_config_works_correctly() {
    let cfg = BrentSolverConfig {
        tolerance: 1e-10,
        max_iterations: 200,
        bracket_expansion: 2.0,
    };

    let solver = BrentSolver::from_config(&cfg);

    // Solve: x^3 - x - 1 = 0
    let f = |x: f64| x * x * x - x - 1.0;
    let root = solver.solve(f, 1.0).expect("solver should converge");

    // Verify the root
    assert!(f(root).abs() < 1e-8);
}

