use finstack_core::math::solver::{BrentSolver, HybridSolver, Solver};

#[test]
fn brent_finds_root_simple_quadratic() {
    // f(x) = x^2 - 2 ⇒ root = sqrt(2)
    let f = |x: f64| x * x - 2.0;
    let solver = BrentSolver::new().with_tolerance(1e-12);
    let r = solver.solve(f, 1.5).unwrap(); // Initial guess between 1 and 2
    assert!((r - 2.0_f64.sqrt()).abs() < 1e-9);
}

#[test]
fn hybrid_solver_handles_derivative_small() {
    // f(x)=x^3 - x, roots at -1, 0, 1 ⇒ 1
    let f = |x: f64| x * x * x - x;
    let solver = HybridSolver::new().with_tolerance(1e-12);
    let r = solver.solve(f, 0.85).unwrap(); // Initial guess near root at 1
    assert!((r - 1.0).abs() < 1e-9);
}

#[test]
fn hybrid_solver_with_good_newton_guess() {
    // Simple case where Newton should work well
    let f = |x: f64| x * x - 4.0; // root at x = 2
    let solver = HybridSolver::new().with_tolerance(1e-12);

    let root = solver.solve(f, 1.8).unwrap();
    assert!((root - 2.0).abs() < 1e-9);
}

#[test]
fn hybrid_solver_with_bad_newton_guess() {
    // Case where Newton might struggle but hybrid should still work
    let f = |x: f64| x * x * x - x - 2.0; // Cubic with root near 1.5
    let solver = HybridSolver::new().with_tolerance(1e-12);

    // Bad initial guess that might cause Newton to diverge
    let root = solver.solve(f, 100.0).unwrap();

    // Verify it's actually a root
    assert!(f(root).abs() < 1e-9);
}

#[test]
fn hybrid_solver_bond_yield() {
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

    // Use HybridSolver with good initial guess
    let solver = HybridSolver::new().with_tolerance(1e-10);
    let yield_result = solver.solve(f, 0.06).unwrap();

    // Verify the yield makes sense (should be around 6-7% for this bond)
    assert!(yield_result > 0.05 && yield_result < 0.08);
    assert!(f(yield_result).abs() < 1e-6);
}

#[test]
fn hybrid_solver_fallback_to_brent() {
    // Pathological case where derivative is problematic
    let f = |x: f64| (x - 1.5).signum() * (x - 1.5).abs().powf(0.5); // sqrt function with sign
    let solver = HybridSolver::new().with_tolerance(1e-6);

    let root = solver.solve(f, 2.0).unwrap();
    assert!((root - 1.5).abs() < 1e-6);
}
