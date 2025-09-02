use finstack_core::math::{simpson_rule, adaptive_quadrature, trapezoidal_rule, GaussHermiteQuadrature};
use std::f64::consts::PI;

#[test]
fn test_simpson_rule_polynomial() {
    // Test Simpson's rule on x² over [0, 1] = 1/3
    let f = |x: f64| x * x;
    let result = simpson_rule(f, 0.0, 1.0, 100).unwrap();
    assert!((result - 1.0/3.0).abs() < 1e-6);
}

#[test]
fn test_simpson_rule_sin() {
    // Test Simpson's rule on sin(x) over [0, π] = 2
    let f = |x: f64| x.sin();
    let result = simpson_rule(f, 0.0, PI, 100).unwrap();
    assert!((result - 2.0).abs() < 1e-4);
}

#[test]
fn test_trapezoidal_rule_linear() {
    // Test trapezoidal rule on linear function x over [0, 2] = 2
    let f = |x: f64| x;
    let result = trapezoidal_rule(f, 0.0, 2.0, 100).unwrap();
    assert!((result - 2.0).abs() < 1e-6);
}

#[test]
fn test_trapezoidal_rule_quadratic() {
    // Test trapezoidal rule on x² over [0, 1] = 1/3
    let f = |x: f64| x * x;
    let result = trapezoidal_rule(f, 0.0, 1.0, 1000).unwrap();
    assert!((result - 1.0/3.0).abs() < 1e-3); // Less accurate than Simpson
}

#[test]
fn test_adaptive_quadrature_smooth_function() {
    // Test adaptive quadrature on e^(-x²) over [-2, 2]
    // This integral ≈ √π ≈ 1.7725
    let f = |x: f64| (-x * x).exp();
    let result = adaptive_quadrature(f, -2.0, 2.0, 1e-4, 20).unwrap();
    let expected = PI.sqrt();
    assert!((result - expected).abs() < 1e-2);
}

#[test]
fn test_adaptive_quadrature_oscillatory() {
    // Test on oscillatory function sin(10x) over [0, π]
    // Exact integral = (1 - cos(10π))/10 = 2/10 = 0.2
    let f = |x: f64| (10.0 * x).sin();
    let result = adaptive_quadrature(f, 0.0, PI, 1e-4, 25).unwrap();
    let expected = (1.0 - (10.0 * PI).cos()) / 10.0;
    assert!((result - expected).abs() < 1e-2);
}

#[test]
fn test_financial_application_option_payoff() {
    // Test integration of option payoff max(S - K, 0) under lognormal
    // This would be used in Monte Carlo option pricing
    let strike = 100.0_f64;
    let spot = 100.0_f64;
    let vol = 0.2_f64;
    let time = 1.0_f64;
    
    // Simplified Black-Scholes integrand (just the payoff part)
    let f = |z: f64| {
        // z is standard normal, transform to stock price
        let log_s = spot.ln() + (-0.5 * vol * vol * time) + vol * time.sqrt() * z;
        let s = log_s.exp();
        (s - strike).max(0.0)
    };
    
    // Use Gauss-Hermite for integration over normal distribution
    let quad = GaussHermiteQuadrature::order_10();
    let result = quad.integrate(f);
    
    // Result should be positive (call option value component)
    assert!(result > 0.0);
    // For ATM option with 20% vol, should be reasonable magnitude
    assert!(result > 5.0 && result < 25.0);
}

#[test]
fn test_integration_methods_comparison() {
    // Compare different methods on same function: x³ over [0, 2] = 4
    let f = |x: f64| x * x * x;
    let exact = 4.0;
    
    let simpson = simpson_rule(f, 0.0, 2.0, 100).unwrap();
    let trapezoidal = trapezoidal_rule(f, 0.0, 2.0, 100).unwrap();
    let adaptive = adaptive_quadrature(f, 0.0, 2.0, 1e-6, 20).unwrap();
    
    // Simpson should be most accurate for polynomials
    assert!((simpson - exact).abs() < 1e-8);
    // Trapezoidal less accurate
    assert!((trapezoidal - exact).abs() < 1e-2);
    // Adaptive should be very accurate
    assert!((adaptive - exact).abs() < 1e-10);
}

#[test]
fn test_integration_error_cases() {
    let f = |x: f64| x;
    
    // Simpson with odd number of intervals should fail
    assert!(simpson_rule(f, 0.0, 1.0, 99).is_err());
    
    // Zero intervals should fail
    assert!(simpson_rule(f, 0.0, 1.0, 0).is_err());
    assert!(trapezoidal_rule(f, 0.0, 1.0, 0).is_err());
}

#[test]
fn test_financial_yield_curve_integration() {
    // Test integration relevant to yield curve construction
    // Integrate forward rate over time to get zero rate
    let f = |t: f64| 0.03 + 0.01 * t; // Simple linear forward rate
    
    // Integrate from 0 to 5 years 
    let integrated_rate = simpson_rule(f, 0.0, 5.0, 1000).unwrap() / 5.0_f64;
    
    // Should be around 5.5% (average of 3% to 8%)
    assert!((integrated_rate - 0.055).abs() < 1e-3);
}
