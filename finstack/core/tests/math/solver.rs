//! Root-finding solver tests.
//!
//! This module consolidates all solver tests including:
//! - Brent solver tests
//! - Newton solver tests
//! - Serialization tests

use finstack_core::math::solver::{BrentSolver, NewtonSolver, Solver};

// ============================================================================
// Brent Solver Tests
// ============================================================================

mod brent {
    use super::*;

    #[test]
    fn finds_root_simple_quadratic() {
        // f(x) = x^2 - 2 ⇒ root = sqrt(2)
        let f = |x: f64| x * x - 2.0;
        let solver = BrentSolver::new().with_tolerance(1e-12);
        let r = solver.solve(f, 1.5).unwrap();

        assert!(
            f(r).abs() < 1e-11,
            "f(root) = {} exceeds tolerance",
            f(r).abs()
        );
        assert!((r - 2.0_f64.sqrt()).abs() < 1e-10);
    }

    #[test]
    fn handles_cubic() {
        // f(x)=x^3 - x, roots at -1, 0, 1 ⇒ 1
        let f = |x: f64| x * x * x - x;
        let solver = BrentSolver::new().with_tolerance(1e-12);
        let r = solver.solve(f, 0.85).unwrap();

        assert!(
            f(r).abs() < 1e-11,
            "f(root) = {} exceeds tolerance",
            f(r).abs()
        );
        assert!((r - 1.0).abs() < 1e-10);
    }

    #[test]
    fn simple_quadratic() {
        let f = |x: f64| x * x - 4.0; // root at x = 2
        let solver = BrentSolver::new().with_tolerance(1e-12);

        let root = solver.solve(f, 1.8).unwrap();

        assert!(
            f(root).abs() < 1e-11,
            "f(root) = {} exceeds tolerance",
            f(root).abs()
        );
        assert!((root - 2.0).abs() < 1e-10);
    }

    #[test]
    fn with_distant_guess() {
        // Case where initial guess is far from root
        let f = |x: f64| x * x * x - x - 2.0; // Cubic with root near 1.5
        let solver = BrentSolver::new().with_tolerance(1e-12);

        // Bad initial guess that would cause Newton to diverge
        let root = solver.solve(f, 100.0).unwrap();

        assert!(
            f(root).abs() < 1e-11,
            "f(root) = {} exceeds tolerance",
            f(root).abs()
        );
    }

    #[test]
    fn bond_yield() {
        // Financial application: yield-to-maturity type calculation
        let target_price = 95.0;
        let coupon = 5.0;
        let face_value = 100.0;
        let periods = 5.0;

        let f = |y: f64| {
            if y.abs() < 1e-10 {
                return coupon * periods + face_value - target_price;
            }
            let discount_factor = 1.0 / (1.0 + y);
            let annuity_pv = coupon * (1.0 - discount_factor.powf(periods)) / y;
            let principal_pv = face_value * discount_factor.powf(periods);
            annuity_pv + principal_pv - target_price
        };

        let solver = BrentSolver::new().with_tolerance(1e-10);
        let yield_result = solver.solve(f, 0.06).unwrap();

        assert!(yield_result > 0.05 && yield_result < 0.08);
        assert!(
            f(yield_result).abs() < 1e-9,
            "f(yield) = {} exceeds tolerance",
            f(yield_result).abs()
        );
    }

    #[test]
    fn sqrt_function() {
        // Pathological case where derivative is problematic
        let f = |x: f64| (x - 1.5).signum() * (x - 1.5).abs().powf(0.5);
        let solver = BrentSolver::new().with_tolerance(1e-6);

        let root = solver.solve(f, 2.0).unwrap();
        assert!(
            f(root).abs() < 1e-5,
            "f(root) = {} exceeds tolerance",
            f(root).abs()
        );
        assert!((root - 1.5).abs() < 1e-5);
    }
}

// ============================================================================
// Newton Solver Tests
// ============================================================================

mod newton {
    use super::*;

