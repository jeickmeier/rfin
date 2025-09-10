//! Tests for calibration module components.

#[cfg(test)]
mod tests {
    use crate::calibration::*;
    use finstack_core::math::Solver;
    use finstack_core::F;
    use std::collections::HashMap;

    #[test]
    fn test_solver_selection() {
        // Test that different solver kinds can be created and work
        let mut config = CalibrationConfig::default();

        // Test default (Hybrid) - verify it can solve
        let f = |x: F| x * x - 4.0; // Root at x = 2
        let root = crate::with_solver!(&config, |solver| solver.solve(f, 1.5).unwrap());
        assert!((root - 2.0).abs() < 1e-6);

        // Test Newton
        config.solver_kind = SolverKind::Newton;
        let root = crate::with_solver!(&config, |solver| solver.solve(f, 1.5).unwrap());
        assert!((root - 2.0).abs() < 1e-6);

        // Test Brent
        config.solver_kind = SolverKind::Brent;
        let root = crate::with_solver!(&config, |solver| solver.solve(f, 1.5).unwrap());
        assert!((root - 2.0).abs() < 1e-6);
    }

    #[test]
    fn test_calibration_config_defaults() {
        let config = CalibrationConfig::default();
        assert_eq!(config.tolerance, 1e-10);
        assert_eq!(config.max_iterations, 100);
        assert!(!config.use_parallel);
        assert_eq!(config.random_seed, Some(42));
        assert!(!config.verbose);
        assert_eq!(config.solver_kind, SolverKind::Hybrid);
        assert!(config.entity_seniority.is_empty());
    }

    #[test]
    fn test_solver_kind_default() {
        assert_eq!(SolverKind::default(), SolverKind::Hybrid);
    }

    #[test]
    fn test_calibration_report_success_simple() {
        let mut residuals = HashMap::new();
        residuals.insert("test_instrument".to_string(), 1e-6);
        residuals.insert("another_instrument".to_string(), 2e-6);

        let report = CalibrationReport::success_simple(residuals, 10);

        assert!(report.success);
        assert_eq!(report.iterations, 10);
        assert_eq!(report.convergence_reason, "Calibration completed successfully");
        assert_eq!(report.residuals.len(), 2);
        assert!(report.max_residual > 0.0);
        assert!(report.rmse > 0.0);
    }

    #[test]
    fn test_calibration_report_push_residual() {
        let mut report = CalibrationReport::empty_success("Testing push_residual");

        report.push_residual("instrument1", 1e-6);
        report.push_residual("instrument2", 2e-6);

        assert_eq!(report.residuals.len(), 2);
        assert!((report.max_residual - 2e-6).abs() < 1e-12);

        // Test that metrics are updated correctly
        let expected_rmse = ((1e-12_f64 + 4e-12_f64) / 2.0_f64).sqrt();
        assert!((report.rmse - expected_rmse).abs() < 1e-15);
    }
}
