use finstack_core::math::solver::{BrentSolver, NewtonSolver, Solver};

// ============================================
// Brent Solver Tests
// ============================================

#[test]
fn brent_finds_root_simple_quadratic() {
    // f(x) = x^2 - 2 ⇒ root = sqrt(2)
    let f = |x: f64| x * x - 2.0;
    let solver = BrentSolver::new().with_tolerance(1e-12);
    let r = solver.solve(f, 1.5).unwrap();

    // Verify residual matches solver tolerance
    assert!(
        f(r).abs() < 1e-11,
        "f(root) = {} exceeds tolerance",
        f(r).abs()
    );
    assert!((r - 2.0_f64.sqrt()).abs() < 1e-10);
}

#[test]
fn brent_solver_handles_cubic() {
    // f(x)=x^3 - x, roots at -1, 0, 1 ⇒ 1
    let f = |x: f64| x * x * x - x;
    let solver = BrentSolver::new().with_tolerance(1e-12);
    let r = solver.solve(f, 0.85).unwrap();

    // Verify residual matches solver tolerance
    assert!(
        f(r).abs() < 1e-11,
        "f(root) = {} exceeds tolerance",
        f(r).abs()
    );
    assert!((r - 1.0).abs() < 1e-10);
}

#[test]
fn brent_solver_simple_quadratic() {
    // Simple case
    let f = |x: f64| x * x - 4.0; // root at x = 2
    let solver = BrentSolver::new().with_tolerance(1e-12);

    let root = solver.solve(f, 1.8).unwrap();

    // Verify residual matches solver tolerance
    assert!(
        f(root).abs() < 1e-11,
        "f(root) = {} exceeds tolerance",
        f(root).abs()
    );
    assert!((root - 2.0).abs() < 1e-10);
}

#[test]
fn brent_solver_with_distant_guess() {
    // Case where initial guess is far from root
    let f = |x: f64| x * x * x - x - 2.0; // Cubic with root near 1.5
    let solver = BrentSolver::new().with_tolerance(1e-12);

    // Bad initial guess that would cause Newton to diverge
    let root = solver.solve(f, 100.0).unwrap();

    // Verify residual matches solver tolerance
    assert!(
        f(root).abs() < 1e-11,
        "f(root) = {} exceeds tolerance",
        f(root).abs()
    );
}

#[test]
fn brent_solver_bond_yield() {
    // Financial application: yield-to-maturity type calculation
    // Bond price equation: P = Σ C/(1+y)^t where we solve for y given P
    let target_price = 95.0;
    let coupon = 5.0;
    let face_value = 100.0;
    let periods = 5.0;

    // Simplified bond pricing: P = C * sum_geo + FV / (1+y)^n
    let f = |y: f64| {
        if y.abs() < 1e-10 {
            return coupon * periods + face_value - target_price;
        }
        let discount_factor = 1.0 / (1.0 + y);
        let annuity_pv = coupon * (1.0 - discount_factor.powf(periods)) / y;
        let principal_pv = face_value * discount_factor.powf(periods);
        annuity_pv + principal_pv - target_price
    };

    // Use BrentSolver
    let solver = BrentSolver::new().with_tolerance(1e-10);
    let yield_result = solver.solve(f, 0.06).unwrap();

    // Verify the yield makes sense (should be around 6-7% for this bond)
    assert!(yield_result > 0.05 && yield_result < 0.08);
    // Verify residual matches solver tolerance
    assert!(
        f(yield_result).abs() < 1e-9,
        "f(yield) = {} exceeds tolerance",
        f(yield_result).abs()
    );
}

#[test]
fn brent_solver_sqrt_function() {
    // Pathological case where derivative is problematic
    let f = |x: f64| (x - 1.5).signum() * (x - 1.5).abs().powf(0.5); // sqrt function with sign
    let solver = BrentSolver::new().with_tolerance(1e-6);

    let root = solver.solve(f, 2.0).unwrap();
    // Verify residual matches solver tolerance
    assert!(
        f(root).abs() < 1e-5,
        "f(root) = {} exceeds tolerance",
        f(root).abs()
    );
    assert!((root - 1.5).abs() < 1e-5);
}

// ============================================
// Newton Solver Tests
// ============================================

#[test]
fn newton_finds_root_simple_quadratic() {
    // f(x) = x^2 - 2 ⇒ root = sqrt(2)
    let f = |x: f64| x * x - 2.0;
    let solver = NewtonSolver::new().with_tolerance(1e-12);
    let r = solver.solve(f, 1.5).unwrap();

    // Verify residual matches solver tolerance
    assert!(
        f(r).abs() < 1e-11,
        "f(root) = {} exceeds tolerance",
        f(r).abs()
    );
    assert!((r - 2.0_f64.sqrt()).abs() < 1e-10);
}

#[test]
fn newton_with_analytical_derivative() {
    // f(x) = x^2 - 2, f'(x) = 2x
    let f = |x: f64| x * x - 2.0;
    let df = |x: f64| 2.0 * x;
    let solver = NewtonSolver::new().with_tolerance(1e-12);
    let r = solver.solve_with_derivative(f, df, 1.5).unwrap();

    // Verify residual matches solver tolerance
    assert!(
        f(r).abs() < 1e-11,
        "f(root) = {} exceeds tolerance",
        f(r).abs()
    );
    assert!((r - 2.0_f64.sqrt()).abs() < 1e-10);
}

#[test]
fn newton_solver_handles_cubic() {
    // f(x) = x^3 - x, roots at -1, 0, 1
    let f = |x: f64| x * x * x - x;
    let solver = NewtonSolver::new().with_tolerance(1e-12);
    let r = solver.solve(f, 0.85).unwrap();

    // Verify residual matches solver tolerance
    assert!(
        f(r).abs() < 1e-11,
        "f(root) = {} exceeds tolerance",
        f(r).abs()
    );
    assert!((r - 1.0).abs() < 1e-10);
}

#[test]
fn newton_with_analytical_derivative_cubic() {
    // f(x) = x^3 - 2x - 5, f'(x) = 3x^2 - 2
    let f = |x: f64| x.powi(3) - 2.0 * x - 5.0;
    let df = |x: f64| 3.0 * x.powi(2) - 2.0;
    let solver = NewtonSolver::new().with_tolerance(1e-12);
    let r = solver.solve_with_derivative(f, df, 2.0).unwrap();

    // Verify residual matches solver tolerance
    assert!(
        f(r).abs() < 1e-11,
        "f(root) = {} exceeds tolerance",
        f(r).abs()
    );
    // Root is approximately 2.0946
    assert!((r - 2.0946).abs() < 1e-4);
}

#[test]
fn newton_solver_bond_yield() {
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

    // Verify the yield makes sense
    assert!(yield_result > 0.05 && yield_result < 0.08);
    // Verify residual matches solver tolerance
    assert!(
        f(yield_result).abs() < 1e-9,
        "f(yield) = {} exceeds tolerance",
        f(yield_result).abs()
    );
}

#[test]
fn newton_transcendental_equation() {
    // f(x) = e^x - 3x, has root near x ≈ 1.05
    let f = |x: f64| x.exp() - 3.0 * x;
    let df = |x: f64| x.exp() - 3.0;
    let solver = NewtonSolver::new().with_tolerance(1e-12);
    let r = solver.solve_with_derivative(f, df, 1.0).unwrap();

    // Verify residual
    assert!(
        f(r).abs() < 1e-11,
        "f(root) = {} exceeds tolerance",
        f(r).abs()
    );
}