    #[test]
    fn finds_root_simple_quadratic() {
        // f(x) = x^2 - 2 ⇒ root = sqrt(2)
        let f = |x: f64| x * x - 2.0;
        let solver = NewtonSolver::new().with_tolerance(1e-12);
        let r = solver.solve(f, 1.5).unwrap();

        assert!(
            f(r).abs() < 1e-11,
            "f(root) = {} exceeds tolerance",
            f(r).abs()
        );
        assert!((r - 2.0_f64.sqrt()).abs() < 1e-10);
    }

    #[test]
    fn with_analytical_derivative() {
        // f(x) = x^2 - 2, f'(x) = 2x
        let f = |x: f64| x * x - 2.0;
        let df = |x: f64| 2.0 * x;
        let solver = NewtonSolver::new().with_tolerance(1e-12);
        let r = solver.solve_with_derivative(f, df, 1.5).unwrap();

        assert!(
            f(r).abs() < 1e-11,
            "f(root) = {} exceeds tolerance",
            f(r).abs()
        );
        assert!((r - 2.0_f64.sqrt()).abs() < 1e-10);
    }

    #[test]
    fn handles_cubic() {
        // f(x) = x^3 - x, roots at -1, 0, 1
        let f = |x: f64| x * x * x - x;
        let solver = NewtonSolver::new().with_tolerance(1e-12);
        let r = solver.solve(f, 0.85).unwrap();

        assert!(
            f(r).abs() < 1e-11,
            "f(root) = {} exceeds tolerance",
            f(r).abs()
        );
        assert!((r - 1.0).abs() < 1e-10);
    }

    #[test]
    fn with_analytical_derivative_cubic() {
        // f(x) = x^3 - 2x - 5, f'(x) = 3x^2 - 2
        let f = |x: f64| x.powi(3) - 2.0 * x - 5.0;
        let df = |x: f64| 3.0 * x.powi(2) - 2.0;
        let solver = NewtonSolver::new().with_tolerance(1e-12);
        let r = solver.solve_with_derivative(f, df, 2.0).unwrap();

        assert!(
            f(r).abs() < 1e-11,
            "f(root) = {} exceeds tolerance",
            f(r).abs()
        );
        // Root is approximately 2.0946
        assert!((r - 2.0946).abs() < 1e-4);
    }

    #[test]
    fn bond_yield() {
        // Same financial application as Brent: yield-to-maturity
        let target_price = 95.0;
        let coupon = 5.0;
        let face_value = 100.0;
        let periods = 5.0;

        let f = |y: f64| {
            if y.abs() < 1e-10 {
                return coupon * periods + face_value - target_price;
            }
            let discount_factor = 1.0 / (1.0 + y);
            let annuity_pv = coupon * (1.0 - discount_factor.powf(periods)) / y;
            let principal_pv = face_value * discount_factor.powf(periods);
            annuity_pv + principal_pv - target_price
        };

        let solver = NewtonSolver::new().with_tolerance(1e-10);
        let yield_result = solver.solve(f, 0.06).unwrap();

        assert!(yield_result > 0.05 && yield_result < 0.08);
        assert!(
            f(yield_result).abs() < 1e-9,
            "f(yield) = {} exceeds tolerance",
            f(yield_result).abs()
        );
    }

    #[test]
    fn transcendental_equation() {
        // f(x) = e^x - 3x, has root near x ≈ 1.05
        let f = |x: f64| x.exp() - 3.0 * x;
        let df = |x: f64| x.exp() - 3.0;
        let solver = NewtonSolver::new().with_tolerance(1e-12);
        let r = solver.solve_with_derivative(f, df, 1.0).unwrap();

        assert!(
            f(r).abs() < 1e-11,
            "f(root) = {} exceeds tolerance",
            f(r).abs()
        );
    }
}

// ============================================================================
// Serialization Tests
// ============================================================================

#[cfg(feature = "serde")]
mod serde_tests {
    use finstack_core::math::integration::GaussHermiteQuadrature;
    use finstack_core::math::random::{RandomNumberGenerator, TestRng};
    use finstack_core::math::solver::{BrentSolver, NewtonSolver};

