mod callable;
pub mod distributions;
pub mod integration;
pub mod solver;

pub use distributions::{
    binomial_probability_js as binomialProbability,
    log_binomial_coefficient_js as logBinomialCoefficient, log_factorial_js as logFactorial,
};
pub use integration::{
    adaptive_quadrature as adaptiveQuadrature, adaptive_simpson as adaptiveSimpson,
    gauss_legendre_integrate as gaussLegendreIntegrate,
    gauss_legendre_integrate_adaptive as gaussLegendreIntegrateAdaptive,
    gauss_legendre_integrate_composite as gaussLegendreIntegrateComposite,
    simpson_rule as simpsonRule, trapezoidal_rule as trapezoidalRule,
    JsGaussHermiteQuadrature as GaussHermiteQuadrature,
};
pub use solver::{
    JsBrentSolver as BrentSolver, JsHybridSolver as HybridSolver, JsNewtonSolver as NewtonSolver,
};
