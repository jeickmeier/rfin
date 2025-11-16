//! Test serialization of mathematical solver configurations

#[cfg(feature = "serde")]
mod tests {
    use finstack_core::math::integration::GaussHermiteQuadrature;
    use finstack_core::math::random::{RandomNumberGenerator, SimpleRng};
    use finstack_core::math::solver::{BrentSolver, HybridSolver, NewtonSolver};

    #[test]
    fn test_newton_solver_serde() {
        let solver = NewtonSolver::new()
            .with_tolerance(1e-10)
            .with_max_iterations(100);

        // Serialize
        let json = serde_json::to_string(&solver).unwrap();

        // Deserialize
        let deserialized: NewtonSolver = serde_json::from_str(&json).unwrap();

        // Check equality
        assert_eq!(solver.tolerance, deserialized.tolerance);
        assert_eq!(solver.max_iterations, deserialized.max_iterations);
        assert_eq!(solver.fd_step, deserialized.fd_step);
    }

    #[test]
    fn test_brent_solver_serde() {
        let solver = BrentSolver::new()
            .with_tolerance(1e-8)
            .with_initial_bracket_size(Some(0.5));

        // Serialize
        let json = serde_json::to_string(&solver).unwrap();

        // Deserialize
        let deserialized: BrentSolver = serde_json::from_str(&json).unwrap();

        // Check equality
        assert_eq!(solver.tolerance, deserialized.tolerance);
        assert_eq!(solver.max_iterations, deserialized.max_iterations);
        assert_eq!(solver.bracket_expansion, deserialized.bracket_expansion);
        assert_eq!(
            solver.initial_bracket_size,
            deserialized.initial_bracket_size
        );
    }

    #[test]
    fn test_hybrid_solver_serde() {
        let solver = HybridSolver::new()
            .with_tolerance(1e-12)
            .with_max_iterations(50);

        // Serialize
        let json = serde_json::to_string(&solver).unwrap();

        // Deserialize
        let deserialized: HybridSolver = serde_json::from_str(&json).unwrap();

        // Check that both internal solvers are properly configured
        // Note: We can't directly access newton and brent fields as they're private,
        // but we can verify through a round-trip test
        let json2 = serde_json::to_string(&deserialized).unwrap();
        assert_eq!(json, json2);
    }

    #[test]
    fn test_gauss_hermite_quadrature_serde() {
        // Test order 5
        let quad5 = GaussHermiteQuadrature::order_5();
        let json = serde_json::to_string(&quad5).unwrap();
        let deserialized: GaussHermiteQuadrature = serde_json::from_str(&json).unwrap();
        assert_eq!(quad5.points.len(), deserialized.points.len());
        assert_eq!(quad5.weights.len(), deserialized.weights.len());

        // Test order 7
        let quad7 = GaussHermiteQuadrature::order_7();
        let json = serde_json::to_string(&quad7).unwrap();
        let deserialized: GaussHermiteQuadrature = serde_json::from_str(&json).unwrap();
        assert_eq!(quad7.points.len(), deserialized.points.len());
        assert_eq!(quad7.weights.len(), deserialized.weights.len());

        // Test order 10
        let quad10 = GaussHermiteQuadrature::order_10();
        let json = serde_json::to_string(&quad10).unwrap();
        let deserialized: GaussHermiteQuadrature = serde_json::from_str(&json).unwrap();
        assert_eq!(quad10.points.len(), deserialized.points.len());
        assert_eq!(quad10.weights.len(), deserialized.weights.len());

        // Verify the JSON structure is simple (just contains order)
        let quad5_json = serde_json::to_string(&quad5).unwrap();
        assert!(quad5_json.contains("\"order\":5"));
    }

    #[test]
    fn test_simple_rng_serde() {
        let mut rng = SimpleRng::new(42);

        // Generate some values to change internal state
        let _val1 = rng.uniform();
        let _val2 = rng.uniform();

        // Serialize current state
        let json = serde_json::to_string(&rng).unwrap();

        // Deserialize
        let mut deserialized: SimpleRng = serde_json::from_str(&json).unwrap();

        // Both RNGs should now produce the same sequence
        let val3_orig = rng.uniform();
        let val3_deser = deserialized.uniform();
        assert_eq!(val3_orig, val3_deser);

        let val4_orig = rng.uniform();
        let val4_deser = deserialized.uniform();
        assert_eq!(val4_orig, val4_deser);
    }

    #[test]
    fn test_solver_configs_roundtrip() {
        // Test that complex configurations survive a round-trip
        let newton = NewtonSolver {
            tolerance: 1e-15,
            max_iterations: 200,
            fd_step: 1e-7,
            min_derivative: 1e-14,
        };

        let json = serde_json::to_string_pretty(&newton).unwrap();
        let newton2: NewtonSolver = serde_json::from_str(&json).unwrap();

        assert_eq!(newton.tolerance, newton2.tolerance);
        assert_eq!(newton.max_iterations, newton2.max_iterations);
        assert_eq!(newton.fd_step, newton2.fd_step);

        // Verify JSON contains expected fields
        assert!(json.contains("\"tolerance\""));
        assert!(json.contains("\"max_iterations\""));
        assert!(json.contains("\"fd_step\""));
    }

    #[test]
    fn test_quadrature_functional_equivalence() {
        // Test that deserialized quadrature produces same results
        let quad_orig = GaussHermiteQuadrature::order_7();
        let json = serde_json::to_string(&quad_orig).unwrap();
        let quad_deser: GaussHermiteQuadrature = serde_json::from_str(&json).unwrap();

        // Test integration with both
        let f = |x: f64| x * x; // x^2
        let result_orig = quad_orig.integrate(f);
        let result_deser = quad_deser.integrate(f);

        assert!((result_orig - result_deser).abs() < 1e-15);
    }
}
