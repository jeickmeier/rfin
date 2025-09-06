use finstack_core::math::{brent, hybrid_root_find, newton_bracketed};

#[test]
fn brent_finds_root_simple_quadratic() {
    // f(x) = x^2 - 2 in [1, 2] ⇒ root = sqrt(2)
    let f = |x: f64| x * x - 2.0;
    let r = brent(f, 1.0, 2.0, 1e-12, 100).unwrap();
    assert!((r - 2.0_f64.sqrt()).abs() < 1e-9);
}

#[test]
fn newton_bracketed_handles_derivative_small() {
    // f(x)=x^3 - x, roots at -1, 0, 1; bracket [0.2, 1.5] ⇒ 1
    let f = |x: f64| x * x * x - x;
    let df = |x: f64| 3.0 * x * x - 1.0;
    let r = newton_bracketed(f, df, 0.2, 1.5, 1e-12, 100).unwrap();
    assert!((r - 1.0).abs() < 1e-9);
}

#[test]
fn hybrid_root_find_with_good_newton_guess() {
    // Simple case where Newton should work well
    let f = |x: f64| x * x - 4.0; // root at x = 2
    let df = |x: f64| 2.0 * x;

    let root = hybrid_root_find(f, df, 1.8, None, 1e-12, 100).unwrap();
    assert!((root - 2.0).abs() < 1e-9);
}

#[test]
fn hybrid_root_find_with_bad_newton_guess() {
    // Case where Newton might struggle but hybrid should still work
    let f = |x: f64| x * x * x - x - 2.0; // Cubic with root near 1.5
    let df = |x: f64| 3.0 * x * x - 1.0;

    // Bad initial guess that might cause Newton to diverge
    let root = hybrid_root_find(f, df, 100.0, None, 1e-12, 100).unwrap();

    // Verify it's actually a root
    assert!(f(root).abs() < 1e-9);
}

#[test]
fn hybrid_root_find_with_bracket() {
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

    // Derivative of bond price w.r.t. yield (approximate duration)
    let df = |y: f64| {
        let eps = 1e-8;
        (f(y + eps) - f(y - eps)) / (2.0 * eps)
    };

    // Good bracket for yields: [0, 20%]
    let yield_result = hybrid_root_find(f, df, 0.06, Some((0.001, 0.20)), 1e-10, 100).unwrap();

    // Verify the yield makes sense (should be around 6-7% for this bond)
    assert!(yield_result > 0.05 && yield_result < 0.08);
    assert!(f(yield_result).abs() < 1e-6);
}

#[test]
fn hybrid_root_find_fallback_to_brent() {
    // Pathological case where derivative is problematic
    let f = |x: f64| (x - 1.5).signum() * (x - 1.5).abs().powf(0.5); // sqrt function with sign
    let df = |x: f64| {
        if (x - 1.5).abs() < 1e-10 {
            f64::INFINITY // Problematic derivative at root
        } else {
            0.5 * (x - 1.5).signum() / (x - 1.5).abs().sqrt()
        }
    };

    let root = hybrid_root_find(f, df, 2.0, Some((1.0, 2.0)), 1e-6, 100).unwrap();
    assert!((root - 1.5).abs() < 1e-6);
}
