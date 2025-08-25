use finstack_core::math::{brent, newton_bracketed};

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
