use finstack_core::math::{correlation, covariance, mean, mean_var, variance};

#[test]
fn mean_and_variance_basic() {
    let xs = [1.0, 2.0, 3.0, 4.0];
    let m = mean(&xs);
    let v = variance(&xs);
    let (m2, v2) = mean_var(&xs);
    assert!((m - 2.5).abs() < 1e-12);
    // Population variance of 1..4 is 1.25
    assert!((v - 1.25).abs() < 1e-12);
    assert!((m - m2).abs() < 1e-12);
    assert!((v - v2).abs() < 1e-12);
}

#[test]
fn covariance_and_correlation() {
    let x = [1.0, 2.0, 3.0, 4.0];
    let y = [2.0, 4.0, 6.0, 8.0];
    let cov = covariance(&x, &y);
    let corr = correlation(&x, &y);
    // Perfect linear relationship ⇒ correlation 1
    assert!((corr - 1.0).abs() < 1e-12);
    // Covariance should be positive and consistent with scaling
    assert!(cov > 0.0);
}