    #[test]
    fn newton_solver_roundtrip() {
        let solver = NewtonSolver::new()
            .with_tolerance(1e-10)
            .with_max_iterations(100);

        let json = serde_json::to_string(&solver).unwrap();
        let deserialized: NewtonSolver = serde_json::from_str(&json).unwrap();

        assert_eq!(solver.tolerance, deserialized.tolerance);
        assert_eq!(solver.max_iterations, deserialized.max_iterations);
        assert_eq!(solver.fd_step, deserialized.fd_step);
    }

    #[test]
    fn brent_solver_roundtrip() {
        let solver = BrentSolver::new()
            .with_tolerance(1e-8)
            .with_initial_bracket_size(Some(0.5));

        let json = serde_json::to_string(&solver).unwrap();
        let deserialized: BrentSolver = serde_json::from_str(&json).unwrap();

        assert_eq!(solver.tolerance, deserialized.tolerance);
        assert_eq!(solver.max_iterations, deserialized.max_iterations);
        assert_eq!(solver.bracket_expansion, deserialized.bracket_expansion);
        assert_eq!(
            solver.initial_bracket_size,
            deserialized.initial_bracket_size
        );
    }

    #[test]
    fn gauss_hermite_quadrature_order_5() {
        let quad5 = GaussHermiteQuadrature::order_5();
        let json = serde_json::to_string(&quad5).unwrap();
        let deserialized: GaussHermiteQuadrature = serde_json::from_str(&json).unwrap();
        assert_eq!(quad5.points.len(), deserialized.points.len());
        assert_eq!(quad5.weights.len(), deserialized.weights.len());
    }

    #[test]
    fn gauss_hermite_quadrature_order_7() {
        let quad7 = GaussHermiteQuadrature::order_7();
        let json = serde_json::to_string(&quad7).unwrap();
        let deserialized: GaussHermiteQuadrature = serde_json::from_str(&json).unwrap();
        assert_eq!(quad7.points.len(), deserialized.points.len());
        assert_eq!(quad7.weights.len(), deserialized.weights.len());
    }

    #[test]
    fn gauss_hermite_quadrature_order_10() {
        let quad10 = GaussHermiteQuadrature::order_10();
        let json = serde_json::to_string(&quad10).unwrap();
        let deserialized: GaussHermiteQuadrature = serde_json::from_str(&json).unwrap();
        assert_eq!(quad10.points.len(), deserialized.points.len());
        assert_eq!(quad10.weights.len(), deserialized.weights.len());

        let quad5_json = serde_json::to_string(&GaussHermiteQuadrature::order_5()).unwrap();
        assert!(quad5_json.contains("\"order\":5"));
    }

    #[test]
    fn test_rng_roundtrip() {
        let mut rng = TestRng::new(42);

        let _val1 = rng.uniform();
        let _val2 = rng.uniform();

        let json = serde_json::to_string(&rng).unwrap();
        let mut deserialized: TestRng = serde_json::from_str(&json).unwrap();

        let val3_orig = rng.uniform();
        let val3_deser = deserialized.uniform();
        assert_eq!(val3_orig, val3_deser);

        let val4_orig = rng.uniform();
        let val4_deser = deserialized.uniform();
        assert_eq!(val4_orig, val4_deser);
    }

    #[test]
    fn solver_configs_roundtrip() {
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

        assert!(json.contains("\"tolerance\""));
        assert!(json.contains("\"max_iterations\""));
        assert!(json.contains("\"fd_step\""));
    }

    #[test]
    fn quadrature_functional_equivalence() {
        let quad_orig = GaussHermiteQuadrature::order_7();
        let json = serde_json::to_string(&quad_orig).unwrap();
        let quad_deser: GaussHermiteQuadrature = serde_json::from_str(&json).unwrap();

        let f = |x: f64| x * x;
        let result_orig = quad_orig.integrate(f);
        let result_deser = quad_deser.integrate(f);

        assert!((result_orig - result_deser).abs() < 1e-15);
    }
}
